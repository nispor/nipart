// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use nipart::{
    NetworkState, NipartError, NipartIpcConnection, NipartPlugin,
    NipartPluginInfo, NipartstateApplyOption, NipartstateQueryOption,
};

#[derive(Debug)]
pub(crate) struct NipartPluginDemo;

impl NipartPlugin for NipartPluginDemo {
    const PLUGIN_NAME: &'static str = "demo";

    async fn init() -> Result<Self, NipartError> {
        Ok(Self {})
    }

    async fn plugin_info(
        _plugin: &Arc<Self>,
    ) -> Result<NipartPluginInfo, NipartError> {
        Ok(NipartPluginInfo::new(
            "demo".to_string(),
            "0.1.0".to_string(),
            vec![],
        ))
    }

    async fn query_network_state(
        _plugin: &Arc<Self>,
        opt: NipartstateQueryOption,
        conn: &mut NipartIpcConnection,
    ) -> Result<NetworkState, NipartError> {
        conn.log_trace(format!(
            "Demo plugin got query_network_state request with option {opt}"
        ))
        .await;
        Ok(NetworkState::default())
    }

    async fn apply_network_state(
        _plugin: &Arc<Self>,
        desired_state: NetworkState,
        opt: NipartstateApplyOption,
        conn: &mut NipartIpcConnection,
    ) -> Result<(), NipartError> {
        conn.log_trace(format!(
            "Demo plugin got apply_network_state request with state \
             {desired_state} and option {opt}"
        ))
        .await;
        Ok(())
    }
}
