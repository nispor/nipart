// SPDX-License-Identifier: Apache-2.0

use crate::{
    BaseInterface, EthernetInterface, Interface, InterfaceState, InterfaceType,
    Interfaces, MergedInterface, MergedInterfaces, NipartstateInterface,
};

impl MergedInterfaces {
    pub(crate) fn post_merge_sanitize_veth(&mut self) {
        self.bring_new_veth_peer_up();
        self.mark_veth_peer_absent_also();
    }

    fn bring_new_veth_peer_up(&mut self) {
        let mut new_veth_peers: Vec<MergedInterface> = Vec::new();
        for iface in self.kernel_ifaces.values().filter(|i| i.current.is_none())
        {
            if let Some(Interface::Ethernet(eth_iface)) =
                iface.for_apply.as_ref()
            {
                if let Some(peer) =
                    eth_iface.veth.as_ref().map(|v| v.peer.as_str())
                {
                    if !self.kernel_ifaces.contains_key(peer) {
                        let mut base_iface = BaseInterface::new(
                            peer.to_string(),
                            InterfaceType::Ethernet,
                        );
                        let mut cur_base_iface = base_iface.clone();
                        cur_base_iface.state = InterfaceState::Down;
                        let cur_iface = Interface::Ethernet(Box::new(
                            EthernetInterface::new_veth(
                                cur_base_iface,
                                eth_iface.name(),
                            ),
                        ));

                        // Veth peer should be activated after creation which
                        // is holding up_priority 0
                        base_iface.up_priority = 1;
                        let des_iface = Interface::Ethernet(Box::new(
                            EthernetInterface::new_veth(
                                base_iface,
                                eth_iface.name(),
                            ),
                        ));

                        let mut merged_iface = match MergedInterface::new(
                            Some(des_iface),
                            Some(cur_iface),
                        ) {
                            Ok(i) => i,
                            Err(e) => {
                                log::error!(
                                    "BUG: Cannot create MergedInterface for \
                                     newly created veth peer {}: {e}",
                                    peer
                                );
                                continue;
                            }
                        };
                        merged_iface.for_verify = None;

                        new_veth_peers.push(merged_iface);
                    }
                }
            }
        }
        for merged_iface in new_veth_peers {
            let iface_name = merged_iface.merged.name().to_string();
            self.kernel_ifaces.insert(iface_name, merged_iface);
        }
    }

    /// When veth is marked as absent, its peer should be marked as absent
    /// if not desired.
    fn mark_veth_peer_absent_also(&mut self) {
        let mut pending_changes: Vec<String> = Vec::new();
        for iface in self.kernel_ifaces.values().filter(|i| {
            i.for_apply.as_ref().map(|i| i.is_absent()) == Some(true)
        }) {
            if let Some(Interface::Ethernet(iface)) = iface.current.as_ref() {
                if let Some(peer) = iface.veth.as_ref().map(|v| v.peer.as_str())
                {
                    if let Some(peer_iface) = self.kernel_ifaces.get(peer) {
                        if peer_iface.desired.is_none() {
                            pending_changes.push(peer.to_string());
                        }
                    }
                }
            }
        }
        for iface_name in pending_changes {
            if let Some(iface) = self
                .kernel_ifaces
                .get_mut(&iface_name)
                .and_then(|i| i.for_apply.as_mut())
            {
                iface.base_iface_mut().state = InterfaceState::Absent;
            }
        }
    }
}

impl Interfaces {
    // * Sync veth state in new_ifaces to its peer(if exist) also
    pub(crate) fn post_merge_veth(&mut self, new_ifaces: &Self) {
        // Holds Vec<veth_peer_name, interface_type>
        let mut pending_changes: Vec<(String, InterfaceState)> = Vec::new();
        for new_iface in new_ifaces.kernel_ifaces.values().filter_map(|i| {
            if let Interface::Ethernet(iface) = i {
                Some(iface)
            } else {
                None
            }
        }) {
            let old_iface = if let Some(old_iface) =
                self.kernel_ifaces.get(new_iface.name())
                && let Interface::Ethernet(iface) = old_iface
            {
                iface
            } else {
                continue;
            };
            let peer = if let Some(p) = new_iface
                .veth
                .as_ref()
                .map(|v| v.peer.as_str())
                .or_else(|| old_iface.veth.as_ref().map(|v| v.peer.as_str()))
            {
                p
            } else {
                continue;
            };
            if let Some(peer_iface) = self.kernel_ifaces.get(peer) {
                if peer_iface.base_iface().state != new_iface.base.state {
                    pending_changes
                        .push((peer.to_string(), new_iface.base.state));
                }
            }
        }
        for (peer_name, iface_state) in pending_changes {
            if let Some(iface) = self.kernel_ifaces.get_mut(&peer_name) {
                iface.base_iface_mut().state = iface_state;
            }
        }
    }
}
