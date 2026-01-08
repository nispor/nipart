// SPDX-License-Identifier: Apache-2.0

use nipart::{
    NetworkState, NipartError, NipartPluginClient, NipartPluginInfo,
    NipartstateApplyOption, NipartstateInterface, NipartstateQueryOption,
};

#[derive(Debug, Clone)]
pub(crate) struct NipartDaemonPlugin {
    pub(crate) name: String,
    pub(crate) plugin_info: NipartPluginInfo,
    pub(crate) socket_path: String,
}

impl NipartDaemonPlugin {
    // TODO(Gris Ge):
    // * Timeout
    // * Ignore failure of plugins
    pub(crate) async fn query_network_state(
        &self,
        opt: &NipartstateQueryOption,
    ) -> Result<NetworkState, NipartError> {
        let mut cli = NipartPluginClient::new(&self.socket_path).await?;
        cli.query_network_state(opt.clone()).await
    }

    // TODO(Gris Ge):
    // * Timeout
    // * Ignore failure of plugins
    pub(crate) async fn apply_network_state(
        &self,
        apply_state: &NetworkState,
        opt: &NipartstateApplyOption,
    ) -> Result<(), NipartError> {
        let mut new_state = NetworkState::new();
        // Include only interfaces supported by plugin
        for iface in apply_state.ifaces.iter() {
            if self.plugin_info.iface_types.contains(iface.iface_type()) {
                new_state.ifaces.push(iface.clone());
            }
        }
        if new_state.is_empty() {
            log::trace!("No state require {} to apply", self.name);
            Ok(())
        } else {
            log::trace!(
                "Plugin {} apply_network_state {}",
                self.name,
                new_state
            );

            let mut cli = NipartPluginClient::new(&self.socket_path).await?;
            cli.apply_network_state(new_state, opt.clone()).await
        }
    }
}
