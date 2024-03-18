// SPDX-License-Identifier: Apache-2.0

use std::net::{Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};

const DEFAULT_DHCP_TIMEOUT: u32 = 30;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum NipartDhcpConfig {
    V4(NipartDhcpConfigV4),
    V6(NipartDhcpConfigV6),
}

impl std::fmt::Display for NipartDhcpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V4(c) => write!(f, "dhcpv4:{c}"),
            Self::V6(c) => write!(f, "dhcpv6:{c}"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NipartDhcpConfigV6 {
    pub iface: String,
    pub enabled: bool,
    pub timeout: u32,
}

impl std::fmt::Display for NipartDhcpConfigV6 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}",
            self.iface,
            if self.enabled { "enabled" } else { "disable" }
        )
    }
}

impl Default for NipartDhcpConfigV6 {
    fn default() -> Self {
        Self {
            iface: "".into(),
            enabled: false,
            timeout: DEFAULT_DHCP_TIMEOUT,
        }
    }
}

impl NipartDhcpConfigV6 {
    pub fn new(iface: String, enabled: bool) -> Self {
        Self {
            iface,
            enabled,
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NipartDhcpConfigV4 {
    pub iface: String,
    pub client_id: Option<String>,
    pub enabled: bool,
    pub timeout: u32,
}

impl NipartDhcpConfigV4 {
    pub fn new(iface: String, enabled: bool) -> Self {
        Self {
            iface,
            enabled,
            ..Default::default()
        }
    }
}

impl Default for NipartDhcpConfigV4 {
    fn default() -> Self {
        Self {
            iface: "".into(),
            client_id: None,
            enabled: false,
            timeout: DEFAULT_DHCP_TIMEOUT,
        }
    }
}

impl std::fmt::Display for NipartDhcpConfigV4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}",
            self.iface,
            if self.enabled { "enabled" } else { "disable" },
        )
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum NipartDhcpLease {
    V4(NipartDhcpLeaseV4),
    V6(NipartDhcpLeaseV6),
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NipartDhcpLeaseV4 {
    pub iface: String,
    pub ip: Ipv4Addr,
    pub prefix_length: u8,
    pub server_ip: Ipv4Addr,
    pub lease_time: u32,
}

impl NipartDhcpLeaseV4 {
    pub fn new(
        iface: String,
        ip: Ipv4Addr,
        prefix_length: u8,
        server_ip: Ipv4Addr,
        lease_time: u32,
    ) -> Self {
        Self {
            iface,
            ip,
            prefix_length,
            server_ip,
            lease_time,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NipartDhcpLeaseV6 {
    pub iface: String,
    pub ip: Ipv6Addr,
    pub prefix_length: u8,
    pub server_ip: Ipv4Addr,
    pub lease_time: u32,
}
