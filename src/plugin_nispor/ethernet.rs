// SPDX-License-Identifier: Apache-2.0

use nipart::{BaseInterface, EthernetConfig, EthernetInterface};

pub(crate) fn np_ethernet_to_nipart(
    np_iface: &nispor::Iface,
    base_iface: BaseInterface,
) -> EthernetInterface {
    let mut iface = EthernetInterface::default();
    iface.base = base_iface;
    iface.ethernet = Some(gen_eth_conf(np_iface));
    iface
}

fn gen_eth_conf(_np_iface: &nispor::Iface) -> EthernetConfig {
    // TODO
    EthernetConfig::default()
}
