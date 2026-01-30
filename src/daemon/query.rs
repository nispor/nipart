// SPDX-License-Identifier: Apache-2.0

use nipart::{
    ErrorKind, InterfaceType, NetworkState, NipartError, NipartIpcConnection,
    NipartNoDaemon, NipartstateInterface, NipartstateQueryOption,
    NipartstateStateKind,
};

use super::commander::NipartCommander;

impl NipartCommander {
    pub(crate) async fn query_network_state(
        &mut self,
        conn: Option<&mut NipartIpcConnection>,
        opt: NipartstateQueryOption,
    ) -> Result<NetworkState, NipartError> {
        if let Some(conn) = conn {
            conn.log_debug(format!("querying network state with option {opt}"))
                .await;
        } else {
            log::debug!("querying network state with option {opt}");
        }
        match opt.kind {
            NipartstateStateKind::RunningNetworkState => {
                let mut net_state =
                    NipartNoDaemon::query_network_state(opt.clone()).await?;

                let plugins_net_states = self
                    .plugin_manager
                    .query_network_state(opt.clone())
                    .await?;

                for plugins_net_state in plugins_net_states {
                    net_state.merge(&plugins_net_state)?;
                }

                // Use WIFI config stored in conf_manager
                let mut saved_state = self.conf_manager.query_state().await?;
                for (_, iface) in saved_state.ifaces.user_ifaces.drain() {
                    if iface.iface_type() == &InterfaceType::WifiCfg {
                        net_state.ifaces.push(iface);
                    }
                }

                self.dhcpv4_manager.fill_dhcp_states(&mut net_state).await?;

                if !opt.include_secrets {
                    net_state.hide_secrets();
                }

                // TODO: Mark interface/routes not int saved state as ignored.
                Ok(net_state)
            }
            NipartstateStateKind::SavedNetworkState => {
                let mut state = self.conf_manager.query_state().await?;
                if !opt.include_secrets {
                    state.hide_secrets();
                }
                Ok(state)
            }
            _ => Err(NipartError::new(
                ErrorKind::NoSupport,
                format!("Unsupported query option: {}", opt.kind),
            )),
        }
    }
}
