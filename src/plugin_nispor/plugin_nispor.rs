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

mod iface;

use std::env::args;

use nispor::NetState;
use serde_yaml;
use tokio::{self, io::AsyncWriteExt, net::UnixStream};
use nipart::{
    ipc_bind_with_path, ipc_recv, ipc_send, NipartError, NipartIpcData,
    NipartIpcMessage, NipartPluginCapacity, NipartPluginInfo,
};

use crate::iface::NipartBaseIface;

const PLUGIN_NAME: &str = "nispor";

#[tokio::main()]
async fn main() {
    let argv: Vec<String> = args().collect();

    if argv.len() != 2 {
        eprintln!(
            "Invalid argument, should be single argument: <plugin_socket_path>"
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
                            "{}: failed to send to daemon : {}",
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

// TODO: The lib nipart should provide function call `plugin_start` taking
//       below function pointer as argument. But it is complex to passing
//       async function to a thread.
async fn handle_msg(data: NipartIpcData) -> NipartIpcMessage {
    eprintln!("DEBUG: {}: Got request: {:?}", PLUGIN_NAME, data);
    match data {
        NipartIpcData::QueryIfaceInfo(iface_name) => {
            NipartIpcMessage::from_result(query_iface(&iface_name))
        }
        NipartIpcData::QueryPluginInfo => NipartIpcMessage::new(
            NipartIpcData::QueryPluginInfoReply(NipartPluginInfo::new(
                PLUGIN_NAME,
                vec![NipartPluginCapacity::Query, NipartPluginCapacity::Apply],
            )),
        ),
        NipartIpcData::ValidateConf(conf) => {
            NipartIpcMessage::from_result(validate_conf(&conf))
        }
        _ => {
            eprintln!(
                "WARN: {}: Got unknown request: {:?}",
                PLUGIN_NAME, &data
            );
            NipartIpcMessage::new(NipartIpcData::None)
        }
    }
}

fn query_iface(iface_name: &str) -> Result<NipartIpcMessage, NipartError> {
    let net_state = match NetState::retrieve() {
        Ok(s) => s,
        Err(e) => {
            return Err(NipartError::plugin_error(format!(
                "nispor::NetState::retrieve() failed with {}",
                e
            )))
        }
    };
    match net_state.ifaces.get(iface_name) {
        Some(iface_info) => {
            let nipart_iface: NipartBaseIface = iface_info.into();
            match serde_yaml::to_string(&nipart_iface) {
                Ok(s) => Ok(NipartIpcMessage::new(
                    NipartIpcData::QueryIfaceInfoReply(s),
                )),
                Err(e) => Err(NipartError::plugin_error(format!(
                    "Failed to convert NipartIfaceInfo to yml: {}",
                    e
                ))),
            }
        }
        None => Err(NipartError::invalid_argument(format!(
            "Interface {} not found",
            iface_name
        ))),
    }
}

fn validate_conf(conf: &str) -> Result<NipartIpcMessage, NipartError> {
    if let Ok(nipart_iface) = serde_yaml::from_str::<NipartBaseIface>(conf) {
        if let Ok(s) = serde_yaml::to_string(&nipart_iface) {
            return Ok(NipartIpcMessage::new(NipartIpcData::ValidateConfReply(
                s,
            )));
        }
    }
    Ok(NipartIpcMessage::new(NipartIpcData::ValidateConfReply(
        "".into(),
    )))
}
