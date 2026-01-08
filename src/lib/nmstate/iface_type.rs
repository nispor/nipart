// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original file are:
//  * Gris Ge <fge@redhat.com>
//  * Fernando Fernandez Mancera <ffmancera@riseup.net>
//  * Ales Musil <amusil@redhat.com>
//  * Quique Llorente <ellorent@redhat.com>
//  * Wen Liang <liangwen12year@gmail.com>
//  * Íñigo Huguet <ihuguet@redhat.com>

use serde::{Deserialize, Serialize};

use crate::JsonDisplay;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    JsonDisplay,
)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
/// Interface type
pub enum InterfaceType {
    /// Bond interface.
    /// Deserialize and serialize from/to 'bond'
    Bond,
    /// Bridge provided by Linux kernel.
    /// Deserialize and serialize from/to 'linux-bridge'.
    LinuxBridge,
    /// Dummy interface.
    /// Deserialize and serialize from/to 'dummy'.
    Dummy,
    /// Ethernet interface.
    /// Deserialize and serialize from/to 'ethernet'.
    Ethernet,
    /// HSR interface.
    /// Deserialize and serialize from/to 'hsr'.
    Hsr,
    /// Loopback interface.
    /// Deserialize and serialize from/to 'loopback'.
    Loopback,
    /// MAC VLAN interface.
    /// Deserialize and serialize from/to 'mac-vlan'.
    MacVlan,
    /// MAC VTAP interface.
    /// Deserialize and serialize from/to 'mac-vtap'.
    MacVtap,
    /// OpenvSwitch bridge.
    /// Deserialize and serialize from/to 'ovs-bridge'.
    OvsBridge,
    /// OpenvSwitch system interface.
    /// Deserialize and serialize from/to 'ovs-interface'.
    OvsInterface,
    /// Virtual ethernet provide by Linux kernel.
    /// Deserialize and serialize from/to 'veth'.
    Veth,
    /// VLAN interface.
    /// Deserialize and serialize from/to 'vlan'.
    Vlan,
    /// Virtual Routing and Forwarding interface
    /// Deserialize and serialize from/to 'vrf'.
    Vrf,
    /// VxVLAN interface.
    /// Deserialize and serialize from/to 'vxlan'.
    Vxlan,
    /// IP over InfiniBand interface
    /// Deserialize and serialize from/to 'infiniband'.
    #[serde(rename = "infiniband")]
    InfiniBand,
    /// TUN interface.
    /// Deserialize and serialize from/to 'tun'.
    Tun,
    /// MACsec interface.
    /// Deserialize and serialize from/to 'macsec'
    #[serde(rename = "macsec")]
    MacSec,
    /// Ipsec connection.
    Ipsec,
    /// Linux Xfrm kernel interface.
    Xfrm,
    /// IPVLAN kernel interface
    #[serde(rename = "ipvlan")]
    IpVlan,
    /// WiFi Physical Interface
    WifiPhy,
    /// Pseudo interface for WiFi Configuration
    WifiCfg,
    /// Interface unknown
    #[serde(untagged)]
    Unknown(String),
}

impl Default for InterfaceType {
    fn default() -> Self {
        Self::Unknown("unknown".to_string())
    }
}

impl InterfaceType {
    pub fn is_unknown(&self) -> bool {
        matches!(self, InterfaceType::Unknown(_))
    }

    /// Whether interface only exist in userspace configuration without
    /// any kernel interface index.
    pub fn is_userspace(&self) -> bool {
        matches!(
            self,
            InterfaceType::Unknown(_)
                | InterfaceType::OvsBridge
                | InterfaceType::WifiCfg,
        )
    }

    pub fn is_controller(&self) -> bool {
        matches!(
            self,
            InterfaceType::OvsBridge
                | InterfaceType::Bond
                | InterfaceType::Hsr
                | InterfaceType::Vrf,
        )
    }

    pub fn is_supported(&self) -> bool {
        matches!(
            self,
            InterfaceType::Ethernet
                | InterfaceType::LinuxBridge
                | InterfaceType::OvsBridge
                | InterfaceType::OvsInterface
                | InterfaceType::Veth
                | InterfaceType::Loopback
                | InterfaceType::Dummy
                | InterfaceType::Vlan
                | InterfaceType::WifiPhy
                | InterfaceType::Bond
        )
    }

    /// OVS interface cannot live without controller.
    pub(crate) fn need_controller(&self) -> bool {
        self == &InterfaceType::OvsInterface
    }
}
