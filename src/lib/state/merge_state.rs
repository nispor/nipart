// SPDX-License-Identifier: Apache-2.0

use super::super::NetworkState;

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
