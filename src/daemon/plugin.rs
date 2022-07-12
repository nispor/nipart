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

use std::os::unix::fs::PermissionsExt;

use nipart::{
    ipc_connect_with_path, ipc_exec, ErrorKind, NipartError, NipartIpcMessage,
    NipartPluginInfo,
};
use tokio::net::UnixStream;

const PLUGIN_PREFIX: &str = "nipart_plugin_";
const PLUGIN_SOCKET_PREFIX: &str = "/tmp/nipart_plugin_";

const PLUGIN_CONNECT_REPLY_COUNT: usize = 10;
const PLUGIN_CONNECT_REPLY_INTERVAL: u64 = 100; // 100ms

// Each plugin will be invoked in a thread with a socket path string as its
// first argument. The plugin should listen on that socket and wait command
// from plugin.
//
pub(crate) async fn load_plugins() -> Vec<NipartPluginInfo> {
    log::debug!("Loading plugins");
    let mut plugins = Vec::new();
    let search_folder = match std::env::var("NIPART_PLUGIN_FOLDER") {
        Ok(d) => d,
        Err(_) => get_current_exec_folder(),
    };
    log::debug!("Searching plugin at {}", search_folder);
    match std::fs::read_dir(&search_folder) {
        Ok(dir) => {
            for entry in dir {
                let file_name = match entry {
                    Ok(f) => f.file_name(),
                    Err(e) => {
                        log::error!("Failed to read dir entry: {}", e);
                        continue;
                    }
                };
                let file_name = match file_name.to_str() {
                    Some(n) => n,
                    None => {
                        log::error!("Failed to read file_name",);
                        continue;
                    }
                };
                if file_name.starts_with(PLUGIN_PREFIX) {
                    let plugin_exec_path =
                        format!("{}/{}", &search_folder, file_name);
                    if !is_executable(&plugin_exec_path) {
                        continue;
                    }
                    let plugin_name =
                        match file_name.strip_prefix(PLUGIN_PREFIX) {
                            Some(n) => n,
                            None => {
                                log::error!(
                                    "file_name {} not started with {}",
                                    file_name,
                                    PLUGIN_PREFIX,
                                );
                                continue;
                            }
                        };
                    log::debug!("Found plugin {}", &plugin_exec_path);
                    match plugin_start(&plugin_exec_path, &plugin_name).await {
                        Ok(plugin) => {
                            log::debug!(
                                "Plugin {} started at {} with \
                                capacities: {:?}",
                                &plugin.name,
                                &plugin.socket_path,
                                &plugin.capacities
                            );
                            plugins.push(plugin);
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to start plugin {}: {}",
                                &plugin_exec_path,
                                e
                            );
                            continue;
                        }
                    }
                }
            }
        }
        Err(e) => {
            log::error!("Failed to open plugin search dir /usr/bin: {}", e);
        }
    };
    plugins
}

async fn plugin_start(
    plugin_exec_path: &str,
    plugin_name: &str,
) -> Result<NipartPluginInfo, NipartError> {
    let socket_path = format!("{}{}", PLUGIN_SOCKET_PREFIX, plugin_name);
    // Invoke the plugin in child.
    match std::process::Command::new(plugin_exec_path)
        .arg(&socket_path)
        .spawn()
    {
        Ok(_) => {
            log::debug!(
                "Plugin {} started at {}",
                plugin_exec_path,
                &socket_path
            );

            query_plugin_info(&socket_path).await
        }
        Err(e) => Err(NipartError::new(
            ErrorKind::PluginError,
            format!(
                "Failed to start plugin {} {}: {}",
                plugin_exec_path, &socket_path, e
            ),
        )),
    }
}

fn is_executable(file_path: &str) -> bool {
    if let Ok(attr) = std::fs::metadata(file_path) {
        attr.permissions().mode() & 0o100 != 0
    } else {
        false
    }
}

fn get_current_exec_folder() -> String {
    if let Ok(mut exec_path) = std::env::current_exe() {
        exec_path.pop();
        if let Some(dir_path) = exec_path.to_str() {
            return dir_path.into();
        }
    }

    "/usr/bin".into()
}

async fn query_plugin_info(
    socket_path: &str,
) -> Result<NipartPluginInfo, NipartError> {
    // Plugin might not ready yet right after started, so retry is required.
    let mut stream = ipc_connect_with_retry(socket_path).await?;
    let ipc_msg =
        ipc_exec(&mut stream, &NipartIpcMessage::QueryPluginInfo).await?;

    if let NipartIpcMessage::QueryPluginInfoReply(mut plugin_info) = ipc_msg {
        plugin_info.socket_path = socket_path.into();
        Ok(plugin_info)
    } else {
        Err(NipartError::new(
            ErrorKind::PluginError,
            format!("Invalid plugin reply on QueryPluginInfo: {:?}", ipc_msg),
        ))
    }
}

async fn ipc_connect_with_retry(
    socket_path: &str,
) -> Result<UnixStream, NipartError> {
    for i in 0..PLUGIN_CONNECT_REPLY_COUNT {
        std::thread::sleep(std::time::Duration::from_millis(
            PLUGIN_CONNECT_REPLY_INTERVAL,
        ));
        match ipc_connect_with_path(socket_path).await {
            Err(e) => {
                if i == PLUGIN_CONNECT_REPLY_COUNT - 1 {
                    return Err(NipartError::new(
                        ErrorKind::PluginError,
                        format!(
                            "Failed to connect plugin IPC socket {}: {}",
                            socket_path, e
                        ),
                    ));
                } else {
                    log::debug!(
                        "DEBUG: Failed to connect plugin \
                        socket_path: {}: {}, retrying",
                        socket_path,
                        e
                    );
                    continue;
                }
            }
            Ok(s) => return Ok(s),
        }
    }
    Err(NipartError::new(
        ErrorKind::Bug,
        "This should never happen in ipc_connect_with_retry".into(),
    ))
}
