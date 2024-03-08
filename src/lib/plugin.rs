// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NipartPluginInfo {
    pub name: String,
    pub roles: Vec<NipartRole>,
}

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
#[non_exhaustive]
pub enum NipartRole {
    Daemon,
    Commander,
    Dhcp,
    QueryAndApply,
    ApplyDhcpLease,
    Ovs,
    Lldp,
    Monitor,
    Config,
}

impl std::fmt::Display for NipartRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Daemon => "daemon",
                Self::Commander => "commander",
                Self::Dhcp => "dhcp",
                Self::QueryAndApply => "query_and_apply",
                Self::Ovs => "ovs",
                Self::Lldp => "lldp",
                Self::Monitor => "monitor",
                Self::Config => "config",
                Self::ApplyDhcpLease => "apply_dhcp_lease",
            }
        )
    }
}
