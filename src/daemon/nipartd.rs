//    Copyright 2021 Red Hat, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod plugin;

use std::collections::HashMap;

use serde_yaml;
use tokio::{self, io::AsyncWriteExt, net::UnixStream, task};
use uuid::Uuid;
use nipart::{
    ipc_bind, ipc_plugins_exec, ipc_recv_safe, ipc_send, merge_yaml_mappings,
    NipartConnection, NipartError, NipartIpcData, NipartIpcMessage,
    NipartPluginCapacity, NipartPluginInfo,
};

use crate::plugin::load_plugins;

#[tokio::main(flavor = "multi_thread", worker_threads = 50)]
async fn main() {
    let listener = match ipc_bind() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // We don't plan to unload plugin during runtime when plugin is slow or bad.
    // To support that, we need a mutex protected Vec which is complex.
    // We assume the plugin is trustable.
    let plugins = load_plugins().await;

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                eprintln!("DEBUG: daemon: IPC client connected");
                // TODO: Limit the maximum connected client.
                let plugins_clone = plugins.clone();
                task::spawn(async move {
                    handle_client(stream, &plugins_clone).await
                });
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }
}

async fn shutdown_connection(stream: &mut UnixStream) {
    if let Err(e) = stream.shutdown().await {
        eprintln!("ERROR: Daemon: failed to shutdown a connection: {}", e);
    }
}

// TODO: Implement on:
//  * timeout
async fn handle_client(mut stream: UnixStream, plugins: &[NipartPluginInfo]) {
    loop {
        match ipc_recv_safe(&mut stream).await {
            Ok(ipc_msg) => {
                let reply_ipc_msg =
                    NipartIpcMessage::from_result(match ipc_msg.data {
                        NipartIpcData::ConnectionClosed => {
                            shutdown_connection(&mut stream).await;
                            break;
                        }
                        NipartIpcData::QueryIfaceInfo(filter) => {
                            handle_query(&filter, plugins).await
                        }
                        NipartIpcData::SaveConf(connection) => {
                            handle_save_conf(&connection, plugins).await
                        }
                        NipartIpcData::QuerySavedConf(uuid) => {
                            handle_query_saved_conf(&uuid, plugins).await
                        }
                        NipartIpcData::QuerySavedConfAll => {
                            handle_query_saved_conf_all(plugins).await
                        }
                        _ => {
                            eprintln!(
                                "ERROR: got unknown IPC message: {:?}",
                                &ipc_msg
                            );
                            Ok(NipartIpcMessage::new(NipartIpcData::Error(
                                NipartError::invalid_argument(format!(
                                    "Invalid IPC message: {:?}",
                                    &ipc_msg
                                )),
                            )))
                        }
                    });
                if let Err(e) = ipc_send(&mut stream, &reply_ipc_msg).await {
                    eprintln!("ERROR: Failed to reply via IPC {}", e);
                }
            }
            Err(e) => {
                eprintln!("IPC error {}", e);
                shutdown_connection(&mut stream).await;
                break;
            }
        }
    }
}

async fn handle_query(
    filter: &str,
    plugins: &[NipartPluginInfo],
) -> Result<NipartIpcMessage, NipartError> {
    eprintln!("DEBUG: handle_query {}", filter);
    let ipc_msg =
        NipartIpcMessage::new(NipartIpcData::QueryIfaceInfo(filter.into()));

    let reply_ipc_msg =
        ipc_plugins_exec(&ipc_msg, plugins, &NipartPluginCapacity::Query).await;
    let reply_strs = extract_strs_from_ipc_msg(&reply_ipc_msg);

    Ok(NipartIpcMessage::new(NipartIpcData::QueryIfaceInfoReply(
        merge_yaml_mappings(&reply_strs)?,
    )))
}

// Steps:
//  0. Determin the connection UUID and name if user not defined.
//  1. Send conf string to plugin to validate. Raise error if existing plugins
//     cannot achieve full desire config.
//  2. Send conf string to plugin to save.
//
async fn handle_save_conf(
    connection: &NipartConnection,
    plugins: &[NipartPluginInfo],
) -> Result<NipartIpcMessage, NipartError> {
    eprintln!("DEBUG: handle_save_conf {:?}", connection);

    validate_conf(&connection.config, plugins).await?;

    let mut nip_con = connection.clone();

    // Gen UUID if not defined
    if nip_con.uuid == None {
        nip_con.uuid = Some(format!(
            "{}",
            Uuid::new_v4()
                .to_hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
        ));
    }

    if nip_con.name == None {
        nip_con.name = Some(gen_connection_name(&nip_con.config));
    }

    let ipc_msg = NipartIpcMessage::new(NipartIpcData::SaveConf(nip_con.clone()));

    let reply_ipc_msgs =
        ipc_plugins_exec(&ipc_msg, plugins, &NipartPluginCapacity::Config).await;

    let mut reply_nip_cons = Vec::new();
    for reply_ipc_msg in reply_ipc_msgs {
        if let NipartIpcData::SaveConfReply(nip_con) = reply_ipc_msg.data {
            reply_nip_cons.push(nip_con);
        }
    }
    if reply_nip_cons.len() == 0 {
        Err(NipartError::plugin_error(format!(
            "No plugin has saved desired config"
        )))
    } else {
        nip_con.merge_from(&reply_nip_cons)?;
        Ok(NipartIpcMessage::new(NipartIpcData::SaveConfReply(nip_con)))
    }
}

