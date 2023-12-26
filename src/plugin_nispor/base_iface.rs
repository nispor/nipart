// SPDX-License-Identifier: Apache-2.0

use nipart::{BaseInterface, InterfaceState, InterfaceType};

use crate::{
    ethtool::np_ethtool_to_nipart,
    ip::{np_ipv4_to_nipart, np_ipv6_to_nipart},
    mptcp::get_iface_mptcp_conf,
};

fn np_iface_type_to_nipart(np_iface_type: &nispor::IfaceType) -> InterfaceType {
    match np_iface_type {
        nispor::IfaceType::Bond => InterfaceType::Bond,
        nispor::IfaceType::Bridge => InterfaceType::LinuxBridge,
        nispor::IfaceType::Dummy => InterfaceType::Dummy,
        nispor::IfaceType::Ethernet => InterfaceType::Ethernet,
        nispor::IfaceType::Loopback => InterfaceType::Loopback,
        nispor::IfaceType::MacSec => InterfaceType::MacSec,
        nispor::IfaceType::MacVlan => InterfaceType::MacVlan,
        nispor::IfaceType::MacVtap => InterfaceType::MacVtap,
        nispor::IfaceType::OpenvSwitch => InterfaceType::OvsInterface,
        nispor::IfaceType::Veth => InterfaceType::Veth,
        nispor::IfaceType::Vlan => InterfaceType::Vlan,
        nispor::IfaceType::Vrf => InterfaceType::Vrf,
        nispor::IfaceType::Vxlan => InterfaceType::Vxlan,
        nispor::IfaceType::Ipoib => InterfaceType::InfiniBand,
        nispor::IfaceType::Tun => InterfaceType::Tun,
        nispor::IfaceType::Xfrm => InterfaceType::Xfrm,
        nispor::IfaceType::Other(v) => InterfaceType::Other(v.to_lowercase()),
        _ => InterfaceType::Other(format!("{np_iface_type:?}").to_lowercase()),
    }
}

fn np_ifaec_state_to_nipart(
    state: &nispor::IfaceState,
    flags: &[nispor::IfaceFlag],
) -> InterfaceState {
    if *state == nispor::IfaceState::Up
        || flags.contains(&nispor::IfaceFlag::Up)
        || flags.contains(&nispor::IfaceFlag::Running)
    {
        InterfaceState::Up
    } else if *state == nispor::IfaceState::Down {
        InterfaceState::Down
    } else {
        InterfaceState::Unknown
    }
}

pub(crate) fn np_iface_to_base_iface(
    np_iface: &nispor::Iface,
    running_config_only: bool,
) -> BaseInterface {
    let mut base_iface = BaseInterface::default();

    base_iface.name = np_iface.name.to_string();
    base_iface.state =
        np_ifaec_state_to_nipart(&np_iface.state, np_iface.flags.as_slice());
    base_iface.iface_type = np_iface_type_to_nipart(&np_iface.iface_type);
    base_iface.ipv4 = np_ipv4_to_nipart(np_iface, running_config_only);
    base_iface.ipv6 = np_ipv6_to_nipart(np_iface, running_config_only);
    base_iface.mac_address = Some(np_iface.mac_address.to_uppercase());
    base_iface.permanent_mac_address = get_permanent_mac_address(np_iface);
    base_iface.controller = np_iface.controller.as_ref().map(|c| c.to_string());
    base_iface.mtu = if np_iface.mtu >= 0 {
        Some(np_iface.mtu as u64)
    } else {
        Some(0u64)
    };
    base_iface.min_mtu = if !running_config_only {
        if let Some(mtu) = np_iface.min_mtu {
            if mtu >= 0 {
                Some(mtu as u64)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    base_iface.max_mtu = if !running_config_only {
        if let Some(mtu) = np_iface.max_mtu {
            if mtu >= 0 {
                Some(mtu as u64)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    base_iface.accept_all_mac_addresses =
        if np_iface.flags.contains(&nispor::IfaceFlag::Promisc) {
            Some(true)
        } else {
            Some(false)
        };
    base_iface.ethtool = np_ethtool_to_nipart(np_iface);
    base_iface.prop_list = vec![
        "name",
        "state",
        "iface_type",
        "ipv4",
        "ipv6",
        "mac_address",
        "permanent_mac_address",
        "controller",
        "mtu",
        "accept_all_mac_addresses",
        "ethtool",
    ];
    if !InterfaceType::SUPPORTED_LIST.contains(&base_iface.iface_type) {
        log::info!(
            "Got unsupported interface type {}: {}, ignoring",
            &base_iface.iface_type,
            &base_iface.name
        );
        base_iface.state = InterfaceState::Ignore;
    }

    base_iface.mptcp = get_iface_mptcp_conf(&base_iface);

    base_iface
}

fn get_permanent_mac_address(iface: &nispor::Iface) -> Option<String> {
    if iface.permanent_mac_address.is_empty() {
        // Bond port also hold perm_hwaddr which is the mac address before
        // this interface been assgined to bond as subordinate.
        if let Some(bond_port_info) = &iface.bond_subordinate {
            if bond_port_info.perm_hwaddr.is_empty() {
                None
            } else {
                Some(bond_port_info.perm_hwaddr.as_str().to_uppercase())
            }
        } else {
            None
        }
    } else {
        Some(iface.permanent_mac_address.clone())
    }
}
