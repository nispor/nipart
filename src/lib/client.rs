// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{
    JsonDisplayHideSecrets, NetworkState, NipartCanIpc, NipartError, NipartIpcConnection,
    NipartstateApplyOption, NipartstateQueryOption,
};

impl NipartCanIpc for NetworkState {
    fn ipc_kind(&self) -> String {
        "network_state".to_string()
    }
}

#[derive(Debug)]
pub struct NipartClient {
    pub(crate) ipc: NipartIpcConnection,
}

#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, JsonDisplayHideSecrets,
)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum NipartClientCmd {
    Ping,
    QueryNetworkState(Box<NipartstateQueryOption>),
    ApplyNetworkState(Box<(NetworkState, NipartstateApplyOption)>),
}

impl NipartCanIpc for NipartClientCmd {
    fn ipc_kind(&self) -> String {
        match self {
            Self::Ping => "ping".to_string(),
            Self::QueryNetworkState(_) => "query-network-state".to_string(),
            Self::ApplyNetworkState(_) => "apply-network-state".to_string(),
        }
    }
}

impl NipartClientCmd {
    pub fn hide_secrets(&mut self) {
        if let NipartClientCmd::ApplyNetworkState(cmd) = self {
            cmd.0.hide_secrets();
        }
    }
}

impl NipartClient {
    pub const DEFAULT_SOCKET_PATH: &'static str =
        "/var/run/nipart/sockets/daemon";

    /// Create IPC connect to nipart daemon
    pub async fn new() -> Result<Self, NipartError> {
        Self::new_with_name("client").await
    }

    pub async fn new_with_name(name: &str) -> Result<Self, NipartError> {
        Ok(Self {
            ipc: NipartIpcConnection::new_with_path(
                Self::DEFAULT_SOCKET_PATH,
                name,
                "daemon",
            )
            .await?,
        })
    }

    pub async fn ping(&mut self) -> Result<String, NipartError> {
        self.ipc.send(Ok(NipartClientCmd::Ping)).await?;
        self.ipc.recv::<String>().await
    }

    pub async fn query_network_state(
        &mut self,
        option: NipartstateQueryOption,
    ) -> Result<NetworkState, NipartError> {
        self.ipc
            .send(Ok(NipartClientCmd::QueryNetworkState(Box::new(option))))
            .await?;
        self.ipc.recv::<NetworkState>().await
    }

    pub async fn apply_network_state(
        &mut self,
        desired_state: NetworkState,
        option: NipartstateApplyOption,
    ) -> Result<NetworkState, NipartError> {
        self.ipc
            .send(Ok(NipartClientCmd::ApplyNetworkState(Box::new((
                desired_state,
                option,
            )))))
            .await?;
        self.ipc.recv::<NetworkState>().await
    }
}
