// SPDX-License-Identifier: Apache-2.0

use nipart::{
    BaseInterface, EthernetConfig, EthernetDuplex, EthernetInterface,
};

pub(crate) fn np_ethernet_to_nipart(
    np_iface: &nispor::Iface,
    base_iface: BaseInterface,
) -> EthernetInterface {
    let mut iface = EthernetInterface::default();
    iface.base = base_iface;
    iface.ethernet = Some(gen_eth_conf(np_iface));
    iface
}

fn gen_eth_conf(np_iface: &nispor::Iface) -> EthernetConfig {
    let mut eth_conf = EthernetConfig::default();
    if let Some(ethtool_info) = &np_iface.ethtool {
        if let Some(link_mode_info) = &ethtool_info.link_mode {
            if link_mode_info.speed > 0 {
                eth_conf.speed = Some(link_mode_info.speed);
            }
            eth_conf.auto_neg = Some(link_mode_info.auto_negotiate);
            match link_mode_info.duplex {
                nispor::EthtoolLinkModeDuplex::Full => {
                    eth_conf.duplex = Some(EthernetDuplex::Full);
                }
                nispor::EthtoolLinkModeDuplex::Half => {
                    eth_conf.duplex = Some(EthernetDuplex::Half);
                }
                _ => (),
            }
        }
    }
    eth_conf
}
