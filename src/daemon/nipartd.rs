//    Copyright 2021-2022 Red Hat, Inc.
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

use nipart::{
    ipc_bind, ipc_plugins_exec, ipc_recv_safe, ipc_send, ErrorKind,
    NipartApplyOption, NipartError, NipartIpcMessage, NipartPluginCapacity,
    NipartPluginInfo, NipartPluginIpcMessage, NipartQueryOption,
};
use nmstate::NetworkState;
use tokio::{self, io::AsyncWriteExt, net::UnixStream, task};

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
                    NipartIpcMessage::from_result(match ipc_msg {
                        NipartIpcMessage::ConnectionClosed => {
                            shutdown_connection(&mut stream).await;
                            break;
                        }
                        NipartIpcMessage::QueryState(opt) => {
                            handle_query(plugins, &opt).await
                        }
                        NipartIpcMessage::ApplyState(state, opt) => {
                            handle_apply(&state, plugins, &opt).await
                        }
                        _ => {
                            eprintln!(
                                "ERROR: got unknown IPC message: {:?}",
                                &ipc_msg
                            );
                            Ok(NipartIpcMessage::Error(NipartError::new(
                                ErrorKind::InvalidArgument,
                                format!("Invalid IPC message: {:?}", &ipc_msg),
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
    plugins: &[NipartPluginInfo],
    opt: &NipartQueryOption,
) -> Result<NipartIpcMessage, NipartError> {
    log::debug!("handle_query: {:?}", plugins);
    let ipc_msg =
        NipartIpcMessage::Plugin(NipartPluginIpcMessage::Query(opt.clone()));

    let reply_ipc_msgs =
        ipc_plugins_exec(&ipc_msg, plugins, &NipartPluginCapacity::QueryKernel)
            .await;

    // TODO: merge NetworkState from plugins
    Ok(reply_ipc_msgs[0].clone())
}

async fn handle_apply(
    state: &NetworkState,
    plugins: &[NipartPluginInfo],
    opt: &NipartApplyOption,
) -> Result<NipartIpcMessage, NipartError> {
    todo!()
}
