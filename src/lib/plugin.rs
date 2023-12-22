// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NipartPluginInfo {
    pub name: String,
    pub roles: Vec<NipartRole>,
    pub socket_path: String,
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
    Kernel,
    Ovs,
    Lldp,
    Monitor,
    State,
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
                Self::Kernel => "kernel",
                Self::Ovs => "ovs",
                Self::Lldp => "lldp",
                Self::Monitor => "monitor",
                Self::State => "state",
                Self::Config => "config",
            }
        )
    }
}
