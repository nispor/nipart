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

use std::env::args;
use std::fs::OpenOptions;
use std::io::{Read, Write};

use serde_yaml;
use tokio::{self, io::AsyncWriteExt, net::UnixStream};
use nipart::{
    ipc_bind_with_path, ipc_recv, ipc_send, NipartConnection, NipartError,
    NipartIpcData, NipartIpcMessage, NipartPluginCapacity, NipartPluginInfo,
};

const PLUGIN_NAME: &str = "conman";
const CONF_FOLDER: &str = "/tmp/nipart";
const CONN_FILE_POSTFIX: &str = ".yml";

const CONNECTION_KEY: &str = "_connection";

#[tokio::main()]
async fn main() {
    let argv: Vec<String> = args().collect();

    if argv.len() != 2 {
        eprintln!(
            "Invalid argument, should be single argument: <plugin_socket_path>"
        );
        std::process::exit(1);
    }

    if let Err(e) = create_conf_dir() {
        eprintln!(
            "Failed to create folder for saving configurations {}: {}",
            CONF_FOLDER, e
        );
        std::process::exit(1);
    }

    let socket_path = &argv[1];

    let listener = match ipc_bind_with_path(socket_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    };
    eprintln!("DEBUG: {}: listening on {}", PLUGIN_NAME, socket_path);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                // TODO: Limit the maximum connected client as it could
                //       from suspicious source, not daemon
                tokio::task::spawn(async move { handle_client(stream).await });
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }
}

async fn shutdown_connection(stream: &mut UnixStream) {
    if let Err(e) = stream.shutdown().await {
        eprintln!("{}", e);
    }
}

// TODO: Implement on:
//  * timeout
async fn handle_client(mut stream: UnixStream) {
    loop {
        match ipc_recv(&mut stream).await {
            Ok(ipc_msg) => match ipc_msg.data {
                NipartIpcData::ConnectionClosed => {
                    shutdown_connection(&mut stream).await;
                    break;
                }
                _ => {
                    let message = handle_msg(ipc_msg.data).await;
                    eprintln!("DEBUG: {}: reply: {:?}", PLUGIN_NAME, &message);
                    if let Err(e) = ipc_send(&mut stream, &message).await {
                        eprintln!(
                            "DEBUG: {}: failed to send to daemon : {}",
                            PLUGIN_NAME, e
                        );
                    }
                }
            },
            Err(e) => {
                eprintln!("IPC error {}", e);
                shutdown_connection(&mut stream).await;
                break;
            }
        }
    }
}

async fn handle_msg(data: NipartIpcData) -> NipartIpcMessage {
    eprintln!("DEBUG: {}: Got request: {:?}", PLUGIN_NAME, data);
    match data {
        NipartIpcData::QueryPluginInfo => NipartIpcMessage::new(
            NipartIpcData::QueryPluginInfoReply(NipartPluginInfo::new(
                PLUGIN_NAME,
                vec![NipartPluginCapacity::Config],
            )),
        ),
        NipartIpcData::SaveConf(nip_con) => {
            NipartIpcMessage::from_result(save_conf(nip_con))
        }
        NipartIpcData::QuerySavedConf(uuid) => {
            NipartIpcMessage::from_result(query(&uuid))
        }
        NipartIpcData::QuerySavedConfAll => {
            NipartIpcMessage::from_result(query_all())
        }
        _ => NipartIpcMessage::new(NipartIpcData::None),
    }
}

fn save_conf(nip_con: NipartConnection) -> Result<NipartIpcMessage, NipartError> {
    let uuid = match &nip_con.uuid {
        Some(u) => u,
        None => {
            return Err(NipartError::bug(format!(
                "Got None uuid from daemon for connection {:?}",
                &nip_con,
            )))
        }
    };
    let file_path = gen_file_path(uuid);
    let mut fd =
        match OpenOptions::new().create(true).write(true).open(&file_path) {
            Ok(f) => f,
            Err(e) => {
                return Err(NipartError::plugin_error(format!(
                    "Failed to open file {}: {}",
                    &file_path, e
                )));
            }
        };

    let nip_con_yaml = nipart_connection_to_flat_string(&nip_con)?;

    if let Err(e) = fd.write_all(nip_con_yaml.as_bytes()) {
        Err(NipartError::plugin_error(format!(
            "Failed to write file {}: {}",
            &file_path, e
        )))
    } else {
        Ok(NipartIpcMessage::new(NipartIpcData::SaveConfReply(nip_con)))
    }
}

