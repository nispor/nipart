// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{ErrorKind, Interfaces, NipartError};

/// Network state
///
/// [NetworkState] is idempotent meaning it could be applied multiple times and
/// generate the same final network state.
///
/// Example yaml(many lines omitted) serialized NetworkState would be:
///
/// ```yaml
/// version: 1
/// hostname:
///   running: host.example.org
///   config: host.example.org
/// dns-resolver:
///   config:
///     server:
///     - 2001:db8:1::
///     - 192.0.2.1
///     search: []
/// route-rules:
///   config:
///   - ip-from: 2001:db8:b::/64
///     priority: 30000
///     route-table: 200
///   - ip-from: 192.0.2.2/32
///     priority: 30000
///     route-table: 200
/// routes:
///   config:
///   - destination: 2001:db8:a::/64
///     next-hop-interface: eth1
///     next-hop-address: 2001:db8:1::2
///     metric: 108
///     table-id: 200
///   - destination: 192.168.2.0/24
///     next-hop-interface: eth1
///     next-hop-address: 192.168.1.3
///     metric: 108
///     table-id: 200
/// interfaces:
/// - name: eth1
///   type: ethernet
///   state: up
///   mac-address: 0E:F9:2B:28:42:D9
///   mtu: 1500
///   ipv4:
///     enabled: true
///     dhcp: false
///     address:
///     - ip: 192.168.1.3
///       prefix-length: 24
///   ipv6:
///     enabled: true
///     dhcp: false
///     autoconf: false
///     address:
///     - ip: 2001:db8:1::1
///       prefix-length: 64
/// ovs-db:
///   external_ids:
///     hostname: host.example.org
///     rundir: /var/run/openvswitch
///     system-id: 176866c7-6dc8-400f-98ac-c658509ec09f
///   other_config: {}
/// ```
#[derive(Clone, Debug, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct NetworkState {
    /// Please set it to 1 explicitly
    #[serde(default)]
    pub version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    /// Description for the whole desire state.
    pub description: String,
    /// Network interfaces
    #[serde(
        default,
        skip_serializing_if = "Interfaces::is_empty",
        rename = "interfaces"
    )]
    pub ifaces: Interfaces,
}

impl NetworkState {
    pub const HIDE_PASSWORD_STR: &str = "<_password_hidden_by_nipart>";

    pub fn hide_secrets(&mut self) {
        log::debug!("Replacing secrets with {}", Self::HIDE_PASSWORD_STR);
        self.ifaces.hide_secrets();
    }

    pub fn is_empty(&self) -> bool {
        self == &Self {
            version: self.version,
            ..Default::default()
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(&mut self, new_state: &Self) -> Result<(), NipartError> {
        self.ifaces.merge(&new_state.ifaces)?;
        Ok(())
    }

    /// Wrapping function of [serde_yaml::from_str()] with error mapped to
    /// [NmstateError].
    pub fn new_from_yaml(net_state_yaml: &str) -> Result<Self, NipartError> {
        match serde_yaml::from_str(net_state_yaml) {
            Ok(s) => Ok(s),
            Err(e) => Err(NipartError::new(
                ErrorKind::InvalidArgument,
                format!("Invalid YAML string: {e}"),
            )),
        }
    }
}

impl std::fmt::Display for NetworkState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dup_self = self.clone();
        dup_self.hide_secrets();
        write!(
            f,
            "{}",
            match serde_yaml::to_string(&dup_self) {
                Ok(s) => s,
                Err(e) => e.to_string(),
            }
        )
    }
}
