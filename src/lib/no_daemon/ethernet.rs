// SPDX-License-Identifier: Apache-2.0

use crate::{
    BaseInterface, EthernetConfig, EthernetDuplex, EthernetInterface,
    Interface, NipartError, NipartstateInterface, VethConfig,
};

pub(crate) fn apply_ethernet_conf(
    mut np_iface: nispor::IfaceConf,
    apply_iface: &EthernetInterface,
    cur_iface: Option<&Interface>,
) -> Result<Vec<nispor::IfaceConf>, NipartError> {
    if apply_iface.is_up()
        && cur_iface.is_none()
        && let Some(peer) =
            // Create new veth, already sanitized, so no need to validate
            apply_iface.veth.as_ref().map(|v| v.peer.as_str())
    {
        np_iface.iface_type = Some(nispor::IfaceType::Veth);
        let mut np_veth_conf = nispor::VethConf::default();
        np_veth_conf.peer = peer.to_string();
        np_iface.veth = Some(np_veth_conf);

        let mut peer_np_iface = nispor::IfaceConf::default();
        peer_np_iface.name = peer.to_string();
        peer_np_iface.iface_type = Some(nispor::IfaceType::Veth);
        peer_np_iface.state = nispor::IfaceState::Up;
        let mut np_veth_conf = nispor::VethConf::default();
        np_veth_conf.peer = apply_iface.name().to_string();
        peer_np_iface.veth = Some(np_veth_conf);
        Ok(vec![np_iface, peer_np_iface])
    } else {
        Ok(vec![np_iface])
    }
}

impl EthernetInterface {
    pub(crate) fn new_from_nispor(
        base: BaseInterface,
        np_iface: &nispor::Iface,
    ) -> Self {
        Self {
            base,
            veth: get_veth_conf(np_iface),
            ethernet: get_eth_conf(np_iface),
        }
    }
}

fn get_veth_conf(np_iface: &nispor::Iface) -> Option<VethConfig> {
    np_iface
        .veth
        .as_ref()
        .map(|v| v.peer.as_str())
        .map(|peer| VethConfig {
            peer: peer.to_string(),
        })
}

fn get_eth_conf(np_iface: &nispor::Iface) -> Option<EthernetConfig> {
    np_iface
        .ethtool
        .as_ref()
        .and_then(|ethtool_info| ethtool_info.link_mode.as_ref())
        .map(|link_mode_info| EthernetConfig {
            speed: link_mode_info.speed,
            auto_neg: Some(link_mode_info.auto_negotiate),
            duplex: match link_mode_info.duplex {
                Some(nispor::EthtoolLinkModeDuplex::Full) => {
                    Some(EthernetDuplex::Full)
                }
                Some(nispor::EthtoolLinkModeDuplex::Half) => {
                    Some(EthernetDuplex::Half)
                }
                _ => None,
            },
        })
}
