// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{
    JsonDisplayHideSecrets, NetworkState, NipartCanIpc, NipartError,
    NipartIpcConnection, NipartPluginInfo, NipartstateApplyOption,
    NipartstateQueryOption,
};

#[derive(Debug)]
pub struct NipartPluginClient {
    pub(crate) ipc: NipartIpcConnection,
}

/// Command send from daemon to plugin
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, JsonDisplayHideSecrets,
)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum NipartPluginCmd {
    /// Query plugin info, should reply with [NipartPluginInfo]
    QueryPluginInfo,
    /// Query network state, should reply with [NetworkState]
    QueryNetworkState(Box<NipartstateQueryOption>),
    ApplyNetworkState(Box<(NetworkState, NipartstateApplyOption)>),
    Quit,
}

impl NipartCanIpc for NipartPluginCmd {
    fn ipc_kind(&self) -> String {
        match self {
            Self::QueryPluginInfo => "query-plugin-info".to_string(),
            Self::QueryNetworkState(_) => "query-network-state".to_string(),
            Self::ApplyNetworkState(_) => "apply-network-state".to_string(),
            Self::Quit => "quit".to_string(),
        }
    }
}

impl NipartPluginCmd {
    pub fn hide_secrets(&mut self) {
        if let Self::ApplyNetworkState(cmd) = self {
            cmd.0.hide_secrets();
        }
    }
}

impl NipartPluginClient {
    pub const DEFAULT_SOCKET_DIR: &'static str =
        "/var/run/nipart/sockets/plugin";

    /// Create IPC connect from daemon to plugin
    pub async fn new(socket_path: &str) -> Result<Self, NipartError> {
        let dst_name = std::path::Path::new(socket_path)
            .file_name()
            .and_then(|p| p.to_str())
            .unwrap_or("plugin");
        Ok(Self {
            ipc: NipartIpcConnection::new_with_path(
                socket_path,
                "daemon",
                dst_name,
            )
            .await?,
        })
    }

    pub async fn query_plugin_info(
        &mut self,
    ) -> Result<NipartPluginInfo, NipartError> {
        self.ipc.send(Ok(NipartPluginCmd::QueryPluginInfo)).await?;
        self.ipc.recv::<NipartPluginInfo>().await
    }

    pub async fn query_network_state(
        &mut self,
        opt: NipartstateQueryOption,
    ) -> Result<NetworkState, NipartError> {
        self.ipc
            .send(Ok(NipartPluginCmd::QueryNetworkState(Box::new(opt))))
            .await?;
        self.ipc.recv::<NetworkState>().await
    }

    pub async fn apply_network_state(
        &mut self,
        desired_state: NetworkState,
        opt: NipartstateApplyOption,
    ) -> Result<(), NipartError> {
        self.ipc
            .send(Ok(NipartPluginCmd::ApplyNetworkState(Box::new((
                desired_state,
                opt,
            )))))
            .await?;
        self.ipc.recv::<()>().await
    }

    pub async fn send<T>(
        &mut self,
        data: Result<T, NipartError>,
    ) -> Result<(), NipartError>
    where
        T: NipartCanIpc,
    {
        self.ipc.send::<T>(data).await
    }

    pub async fn recv<T>(&mut self) -> Result<T, NipartError>
    where
        T: NipartCanIpc,
    {
        self.ipc.recv::<T>().await
    }
}
