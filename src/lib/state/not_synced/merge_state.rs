// SPDX-License-Identifier: Apache-2.0

use super::super::{
    MergedInterface, MergedNetworkState, NetworkState, NipartError,
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
    fn update_state(&mut self, other: &Self) {
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
        self.routes
            .verify(&current.routes, ignored_kernel_ifaces.as_slice())?;
        self.rules
            .verify(&current.rules, ignored_kernel_ifaces.as_slice())?;
        self.dns.verify(current.dns.clone().unwrap_or_default())?;
        self.ovsdb
            .verify(current.ovsdb.clone().unwrap_or_default())?;
        self.ovn.verify(&current.ovn)?;
        Ok(())
    }
}
