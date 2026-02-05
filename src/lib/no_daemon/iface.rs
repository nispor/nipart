// SPDX-License-Identifier: Apache-2.0

use super::{
    base_iface::apply_base_iface_link_changes, bond::apply_bond_conf,
    ethernet::apply_ethernet_conf, linux_bridge::apply_bridge_conf,
    vlan::apply_vlan_conf, wireguard::apply_wg_conf,
};
use crate::{
    BaseInterface, Interface, InterfaceState, InterfaceType, MergedInterfaces,
    NipartError, NipartstateInterface,
};

pub(crate) fn nmstate_iface_type_to_nispor(
    iface_type: &InterfaceType,
) -> nispor::IfaceType {
    match iface_type {
        InterfaceType::Ethernet => nispor::IfaceType::Ethernet,
        InterfaceType::Loopback => nispor::IfaceType::Loopback,
        InterfaceType::Veth => nispor::IfaceType::Veth,
        InterfaceType::WifiPhy => nispor::IfaceType::Wifi,
        InterfaceType::Dummy => nispor::IfaceType::Dummy,
        InterfaceType::Vlan => nispor::IfaceType::Vlan,
        InterfaceType::Bond => nispor::IfaceType::Bond,
        InterfaceType::LinuxBridge => nispor::IfaceType::Bridge,
        InterfaceType::Wireguard => nispor::IfaceType::Wireguard,
        v => {
            log::warn!(
                "BUG: Requesting unsupported interface type {iface_type}"
            );
            nispor::IfaceType::Other(v.to_string())
        }
    }
}

pub(crate) fn nmstate_iface_state_to_nispor(
    iface_state: InterfaceState,
) -> nispor::IfaceState {
    match iface_state {
        InterfaceState::Up => nispor::IfaceState::Up,
        InterfaceState::Down => nispor::IfaceState::Down,
        InterfaceState::Absent => nispor::IfaceState::Absent,
        _ => {
            log::warn!(
                "BUG: Requesting unsupported interface state {iface_state}"
            );
            nispor::IfaceState::Unknown
        }
    }
}

pub(crate) fn init_np_iface(iface: &BaseInterface) -> nispor::IfaceConf {
    let mut np_iface = nispor::IfaceConf::default();
    np_iface.name = iface.name.to_string();
    np_iface.iface_type = Some(nmstate_iface_type_to_nispor(&iface.iface_type));
    np_iface.state = nmstate_iface_state_to_nispor(iface.state);
    np_iface
}

pub(crate) fn apply_iface_link_changes(
    apply_iface: &Interface,
    cur_iface: Option<&Interface>,
    merged_ifaces: &MergedInterfaces,
) -> Result<Vec<nispor::IfaceConf>, NipartError> {
    if should_skip_link_change(apply_iface, cur_iface, merged_ifaces) {
        return Ok(Vec::new());
    }

    let mut np_iface = init_np_iface(apply_iface.base_iface());

    apply_base_iface_link_changes(&mut np_iface, apply_iface.base_iface())?;

    if let Interface::Ethernet(apply_iface) = apply_iface {
        apply_ethernet_conf(np_iface, apply_iface, cur_iface)
    } else if let Interface::Vlan(apply_iface) = apply_iface {
        apply_vlan_conf(np_iface, apply_iface)
    } else if let Interface::Bond(apply_iface) = apply_iface {
        apply_bond_conf(
            np_iface,
            apply_iface,
            if let Some(Interface::Bond(cur_iface)) = cur_iface {
                Some(cur_iface)
            } else {
                None
            },
            merged_ifaces,
        )
    } else if let Interface::LinuxBridge(apply_iface) = apply_iface {
        apply_bridge_conf(
            np_iface,
            apply_iface,
            if let Some(Interface::LinuxBridge(cur_iface)) = cur_iface {
                Some(cur_iface)
            } else {
                None
            },
        )
    } else if let Interface::Wireguard(apply_iface) = apply_iface {
        apply_wg_conf(np_iface, apply_iface)
    } else {
        Ok(vec![np_iface])
    }
}

/// Skip link:
///  * loopback interface cannot be deleted
///  * Absent on non-exist interface
///  * Veth peer should be skipped when both end is marked as absent
fn should_skip_link_change(
    apply_iface: &Interface,
    cur_iface: Option<&Interface>,
    merged_ifaces: &MergedInterfaces,
) -> bool {
    if apply_iface.is_absent() {
        if apply_iface.iface_type() == &InterfaceType::Loopback {
            log::info!(
                "Skipping removing loopback interface because it cannot be \
                 deleted",
            );
            return true;
        }
        if !apply_iface.is_virtual() {
            log::debug!(
                "Skipping removing interface {} because it is not virtual",
                apply_iface.name()
            );
            return true;
        }
        if cur_iface.is_none() {
            log::info!(
                "Skipping removing interface {}/{} because it does not exists",
                apply_iface.name(),
                apply_iface.iface_type()
            );
            return true;
        }
        if let Some(Interface::Ethernet(cur_iface)) = cur_iface
            && let Some(peer) = cur_iface.veth.as_ref().map(|v| v.peer.as_str())
            && peer > cur_iface.base.name.as_str()
            && let Some(peer_iface) = merged_ifaces
                .kernel_ifaces
                .get(peer)
                .and_then(|m| m.for_apply.as_ref())
            && peer_iface.is_absent()
        {
            log::info!(
                "Skipping removing interface {}/{} because its veth peer is \
                 already marked as absent",
                apply_iface.name(),
                apply_iface.iface_type()
            );
            return true;
        }
    }
    false
}
