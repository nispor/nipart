// SPDX-License-Identifier: Apache-2.0

use nipart::{
    BaseInterface, VlanConfig, VlanInterface, VlanProtocol,
    VlanRegistrationProtocol,
};

pub(crate) fn np_vlan_to_nipart(
    np_iface: &nispor::Iface,
    base_iface: BaseInterface,
) -> VlanInterface {
    let vlan_conf = np_iface.vlan.as_ref().map(|np_vlan_info| {
        let mut conf = VlanConfig::default();
        conf.id = np_vlan_info.vlan_id;
        conf.base_iface = np_vlan_info.base_iface.clone();
        conf.protocol = match &np_vlan_info.protocol {
            nispor::VlanProtocol::Ieee8021Q => Some(VlanProtocol::Ieee8021Q),
            nispor::VlanProtocol::Ieee8021AD => Some(VlanProtocol::Ieee8021Ad),
            p => {
                log::warn!(
                    "Got unknown VLAN protocol {p:?} on VLAN iface {}",
                    np_iface.name.as_str()
                );
                None
            }
        };
        conf.reorder_headers = Some(np_vlan_info.is_reorder_hdr);
        conf.loose_binding = Some(np_vlan_info.is_loose_binding);
        // They are mutually exclusive, vlan cannot be gvrp and mvrp at the
        // same time
        conf.registration_protocol = if np_vlan_info.is_gvrp {
            Some(VlanRegistrationProtocol::Gvrp)
        } else if np_vlan_info.is_mvrp {
            Some(VlanRegistrationProtocol::Mvrp)
        } else {
            Some(VlanRegistrationProtocol::None)
        };
        conf
    });

    let mut ret = VlanInterface::default();
    ret.base = base_iface;
    ret.vlan = vlan_conf;
    ret
}

pub(crate) fn nms_vlan_conf_to_np(
    nms_vlan_conf: Option<&VlanConfig>,
) -> Option<nispor::VlanConf> {
    nms_vlan_conf.map(|nms_vlan_conf| {
        let mut np_vlan_conf = nispor::VlanConf::default();
        np_vlan_conf.vlan_id = nms_vlan_conf.id;
        np_vlan_conf.base_iface = nms_vlan_conf.base_iface.clone();
        np_vlan_conf
    })
}