fn create_conf_dir() -> Result<(), NipartError> {
    if !std::path::Path::new(CONF_FOLDER).is_dir() {
        std::fs::remove_file(CONF_FOLDER).ok();
        if let Err(e) = std::fs::create_dir(CONF_FOLDER) {
            return Err(NipartError::plugin_error(format!(
                "Failed to create folder {}: {}",
                CONF_FOLDER, e
            )));
        }
    }
    Ok(())
}

fn query_all() -> Result<NipartIpcMessage, NipartError> {
    let conf_dir_path = std::path::Path::new(CONF_FOLDER);
    let mut nip_cons: Vec<NipartConnection> = Vec::new();
    match std::fs::read_dir(CONF_FOLDER) {
        Ok(dir) => {
            for entry in dir {
                let file_path = match entry {
                    Ok(f) => conf_dir_path.join(f.path()),
                    Err(e) => {
                        eprintln!("FAIL: Failed to read dir entry: {}", e);
                        continue;
                    }
                };
                let file_path = match file_path.to_str() {
                    Some(f) => f,
                    None => {
                        eprintln!(
                            "BUG: Should never happen: \
                        file_path.to_str() return None"
                        );
                        continue;
                    }
                };

                let conn_str = match read_file(file_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!(
                            "ERROR: Failed to read file {}: {}",
                            file_path, e
                        );
                        continue;
                    }
                };
                let nip_con: NipartConnection =
                    match nipart_connection_from_flat_string(&conn_str) {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!(
                                "ERROR: Invalid connection YAML file {}: {}",
                                file_path, e
                            );
                            continue;
                        }
                    };
                nip_cons.push(nip_con);
            }
            Ok(NipartIpcMessage::new(NipartIpcData::QuerySavedConfAllReply(
                nip_cons,
            )))
        }
        Err(e) => Err(NipartError::plugin_error(format!(
            "Failed to read dir {}: {}",
            CONF_FOLDER, e
        ))),
    }
}

fn read_file(file_path: &str) -> Result<String, NipartError> {
    let mut fd = match std::fs::File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            return Err(NipartError::plugin_error(format!(
                "Failed to open {}: {}",
                file_path, e
            )))
        }
    };
    let mut contents = String::new();
    if let Err(e) = fd.read_to_string(&mut contents) {
        Err(NipartError::plugin_error(format!(
            "Failed to open {}: {}",
            file_path, e
        )))
    } else {
        Ok(contents)
    }
}

fn nipart_connection_to_flat_string(
    nip_con: &NipartConnection,
) -> Result<String, NipartError> {
    let uuid = match &nip_con.uuid {
        Some(u) => u,
        None => {
            return Err(NipartError::bug(format!(
                "Got None uuid from daemon for connection {:?}",
                &nip_con,
            )))
        }
    };
    let name = match &nip_con.name {
        Some(u) => u,
        None => {
            return Err(NipartError::bug(format!(
                "Got None name from daemon for connection {:?}",
                &nip_con,
            )))
        }
    };
    let mut yaml_map: serde_yaml::Mapping =
        match serde_yaml::from_str(&nip_con.config) {
            Ok(o) => o,
            Err(e) => {
                return Err(NipartError::bug(format!(
                    "This should never happen, \
                    got invalid YAML file from daemon for SaveConf: {}: {}",
                    &nip_con.config, e
                )));
            }
        };
    yaml_map.insert(
        serde_yaml::Value::String(CONNECTION_KEY.to_string()),
        gen_connection_setting(uuid, name),
    );
    match serde_yaml::to_string(&yaml_map) {
        Ok(s) => Ok(s),
        Err(e) => Err(NipartError::bug(format!(
            "This should never happen, \
                failed to generate yaml string from Mapping: {:?}: {}",
            &yaml_map, e
        ))),
    }
}

