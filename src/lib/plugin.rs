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

use async_trait::async_trait;
use futures::future::join_all;
use nmstate::NetworkState;
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncWriteExt, net::UnixStream};

use crate::{
    ipc_bind_with_path, ipc_connect_with_path, ipc_recv, ipc_send, ErrorKind,
    NipartApplyOption, NipartError, NipartIpcMessage, NipartQueryOption,
};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum NipartPluginIpcMessage {
    Error(NipartError),
    Done(Option<NetworkState>),
    Query(NipartQueryOption),
    ApplyKernel((NetworkState, NipartApplyOption)),
    ApplyDhcp(NetworkState),
    // ApplyOvs
    // ApplyDns
    SaveConf(NetworkState),
    ReadConf,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NipartPluginCapacity {
    QueryKernel, // For querying kernel network status
    ApplyKernel, // For applying kernel network config
    QueryDhcp,   // For querying DHCP status
    ApplyDhcp,   // For applying DHCP config
    Config,      // For managing network config files
}

#[async_trait]
pub trait NipartPlugin: Sized + std::fmt::Debug + std::marker::Send {
    fn name() -> &'static str;
    fn capacities() -> Vec<NipartPluginCapacity>;

    async fn query_kernel(
        _opt: &NipartQueryOption,
    ) -> Result<NetworkState, NipartError> {
        Err(NipartError::new(
            ErrorKind::NoSupport,
            format!(
                "query_kernel() not implemented by plugin {}",
                Self::name()
            ),
        ))
    }

    async fn handle_query_dhcp(_opt: &NipartQueryOption) -> NipartIpcMessage {
        NipartIpcMessage::Error(NipartError::new(
            ErrorKind::NoSupport,
            format!(
                "handle_query_dhcp() not implemented by plugin {}",
                Self::name()
            ),
        ))
    }

    async fn handle_apply_kernel(_opt: &NipartApplyOption) -> NipartIpcMessage {
        NipartIpcMessage::Error(NipartError::new(
            ErrorKind::NoSupport,
            format!(
                "handle_apply_kernel() not implemented by plugin {}",
                Self::name()
            ),
        ))
    }

    async fn handle_apply_dhcp(_opt: &NipartApplyOption) -> NipartIpcMessage {
        NipartIpcMessage::Error(NipartError::new(
            ErrorKind::NoSupport,
            format!(
                "handle_apply_dhcp() not implemented by plugin {}",
                Self::name()
            ),
        ))
    }

    async fn run() {
        let argv: Vec<String> = std::env::args().collect();
        if argv.len() != 2 {
            eprintln!(
                "Invalid argument, should be single argument: \
                <plugin_socket_path>"
            );
            return;
        }
        let mut log_builder = env_logger::Builder::new();
        log_builder.filter(None, log::LevelFilter::Warn);
        log_builder.init();

        let socket_path = &argv[1];

        let listener = match ipc_bind_with_path(socket_path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };
        log::debug!("{}: listening on {}", Self::name(), socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    // TODO: Limit the maximum connected client as it could
                    //       from suspicious source, not daemon
                    let socket_path_clone = socket_path.clone();
                    tokio::task::spawn(async move {
                        handle_plugin_client::<Self>(socket_path_clone, stream)
                            .await
                    });
                }
                Err(e) => {
                    log::error!("{}", e);
                }
            }
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct NipartPluginInfo {
    pub name: String,
    pub socket_path: String,
    pub capacities: Vec<NipartPluginCapacity>,
}

impl NipartPluginInfo {
    pub fn new(name: &str, capacities: Vec<NipartPluginCapacity>) -> Self {
        NipartPluginInfo {
            name: name.into(),
            socket_path: "".into(),
            capacities: capacities,
        }
    }
}

pub async fn ipc_plugin_exec(
    plugin_info: &NipartPluginInfo,
    ipc_msg: &NipartIpcMessage,
) -> Result<NipartIpcMessage, NipartError> {
    let mut stream = ipc_connect_with_path(&plugin_info.socket_path).await?;
    ipc_send(&mut stream, ipc_msg).await?;
    // TODO: Handle timeout
    ipc_recv(&mut stream).await
}

//TODO: save plugin name in return also, so we know which plugin to blame.
pub async fn ipc_plugins_exec(
    ipc_msg: &NipartIpcMessage,
    plugins: &[NipartPluginInfo],
    capacity: &NipartPluginCapacity,
) -> Vec<NipartIpcMessage> {
    let mut replys_async = Vec::new();
    for plugin_info in plugins {
        if plugin_info.capacities.contains(capacity) {
            replys_async.push(ipc_plugin_exec(plugin_info, &ipc_msg));
        }
    }
    let replys = join_all(replys_async).await;

    let mut reply_msgs = Vec::new();
    for reply in replys {
        match reply {
            Ok(r) => reply_msgs.push(r),
            Err(e) => {
                eprintln!("WARN: got error from plugin: {:?}", e);
            }
        }
    }
    reply_msgs
}

async fn handle_plugin_client<T>(socket_path: String, mut stream: UnixStream)
where
    T: NipartPlugin + Sized + std::marker::Send,
{
    let caps = T::capacities();
    loop {
        let plugin_msg = match ipc_recv(&mut stream).await {
            Ok(NipartIpcMessage::ConnectionClosed) => {
                stream.shutdown().await.ok();
                return;
            }
            Ok(NipartIpcMessage::QueryPluginInfo) => {
                ipc_send(
                    &mut stream,
                    &NipartIpcMessage::QueryPluginInfoReply(NipartPluginInfo {
                        name: T::name().to_string(),
                        socket_path: socket_path.clone(),
                        capacities: caps.clone(),
                    }),
                )
                .await
                .ok();
                continue;
            }
            Ok(NipartIpcMessage::Plugin(plug_msg)) => plug_msg,
            Ok(ipc_msg) => {
                let e = NipartIpcMessage::Error(NipartError::new(
                    ErrorKind::Bug,
                    format!("Expecting Plugin message, but got {:?}", ipc_msg),
                ));
                log::error!("{:?}", e);
                ipc_send(&mut stream, &e).await.ok();
                continue;
            }
            Err(e) => {
                log::error!("IPC error {}", e);
                stream.shutdown().await.ok();
                return;
            }
        };

        let reply_msg = match plugin_msg {
            NipartPluginIpcMessage::Query(opts) => {
                if caps.contains(&NipartPluginCapacity::QueryKernel) {
                    match T::query_kernel(&opts).await {
                        Ok(s) => NipartIpcMessage::QueryStateReply(s),
                        Err(e) => NipartIpcMessage::Error(e),
                    }
                } else if caps.contains(&NipartPluginCapacity::QueryDhcp) {
                    T::handle_query_dhcp(&opts).await
                } else {
                    NipartIpcMessage::Error(NipartError::new(
                        ErrorKind::NoSupport,
                        "Do not support NipartPluginIpcMessage::Query"
                            .to_string(),
                    ))
                }
            }
            _ => {
                let e = NipartIpcMessage::Error(NipartError::new(
                    ErrorKind::Bug,
                    format!("Not implemented handler for {:?}", plugin_msg),
                ));
                log::error!("{:?}", e);
                e
            }
        };
        ipc_send(&mut stream, &reply_msg).await.ok();
    }
}
