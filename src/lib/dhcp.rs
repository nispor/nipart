// SPDX-License-Identifier: Apache-2.0

use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};

const DEFAULT_DHCP_TIMEOUT: u32 = 30;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NipartDhcpConfig {
    V4(NipartDhcpConfigV4),
    V6(NipartDhcpConfigV6),
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NipartDhcpConfigV6 {
    pub iface: String,
    pub enabled: bool,
    pub timeout: u32,
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NipartDhcpConfigV4 {
    pub iface: String,
    pub client_id: Option<String>,
    pub enabled: bool,
    pub timeout: u32,
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NipartDhcpLeaseV4 {
    pub ip_addr: Ipv4Addr,
    pub dhcp_server_ip_addr: Ipv4Addr,
    pub lease_time: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NipartDhcpState {
    V4(NipartDhcpStateV4),
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct NipartDhcpStateV4 {
    pub status: NipartDhcpStatus,
    pub config: NipartDhcpConfigV4,
    pub lease: Option<NipartDhcpLeaseV4>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NipartDhcpStatus {
    Disabled,
    Requesting,
    Done,
    Timeout,
}

impl Default for NipartDhcpStatus {
    fn default() -> Self {
        Self::Disabled
    }
}
