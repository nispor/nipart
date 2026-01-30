// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    env::current_exe,
    os::unix::fs::{FileTypeExt, PermissionsExt},
};

use futures_channel::{mpsc::UnboundedReceiver, oneshot::Sender};
use futures_util::{StreamExt, stream::FuturesUnordered};
use nipart::{
    NetworkState, NipartError, NipartPluginClient, NipartstateApplyOption,
    NipartstateQueryOption,
};

use super::plugin_exec::NipartDaemonPlugin;
use crate::TaskWorker;

const NM_PLUGIN_PREFIX: &str = "nipart-plugin-";
const NM_PLUGIN_CONN_RETRY: i8 = 50;
const NM_PLUGIN_CONN_RETRY_INTERVAL_MS: u64 = 200;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NipartPluginCmd {
    QueryNetworkState(Box<NipartstateQueryOption>),
    ApplyNetworkState(Box<(NetworkState, NipartstateApplyOption)>),
}

impl std::fmt::Display for NipartPluginCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueryNetworkState(_) => {
                write!(f, "query-network-state")
            }
            Self::ApplyNetworkState(_) => {
                write!(f, "apply-network-state")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NipartPluginReply {
    None,
    States(Vec<NetworkState>),
}

type FromManager = (
    NipartPluginCmd,
    Sender<Result<NipartPluginReply, NipartError>>,
);

#[derive(Debug)]
pub(crate) struct NipartPluginWorker {
    receiver: UnboundedReceiver<FromManager>,
    plugins: HashMap<String, NipartDaemonPlugin>,
}

impl TaskWorker for NipartPluginWorker {
    type Cmd = NipartPluginCmd;
    type Reply = NipartPluginReply;

    async fn new(
        receiver: UnboundedReceiver<FromManager>,
    ) -> Result<Self, NipartError> {
        let plugin_paths = get_plugin_files();

        let mut expected_plugin_count = 0;
        for plugin_path in plugin_paths {
            if std::path::Path::new(&plugin_path)
                .file_name()
                .and_then(|p| p.to_str())
                == Some("nipart-plugin-demo")
            {
                log::debug!("Ignored demo plugin");
                continue;
            }
            log::debug!("Starting nipart plugin {}", plugin_path);
            if let Err(e) = std::process::Command::new(&plugin_path).spawn() {
                log::info!("Ignoring plugin {plugin_path} due to error: {e}");
            }
            expected_plugin_count += 1;
        }

        let mut plugins: HashMap<String, NipartDaemonPlugin> = HashMap::new();
        let mut retry_left = NM_PLUGIN_CONN_RETRY;

        while plugins.len() < expected_plugin_count && retry_left >= 0 {
            retry_left -= 1;
            connect_plugins(&mut plugins).await;
            tokio::time::sleep(std::time::Duration::from_millis(
                NM_PLUGIN_CONN_RETRY_INTERVAL_MS,
            ))
            .await;
        }

        Ok(Self { receiver, plugins })
    }

    fn receiver(&mut self) -> &mut UnboundedReceiver<FromManager> {
        &mut self.receiver
    }

    async fn process_cmd(
        &mut self,
        cmd: NipartPluginCmd,
    ) -> Result<NipartPluginReply, NipartError> {
        log::debug!("Processing plugin command: {cmd}");
        match cmd {
            NipartPluginCmd::QueryNetworkState(opt) => {
                let mut ret = Vec::new();
                // TODO(Gris Ge): Should querying all plugin at the same time
                // instead of one by one.
                for plugin in self.plugins.values() {
                    match plugin.query_network_state(&opt).await {
                        Ok(net_state) => ret.push(net_state),
                        Err(e) => {
                            log::info!("{e}");
                        }
                    }
                }

                Ok(NipartPluginReply::States(ret))
            }
            NipartPluginCmd::ApplyNetworkState(v) => {
                let (apply_state, opt) = *v;
                // TODO(Gris Ge): Should request all plugin at the same time
                // instead of one by one.
                let mut result_futures = FuturesUnordered::new();
                for plugin in self.plugins.values() {
                    let result_future =
                        plugin.apply_network_state(&apply_state, &opt);
                    result_futures.push(result_future);
                }

                while let Some(result) = result_futures.next().await {
                    // It is OK for plugin to fail, verification process will
                    // noticed the difference
                    if let Err(e) = result {
                        log::warn!("{e}");
                    }
                }
                Ok(NipartPluginReply::None)
            }
        }
    }
}

fn get_plugin_files() -> Vec<String> {
    let mut plugins: Vec<String> = Vec::new();

    let search_dir = if let Some(p) = current_exe().ok().and_then(|p| {
        p.parent().and_then(|s| s.to_str()).map(|s| s.to_string())
    }) {
        p
    } else {
        return plugins;
    };

    for file_path in get_file_paths_in_dir(&search_dir) {
        let path = std::path::Path::new(&file_path);
        if is_executable(path)
            && path
                .strip_prefix(&search_dir)
                .ok()
                .and_then(|p| p.to_str())
                .map(|p| p.starts_with(NM_PLUGIN_PREFIX))
                .unwrap_or_default()
        {
            plugins.push(file_path);
        }
    }

    plugins
}

fn is_executable(path: &std::path::Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| (meta.permissions().mode() & 0o100) > 0)
        .unwrap_or_default()
}

fn is_socket(path: &std::path::Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.file_type().is_socket())
        .unwrap_or_default()
}

fn get_file_paths_in_dir(dir: &str) -> Vec<String> {
    let mut ret: Vec<String> = Vec::new();
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        log::debug!("Failed to read dir {dir}: {e}");
                        continue;
                    }
                };
                if !entry.path().is_dir()
                    && let Some(p) = entry.path().to_str()
                {
                    ret.push(p.to_string());
                }
            }
        }
        Err(e) => {
            log::debug!("Failed to read dir {dir}: {e}");
        }
    }
    ret
}

async fn connect_plugins(plugins: &mut HashMap<String, NipartDaemonPlugin>) {
    for file_path in
        get_file_paths_in_dir(NipartPluginClient::DEFAULT_SOCKET_DIR)
    {
        let path = std::path::Path::new(&file_path);
        if is_socket(path)
            && let Ok(mut client) = NipartPluginClient::new(&file_path).await {
                match client.query_plugin_info().await {
                    Ok(info) => {
                        log::info!(
                            "Plugin {} version {} connected",
                            info.name,
                            info.version,
                        );
                        plugins.insert(
                            info.name.to_string(),
                            NipartDaemonPlugin {
                                name: info.name.to_string(),
                                plugin_info: info,
                                socket_path: file_path,
                            },
                        );
                    }
                    Err(e) => {
                        log::debug!("{e}");
                    }
                }
            }
    }
}