fn nipart_connection_from_flat_string(
    conn_str: &str,
) -> Result<NipartConnection, NipartError> {
    let mut yaml_map: serde_yaml::Mapping = match serde_yaml::from_str(conn_str)
    {
        Ok(o) => o,
        Err(e) => {
            return Err(NipartError::invalid_argument(format!(
                "Corrupted connection YAML file from disk: {}: {}",
                conn_str, e
            )));
        }
    };
    let conn_setting = match yaml_map
        .remove(&serde_yaml::Value::String(CONNECTION_KEY.to_string()))
    {
        Some(c) => c,
        None => {
            return Err(NipartError::invalid_argument(format!(
                "connection YAML file does not have section for {}: {}",
                CONNECTION_KEY, conn_str
            )));
        }
    };
    let conn_setting = match conn_setting.as_mapping() {
        Some(m) => m,
        None => {
            return Err(NipartError::invalid_argument(format!(
                "connection YAML file section {} is not a map: {}",
                CONNECTION_KEY, conn_str
            )));
        }
    };
    let uuid_key = serde_yaml::Value::String("uuid".to_string());
    let name_key = serde_yaml::Value::String("name".to_string());
    if !conn_setting.contains_key(&uuid_key)
        || !conn_setting.contains_key(&name_key)
    {
        return Err(NipartError::invalid_argument(format!(
            "connection YAML file does not have name or uuid in section \
            {}: {}",
            CONNECTION_KEY, conn_str
        )));
    }
    let uuid = match conn_setting.get(&uuid_key) {
        Some(u) => match u.as_str() {
            Some(s) => s,
            None => {
                return Err(NipartError::invalid_argument(format!(
                    "connection YAML file does not have \
                    invalid uuid in section {}: {:?}",
                    CONNECTION_KEY, conn_setting
                )));
            }
        },
        None => {
            return Err(NipartError::invalid_argument(format!(
                "connection YAML file does not have \
                    uuid in section {}: {:?}",
                CONNECTION_KEY, conn_setting
            )));
        }
    };
    let name = match conn_setting.get(&name_key) {
        Some(u) => match u.as_str() {
            Some(s) => s,
            None => {
                return Err(NipartError::invalid_argument(format!(
                    "connection YAML file does not have \
                    invalid name in section {}: {:?}",
                    CONNECTION_KEY, conn_setting
                )));
            }
        },
        None => {
            return Err(NipartError::invalid_argument(format!(
                "connection YAML file does not have \
                    name in section {}: {:?}",
                CONNECTION_KEY, conn_setting
            )));
        }
    };
    let config_str = match serde_yaml::to_string(&yaml_map) {
        Ok(s) => s,
        Err(e) => {
            return Err(NipartError::bug(format!(
                "This should never happen, \
                serde_yaml::to_string(yaml_map) failed: {:?} {}",
                yaml_map, e,
            )));
        }
    };
    Ok(NipartConnection {
        name: Some(name.to_string()),
        uuid: Some(uuid.to_string()),
        config: config_str,
    })
}

fn gen_connection_setting(uuid: &str, name: &str) -> serde_yaml::Value {
    let mut setting = serde_yaml::Mapping::new();
    setting.insert(
        serde_yaml::Value::String("uuid".to_string()),
        serde_yaml::Value::String(uuid.to_string()),
    );
    setting.insert(
        serde_yaml::Value::String("name".to_string()),
        serde_yaml::Value::String(name.to_string()),
    );
    serde_yaml::Value::Mapping(setting)
}

fn query(uuid: &str) -> Result<NipartIpcMessage, NipartError> {
    let conn_str = read_file(&gen_file_path(uuid))?;
    Ok(NipartIpcMessage::new(NipartIpcData::QuerySavedConfReply(
        nipart_connection_from_flat_string(&conn_str)?,
    )))
}

fn gen_file_path(uuid: &str) -> String {
    format!("{}/{}{}", CONF_FOLDER, uuid, CONN_FILE_POSTFIX)
}
