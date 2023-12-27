// SPDX-License-Identifier: Apache-2.0

use super::super::{MergedInterface, MergedNetworkState, NetworkState};

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
        if other.prop_list.contains(&"hostname") {
            if let Some(h) = self.hostname.as_mut() {
                if let Some(other_h) = other.hostname.as_ref() {
                    h.update(other_h);
                }
            } else {
                self.hostname = other.hostname.clone();
            }
        }
        if other.prop_list.contains(&"interfaces") {
            self.interfaces.update(&other.interfaces);
        }
        if other.prop_list.contains(&"dns") {
            self.dns = other.dns.clone();
        }
        if other.prop_list.contains(&"ovsdb") {
            self.ovsdb = other.ovsdb.clone();
        }
        if other.prop_list.contains(&"ovn") {
            self.ovn = other.ovn.clone();
        }
    }
}

impl MergedNetworkState {
    // TODO: a lot effort required here
    pub fn gen_state_for_apply(&self) -> NetworkState {
        let mut ret = NetworkState::default();
        let mut ifaces: Vec<&MergedInterface> =
            self.interfaces.iter().filter(|i| i.is_changed()).collect();

        ifaces.sort_unstable_by_key(|iface| iface.merged.name());
        // Use sort_by_key() instead of unstable one, so we can alphabet
        // activation order which is required to simulate the OS boot-up.
        ifaces.sort_by_key(|iface| {
            if let Some(i) = iface.for_apply.as_ref() {
                i.base_iface().up_priority
            } else {
                u32::MAX
            }
        });
        for iface in ifaces {
            if let Some(i) = iface.for_apply.as_ref() {
                ret.interfaces.push(i.clone());
            }
        }
        ret
    }
}
