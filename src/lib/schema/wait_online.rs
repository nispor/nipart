// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{JsonDisplay, NetworkState};

const GATEWAY4: &str = "0.0.0.0/0";
const GATEWAY6: &str = "::/0";

/// Daemon wait online configuration
///
/// Configuration instructing when daemon should consider the network is
/// online on boot.
/// Once daemon reaches online state, it stop tracking whether online
/// conditions still met. This is purely designed for systemd
/// network-online.target.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonDisplay)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct NipartWaitOnline {
    /// Maximum wait time in seconds to wait network state to be online.
    /// Default is 30 seconds. Setting to 0 means mark as online once daemon
    /// starts.
    #[serde(default = "default_tmo")]
    pub timeout_sec: u32,
    /// The network is considered as online when all of these conditions met.
    /// If undefined, daemon wait all saved configuration been applied.
    /// If set to empty list explicitly, daemon will mark online once started.
    #[serde(default = "default_conditions")]
    pub conditions: Vec<NipartWaitOnlineCondition>,
}

fn default_tmo() -> u32 {
    NipartWaitOnline::DEFAULT_TIMEOUT_SEC
}

fn default_conditions() -> Vec<NipartWaitOnlineCondition> {
    vec![NipartWaitOnlineCondition::default()]
}

impl NipartWaitOnline {
    pub const DEFAULT_TIMEOUT_SEC: u32 = 30;
}

impl Default for NipartWaitOnline {
    fn default() -> Self {
        Self {
            timeout_sec: default_tmo(),
            conditions: default_conditions(),
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize, JsonDisplay,
)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum NipartWaitOnlineCondition {
    /// IPv4 or IPv6 gateway added.
    #[default]
    Gateway,
    /// IPv4 gateway added. (TODO: reachable via ARP?)
    Gateway4,
    /// IPv6 gateway added. (TODO: reachable via Neighbor Discovery?)
    Gateway6,
}

impl NipartWaitOnlineCondition {
    pub fn is_met(&self, cur_state: &NetworkState) -> bool {
        match self {
            Self::Gateway => {
                cur_state.routes.running.as_ref().map(|rts| {
                    rts.iter().any(|rt| {
                        rt.destination.as_deref() == Some(GATEWAY4)
                            || rt.destination.as_deref() == Some(GATEWAY6)
                    })
                }) == Some(true)
            }
            Self::Gateway4 => {
                cur_state.routes.running.as_ref().map(|rts| {
                    rts.iter()
                        .any(|rt| rt.destination.as_deref() == Some(GATEWAY4))
                }) == Some(true)
            }
            Self::Gateway6 => {
                cur_state.routes.running.as_ref().map(|rts| {
                    rts.iter()
                        .any(|rt| rt.destination.as_deref() == Some(GATEWAY6))
                }) == Some(true)
            }
        }
    }
}
