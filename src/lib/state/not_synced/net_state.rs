// SPDX-License-Identifier: Apache-2.0

use crate::{NetworkCommit, NetworkState, NipartDhcpConfig};

impl NetworkState {
    pub fn fill_dhcp_config(&mut self, dhcp_configs: &[NipartDhcpConfig]) {
        for dhcp_config in dhcp_configs {
            if let NipartDhcpConfig::V4(dhcp_config) = dhcp_config {
                if dhcp_config.enabled {
                    if let Some(iface) = self
                        .interfaces
                        .kernel_ifaces
                        .get_mut(dhcp_config.iface.as_str())
                    {
                        let ipv4_conf = iface
                            .base_iface_mut()
                            .ipv4
                            .get_or_insert(Default::default());
                        ipv4_conf.enabled = true;
                        ipv4_conf.dhcp = Some(true);
                    }
                }
            }
        }
    }
}

impl std::fmt::Display for NetworkState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match serde_yaml::to_string(self) {
                Ok(s) => s,
                Err(e) => e.to_string(),
            }
        )
    }
}

impl From<Vec<NetworkCommit>> for NetworkState {
    fn from(commits: Vec<NetworkCommit>) -> Self {
        let mut ret = NetworkState::default();
        for commit in commits {
            ret.update_state(&commit.desired_state);
        }
        ret
    }
}
