// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
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
    /// Interface unknown to Nipart
    #[serde(untagged)]
    Unknown(String),
}

impl Default for InterfaceType {
    fn default() -> Self {
        Self::Unknown("unknown".to_string())
    }
}

impl std::fmt::Display for InterfaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InterfaceType::Bond => "bond",
                InterfaceType::LinuxBridge => "linux-bridge",
                InterfaceType::Dummy => "dummy",
                InterfaceType::Ethernet => "ethernet",
                InterfaceType::Hsr => "hsr",
                InterfaceType::Loopback => "loopback",
                InterfaceType::MacVlan => "mac-vlan",
                InterfaceType::MacVtap => "mac-vtap",
                InterfaceType::OvsBridge => "ovs-bridge",
                InterfaceType::OvsInterface => "ovs-interface",
                InterfaceType::Veth => "veth",
                InterfaceType::Vlan => "vlan",
                InterfaceType::Vrf => "vrf",
                InterfaceType::Vxlan => "vxlan",
                InterfaceType::InfiniBand => "infiniband",
                InterfaceType::Tun => "tun",
                InterfaceType::MacSec => "macsec",
                InterfaceType::Ipsec => "ipsec",
                InterfaceType::Xfrm => "xfrm",
                InterfaceType::IpVlan => "ipvlan",
                InterfaceType::Unknown(s) => s,
            }
        )
    }
}

impl InterfaceType {
    pub fn is_unknown(&self) -> bool {
        matches!(self, InterfaceType::Unknown(_))
    }
}
