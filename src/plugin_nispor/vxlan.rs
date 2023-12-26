// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use nipart::{BaseInterface, VxlanConfig, VxlanInterface};

pub(crate) fn np_vxlan_to_nipart(
    np_iface: &nispor::Iface,
    base_iface: BaseInterface,
) -> VxlanInterface {
    let vxlan_conf = np_iface.vxlan.as_ref().map(|np_vxlan_info| {
        let mut conf = VxlanConfig::default();
        conf.id = np_vxlan_info.vxlan_id;
        conf.base_iface = np_vxlan_info.base_iface.clone();
        conf.learning = Some(np_vxlan_info.learning);
        conf.local =
            std::net::IpAddr::from_str(np_vxlan_info.local.as_str()).ok();
        conf.remote =
            std::net::IpAddr::from_str(np_vxlan_info.remote.as_str()).ok();
        conf.dst_port = Some(np_vxlan_info.dst_port);
        conf
    });

    let mut ret = VxlanInterface::default();

    ret.base = base_iface;
    ret.vxlan = vxlan_conf;
    ret
}
