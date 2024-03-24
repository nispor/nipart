// SPDX-License-Identifier: Apache-2.0

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::NipartEventAddress;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NipartMonitorRule {
    Link(NipartLinkMonitorRule),
    Address(NipartAddressMonitorRule),
}

impl std::fmt::Display for NipartMonitorRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Link(rule) => write!(f, "{rule}"),
            Self::Address(rule) => write!(f, "{rule}"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NipartMonitorEvent {
    /// Interface up
    LinkUp(String),
    /// Interface down
    LinkDown(String),
    /// IP address been removed
    AddressRemove(IpAddr),
}

impl std::fmt::Display for NipartMonitorEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LinkUp(iface) => write!(f, "link_up:{iface}"),
            Self::LinkDown(iface) => write!(f, "link_down:{iface}"),
            Self::AddressRemove(ip) => write!(f, "addr_remove:{ip}"),
        }
    }
}

#[derive(
    Deserialize,
    Serialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Copy,
)]
#[non_exhaustive]
pub enum NipartLinkMonitorKind {
    Up,
    Down,
}

impl std::fmt::Display for NipartLinkMonitorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Up => "up",
                Self::Down => "down",
            }
        )
    }
}

/// Monitor on the link state(`IFLA_OPERSTATE`) up/down event of specified
/// interface
#[derive(
    Deserialize, Serialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[non_exhaustive]
pub struct NipartLinkMonitorRule {
    pub kind: NipartLinkMonitorKind,
    /// Who requested this monitor rule
    pub requester: NipartEventAddress,
    /// Event ID for tracing the source of this request
    pub uuid: u128,
    /// Interface to monitor
    pub iface: String,
}

impl std::fmt::Display for NipartLinkMonitorRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "link_monitor: uuid:{}, kind:{}, requester:{}, iface:{}",
            self.uuid, self.kind, self.requester, self.iface
        )
    }
}

impl NipartLinkMonitorRule {
    pub fn new(
        kind: NipartLinkMonitorKind,
        requester: NipartEventAddress,
        uuid: u128,
        iface: String,
    ) -> Self {
        Self {
            kind,
            requester,
            uuid,
            iface,
        }
    }
}

/// Monitor on the IP address (`IFLA_OPERSTATE`) add/remove event of specified
/// interface
#[derive(
    Deserialize, Serialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[non_exhaustive]
pub struct NipartAddressMonitorRule {
    pub kind: NipartAddressMonitorKind,
    /// Who requested this monitor rule
    pub requester: NipartEventAddress,
    /// Event ID for tracing the source of this request
    pub uuid: u128,
    /// Interface to monitor
    pub ip: IpAddr,
    // TODO: this is for DHCPv6 notification for link local address change,
    // hence we should `AddressScope::Link` here.
}

impl std::fmt::Display for NipartAddressMonitorRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "address_monitor: uuid:{}, kind:{}, requester:{}, ip:{}",
            self.uuid, self.kind, self.requester, self.ip
        )
    }
}

#[derive(
    Deserialize,
    Serialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Copy,
)]
#[non_exhaustive]
pub enum NipartAddressMonitorKind {
    Remove,
}

impl std::fmt::Display for NipartAddressMonitorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Remove => "remove",
            }
        )
    }
}
