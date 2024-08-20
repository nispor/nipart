// SPDX-License-Identifier: Apache-2.0

use nipart::{BaseInterface, VrfConfig, VrfInterface};

pub(crate) fn np_vrf_to_nipart(
    np_iface: &nispor::Iface,
    base_iface: BaseInterface,
) -> VrfInterface {
    let vrf_conf = np_iface.vrf.as_ref().map(|np_vrf_info| {
        let mut conf = VrfConfig::default();
        conf.table_id = Some(np_vrf_info.table_id);
        conf.port = {
            let mut ports = np_vrf_info.subordinates.clone();
            ports.sort_unstable();
            Some(ports)
        };
        conf
    });
    let mut ret = VrfInterface::default();
    ret.base = base_iface;
    ret.vrf = vrf_conf;
    ret
}
