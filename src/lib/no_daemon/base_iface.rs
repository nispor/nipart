// SPDX-License-Identifier: Apache-2.0

use super::{
    iface::nmstate_iface_state_to_nispor,
    ip::{np_ipv4_to_nmstate, np_ipv6_to_nmstate},
};
use crate::{
    NipartError,
    nmstate::{BaseInterface, InterfaceState, InterfaceType},
};

fn np_iface_type_to_nmstate(
    np_iface_type: &nispor::IfaceType,
) -> InterfaceType {
    match np_iface_type {
        nispor::IfaceType::Bond => InterfaceType::Bond,
        nispor::IfaceType::Bridge => InterfaceType::LinuxBridge,
        nispor::IfaceType::Dummy => InterfaceType::Dummy,
        nispor::IfaceType::Ethernet => InterfaceType::Ethernet,
        nispor::IfaceType::Hsr => InterfaceType::Hsr,
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
        nispor::IfaceType::IpVlan => InterfaceType::IpVlan,
        nispor::IfaceType::Wifi => InterfaceType::WifiPhy,
        nispor::IfaceType::Other(v) => InterfaceType::Unknown(v.to_lowercase()),
        _ => {
            InterfaceType::Unknown(format!("{np_iface_type:?}").to_lowercase())
        }
    }
}

fn np_iface_state_to_nmstate(
    state: &nispor::IfaceState,
    flags: &[nispor::IfaceFlag],
) -> InterfaceState {
    // nispor::IfaceState::Up means operational up.
    // Check also the Running flag with, according to [1], means operational
    // state Up or Unknown.
    // [1] https://www.kernel.org/doc/Documentation/networking/operstates.txt
    //
    // For nmstate the `state: up` also means administratively up, hence
    // `IfaceFlag::Up` also means `state: up`
    if *state == nispor::IfaceState::Up
        || flags.contains(&nispor::IfaceFlag::Running)
        || flags.contains(&nispor::IfaceFlag::Up)
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
) -> BaseInterface {
    let mut base_iface = BaseInterface {
        name: np_iface.name.to_string(),
        state: np_iface_state_to_nmstate(
            &np_iface.state,
            np_iface.flags.as_slice(),
        ),
        iface_index: Some(np_iface.index),
        iface_type: np_iface_type_to_nmstate(&np_iface.iface_type),
        mac_address: Some(np_iface.mac_address.to_uppercase()),
        permanent_mac_address: get_permanent_mac_address(np_iface),
        controller: np_iface.controller.as_ref().map(|c| c.to_string()),
        mtu: if np_iface.mtu >= 0 {
            Some(np_iface.mtu as u64)
        } else {
            Some(0u64)
        },
        min_mtu: if let Some(mtu) = np_iface.min_mtu {
            if mtu >= 0 { Some(mtu as u64) } else { None }
        } else {
            None
        },
        max_mtu: if let Some(mtu) = np_iface.max_mtu {
            if mtu >= 0 { Some(mtu as u64) } else { None }
        } else {
            None
        },
        ..Default::default()
    };
    if !base_iface.iface_type.is_supported() {
        log::trace!(
            "Got unsupported interface type {}: {}, ignoring",
            &base_iface.iface_type,
            &base_iface.name
        );
        base_iface.state = InterfaceState::Ignore;
    }
    base_iface.ipv4 = np_ipv4_to_nmstate(np_iface);
    base_iface.ipv6 = np_ipv6_to_nmstate(np_iface);

    base_iface
}

fn get_permanent_mac_address(iface: &nispor::Iface) -> Option<String> {
    if iface.permanent_mac_address.is_empty() {
        // Bond port also hold perm_hwaddr which is the mac address before
        // this interface been assgined to bond as subordinate.
        if let Some(bond_port_info) = &iface.bond_port {
            if bond_port_info.perm_hwaddr.is_empty() {
                None
            } else {
                Some(bond_port_info.perm_hwaddr.as_str().to_uppercase())
            }
        } else {
            None
        }
    } else {
        Some(iface.permanent_mac_address.as_str().to_uppercase())
    }
}

/// Apply changes to [BaseInterface] except the IP layer stuff.
pub(crate) fn apply_base_iface_link_changes(
    np_iface: &mut nispor::IfaceConf,
    apply_iface: &BaseInterface,
) -> Result<(), NipartError> {
    // We do not check current property state, nispor will ignore unchanged
    // property.

    np_iface.state = nmstate_iface_state_to_nispor(apply_iface.state);
    // It is OK to use `as` action here:
    // 1. Pre-apply sanitize checker already confirmed it never exceed max_mtu,
    //    in linux kernel, the ethernet max MTU is u32::MAX.
    // 2. Verification process will complains if overflow a u32 for special
    //    interface which support MTU bigger than u32::MAX.
    np_iface.mtu = apply_iface.mtu.map(|mtu| mtu as u32);

    if apply_iface.iface_type != InterfaceType::OvsInterface {
        np_iface.controller = apply_iface.controller.clone();
    }
    Ok(())
}