// Each plugin could only cover a portion of the configure, but they should
// sum up to the full desire config, or else return NipartError
async fn validate_conf(
    conf: &str,
    plugins: &[NipartPluginInfo],
) -> Result<(), NipartError> {
    eprintln!("DEBUG: validate_conf {}", conf);
    let ipc_msg =
        NipartIpcMessage::new(NipartIpcData::ValidateConf(conf.to_string()));

    let desire_yaml_mapping: serde_yaml::Value =
        match serde_yaml::from_str(conf) {
            Ok(i) => i,
            Err(e) => {
                return Err(NipartError::invalid_argument(format!(
                    "Invalid format of YAML: {}",
                    e
                )));
            }
        };

    let reply_ipc_msgs =
        ipc_plugins_exec(&ipc_msg, plugins, &NipartPluginCapacity::Apply).await;
    let reply_strs = extract_strs_from_ipc_msg(&reply_ipc_msgs);
    let merged_reply = merge_yaml_mappings(reply_strs.as_slice())?;
    let validated_yaml_mapping: serde_yaml::Value =
        match serde_yaml::from_str(&merged_reply) {
            Ok(i) => i,
            Err(e) => {
                return Err(NipartError::bug(format!(
                    "This should never happen: {}",
                    e
                )));
            }
        };

    if validated_yaml_mapping != desire_yaml_mapping {
        // TODO: provide fancy difference to user via error.
        Err(NipartError::invalid_argument(format!(
            "Invalid config, validated: {}, desired: {}",
            &merged_reply, conf
        )))
    } else {
        Ok(())
    }
}

async fn handle_query_saved_conf_all(
    plugins: &[NipartPluginInfo],
) -> Result<NipartIpcMessage, NipartError> {
    eprintln!("DEBUG: handle_query_saved_conf_all");

    let ipc_msg = NipartIpcMessage::new(NipartIpcData::QuerySavedConfAll);

    let reply_ipc_msgs =
        ipc_plugins_exec(&ipc_msg, plugins, &NipartPluginCapacity::Config).await;
    let mut all_nip_cons = HashMap::new();
    for reply_ipc_msg in reply_ipc_msgs {
        if let NipartIpcData::QuerySavedConfAllReply(nip_cons) =
            reply_ipc_msg.data
        {
            for nip_con in nip_cons {
                let uuid = match &nip_con.uuid {
                    Some(u) => u.to_string(),
                    None => {
                        eprintln!(
                            "ERROR: plugin reply with None UUID: {:?}",
                            nip_con
                        );
                        continue;
                    }
                };
                if !all_nip_cons.contains_key(&uuid) {
                    all_nip_cons.insert(uuid, nip_con);
                }
            }
        } else {
            eprintln!(
                "ERROR: Invalid plugin reply for QuerySavedConfAll: {:?}",
                reply_ipc_msg
            );
        }
    }
    Ok(NipartIpcMessage::new(NipartIpcData::QuerySavedConfAllReply(
        all_nip_cons.iter().map(|(_, v)| v.clone()).collect(),
    )))
}

fn gen_connection_name(config: &str) -> String {
    if let Ok(yml_value) = serde_yaml::from_str::<serde_yaml::Value>(config) {
        if let Some(m) = yml_value.as_mapping() {
            if let Some(serde_yaml::Value::String(name)) =
                m.get(&serde_yaml::Value::String("name".to_string()))
            {
                return name.clone();
            }
        }
    }
    "unknown".to_string()
}

fn extract_strs_from_ipc_msg<'a>(
    ipc_msgs: &'a [NipartIpcMessage],
) -> Vec<&'a str> {
    let mut data_strs = Vec::new();
    for ipc_msg in ipc_msgs {
        if let Ok(s) = ipc_msg.get_data_str() {
            data_strs.push(s)
        }
    }
    data_strs
}

async fn handle_query_saved_conf(
    uuid: &str,
    plugins: &[NipartPluginInfo],
) -> Result<NipartIpcMessage, NipartError> {
    eprintln!("DEBUG: handle_query_saved_conf: {}", uuid);

    let ipc_msg =
        NipartIpcMessage::new(NipartIpcData::QuerySavedConf(uuid.to_string()));

    let reply_ipc_msgs =
        ipc_plugins_exec(&ipc_msg, plugins, &NipartPluginCapacity::Config).await;

    let mut reply_nip_cons = Vec::new();
    for reply_ip_msg in reply_ipc_msgs {
        if let NipartIpcData::QuerySavedConfReply(nip_con) = reply_ip_msg.data {
            reply_nip_cons.push(nip_con)
        }
    }
    if reply_nip_cons.len() == 0 {
        Err(NipartError::invalid_argument(format!(
            "Connection {} not found",
            uuid
        )))
    } else {
        let mut nip_con = reply_nip_cons[0].clone();
        nip_con.merge_from(&reply_nip_cons)?;
        Ok(NipartIpcMessage::new(NipartIpcData::QuerySavedConfReply(
            nip_con,
        )))
    }
}
