// SPDX-License-Identifier: Apache-2.0

use crate::{
    BaseInterface, NipartError, WireguardConfig, WireguardInterface,
    WireguardIpAddress, WireguardPeerConfig,
};

impl From<&nispor::WireguardInfo> for WireguardConfig {
    fn from(np_wg: &nispor::WireguardInfo) -> Self {
        WireguardConfig {
            public_key: np_wg.public_key.clone(),
            private_key: np_wg.private_key.clone(),
            listen_port: np_wg.listen_port,
            fwmark: np_wg.fwmark,
            peers: np_wg.peers.as_ref().map(|np_peers| {
                np_peers.iter().map(|np_peer| np_peer.into()).collect()
            }),
        }
    }
}

impl From<&WireguardConfig> for nispor::WireguardConf {
    fn from(v: &WireguardConfig) -> Self {
        let mut np_wg = Self::default();
        np_wg.private_key = v.private_key.clone();
        np_wg.listen_port = v.listen_port;
        np_wg.fwmark = v.fwmark;
        np_wg.peers = v
            .peers
            .as_ref()
            .map(|peers| peers.iter().map(|peer| peer.into()).collect());
        np_wg
    }
}

impl From<&nispor::WireguardPeerInfo> for WireguardPeerConfig {
    fn from(np_wg: &nispor::WireguardPeerInfo) -> Self {
        Self {
            endpoint: np_wg.endpoint,
            public_key: np_wg.public_key.clone(),
            preshared_key: np_wg.preshared_key.clone(),
            last_handshake: np_wg.last_handshake.clone(),
            rx_bytes: np_wg.rx_bytes,
            tx_bytes: np_wg.tx_bytes,
            persistent_keepalive: np_wg.persistent_keepalive,
            allowed_ips: np_wg.allowed_ips.as_ref().map(|np_ips| {
                np_ips.iter().map(|np_ip| np_ip.into()).collect()
            }),
            protocol_version: np_wg.protocol_version,
        }
    }
}

impl From<&WireguardPeerConfig> for nispor::WireguardPeerConf {
    fn from(v: &WireguardPeerConfig) -> Self {
        let mut np_wg = Self::default();
        np_wg.endpoint = v.endpoint;
        np_wg.public_key = v.public_key.clone();
        np_wg.preshared_key = v.preshared_key.clone();
        np_wg.persistent_keepalive = v.persistent_keepalive;
        np_wg.allowed_ips = v
            .allowed_ips
            .as_ref()
            .map(|ips| ips.iter().map(|ip| ip.into()).collect());
        np_wg
    }
}

impl From<&nispor::WireguardIpAddress> for WireguardIpAddress {
    fn from(np_wg: &nispor::WireguardIpAddress) -> Self {
        Self {
            ip: np_wg.address,
            prefix_length: np_wg.prefix_length,
        }
    }
}

impl From<&WireguardIpAddress> for nispor::WireguardIpAddress {
    fn from(v: &WireguardIpAddress) -> Self {
        Self {
            address: v.ip,
            prefix_length: v.prefix_length,
        }
    }
}

pub(crate) fn apply_wg_conf(
    mut np_iface: nispor::IfaceConf,
    iface: &WireguardInterface,
) -> Result<Vec<nispor::IfaceConf>, NipartError> {
    if let Some(wg_conf) = iface.wireguard.as_ref() {
        np_iface.wireguard = Some(wg_conf.into());
    }
    Ok(vec![np_iface])
}

impl WireguardInterface {
    pub(crate) fn new_from_nispor(
        base_iface: BaseInterface,
        np_iface: &nispor::Iface,
    ) -> Self {
        if let Some(np_wg_conf) = np_iface.wireguard.as_ref() {
            Self {
                base: base_iface,
                wireguard: Some(np_wg_conf.into()),
            }
        } else {
            Self {
                base: base_iface,
                ..Default::default()
            }
        }
    }
}
