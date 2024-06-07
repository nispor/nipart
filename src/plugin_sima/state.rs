// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use nipart::NetworkState;

// TODO:Store related info into a single NetworkState
pub(crate) fn flatten_net_state(
    state: NetworkState,
) -> HashMap<String, NetworkState> {
    let mut ret: HashMap<String, NetworkState> = HashMap::new();

    if state.dns.is_some() {
        let mut tmp_state = NetworkState::default();
        tmp_state.dns = state.dns;
        ret.insert("dns".to_string(), tmp_state);
    }

    // TODO: Move interface routes to interface-specific NetworkState
    if !state.routes.is_empty() {
        let mut tmp_state = NetworkState::default();
        tmp_state.routes = state.routes;
        ret.insert("routes".to_string(), tmp_state);
    }

    if !state.rules.is_empty() {
        let mut tmp_state = NetworkState::default();
        tmp_state.rules = state.rules;
        ret.insert("rules".to_string(), tmp_state);
    }

    if state.ovsdb.is_some() {
        let mut tmp_state = NetworkState::default();
        tmp_state.ovsdb = state.ovsdb;
        ret.insert("ovsdb".to_string(), tmp_state);
    }

    if !state.ovn.is_empty() {
        let mut tmp_state = NetworkState::default();
        tmp_state.ovn = state.ovn;
        ret.insert("ovn".to_string(), tmp_state);
    }

    for iface in state.interfaces.to_vec() {
        let mut tmp_state = NetworkState::default();
        tmp_state.interfaces.push(iface.clone());
        let name = if iface.is_userspace() {
            format!("{}@{}", iface.name(), iface.iface_type())
        } else {
            iface.name().to_string()
        };
        ret.insert(name, tmp_state);
    }

    ret
}
