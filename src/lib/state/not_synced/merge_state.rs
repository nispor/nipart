// SPDX-License-Identifier: Apache-2.0

use crate::{
    HostNameState, MergedInterface, MergedNetworkState, NetworkState,
    NipartDhcpConfig, NipartDhcpConfigV4, NipartDhcpConfigV6, NipartError,
};

impl NetworkState {
    pub fn merge_states(mut states: Vec<(NetworkState, u32)>) -> Self {
        states.sort_unstable_by_key(|s| s.1);
        let mut ret = Self::default();
        for state in states {
            log::trace!(
                "Merging {state:?} into {ret:?} with priority {}",
                state.1
            );
            ret.update_state(&state.0)
        }
        ret
    }
}

// This one is copy from NetworkState::update_state() in
// query_apply/net_state.rs which is removed by copy_nmstate_code.py

impl NetworkState {
    pub(crate) fn update_state(&mut self, other: &Self) {
        if let Some(other_hostname) = other.hostname.as_ref() {
            if let Some(h) = self.hostname.as_mut() {
                h.update(other_hostname);
            } else {
                self.hostname = other.hostname.clone();
            }
        }
        self.interfaces.update(&other.interfaces);
        if other.dns.is_some() {
            self.dns = other.dns.clone();
        }
        if other.ovsdb.is_some() {
            self.ovsdb = other.ovsdb.clone();
        }
        if !other.ovn.is_none() {
            self.ovn = other.ovn.clone();
        }
    }
}

impl MergedNetworkState {
    pub fn get_dhcp_changes(&self) -> Vec<NipartDhcpConfig> {
        let mut ret: Vec<NipartDhcpConfig> = Vec::new();
        for iface in self
            .interfaces
            .kernel_ifaces
            .values()
            .filter_map(|i| i.for_apply.as_ref())
        {
            if iface.base_iface().can_have_ip() {
                if let Some(ipv4) = iface.base_iface().ipv4.as_ref() {
                    let dhcp_conf = NipartDhcpConfigV4::new(
                        iface.name().to_string(),
                        ipv4.enabled && ipv4.dhcp == Some(true),
                    );
                    if ipv4.dhcp_client_id.as_ref().is_some() {
                        todo!()
                    }
                    ret.push(NipartDhcpConfig::V4(dhcp_conf));
                }
                if let Some(ipv6) = iface.base_iface().ipv6.as_ref() {
                    let dhcp_conf = NipartDhcpConfigV6::new(
                        iface.name().to_string(),
                        ipv6.enabled && ipv6.dhcp == Some(true),
                    );
                    if ipv6.dhcp_duid.as_ref().is_some() {
                        todo!()
                    }
                    ret.push(NipartDhcpConfig::V6(dhcp_conf));
                }
            } else {
                ret.push(NipartDhcpConfig::V4(NipartDhcpConfigV4::new(
                    iface.name().to_string(),
                    false,
                )));
                ret.push(NipartDhcpConfig::V6(NipartDhcpConfigV6::new(
                    iface.name().to_string(),
                    false,
                )));
            }
        }
        ret
    }

    pub fn get_desired_hostname(&self) -> Option<&HostNameState> {
        self.hostname.desired.as_ref()
    }

    pub fn verify(&self, current: &NetworkState) -> Result<(), NipartError> {
        self.hostname.verify(current.hostname.as_ref())?;
        self.interfaces.verify(&current.interfaces)?;
        let ignored_kernel_ifaces: Vec<&str> = self
            .interfaces
            .ignored_ifaces
            .as_slice()
            .iter()
            .filter(|(_, t)| !t.is_userspace())
            .map(|(n, _)| n.as_str())
            .collect();
        self.routes.verify(
            &current.routes,
            ignored_kernel_ifaces.as_slice(),
            &current.interfaces,
        )?;
        self.rules
            .verify(&current.rules, ignored_kernel_ifaces.as_slice())?;
        self.dns.verify(current.dns.clone().unwrap_or_default())?;
        self.ovsdb
            .verify(current.ovsdb.clone().unwrap_or_default())?;
        self.ovn.verify(&current.ovn)?;
        Ok(())
    }
}
