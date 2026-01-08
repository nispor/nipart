// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{CUR_SCHEMA_VERSION, JsonDisplay};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonDisplay)]
#[non_exhaustive]
pub struct NipartstateQueryOption {
    /// Schema version for output
    #[serde(default)]
    pub version: u32,
    /// Which kind of NetworkState to query, default:
    /// [NipartstateStateKind::RunningNetworkState]
    #[serde(default)]
    pub kind: NipartstateStateKind,
    /// Whether include secrets/passwords, default to false.
    #[serde(default)]
    pub include_secrets: bool,
}

impl Default for NipartstateQueryOption {
    fn default() -> Self {
        Self {
            version: CUR_SCHEMA_VERSION,
            kind: NipartstateStateKind::default(),
            include_secrets: false,
        }
    }
}

impl NipartstateQueryOption {
    pub fn running() -> Self {
        Self {
            kind: NipartstateStateKind::RunningNetworkState,
            ..Default::default()
        }
    }

    pub fn saved() -> Self {
        Self {
            kind: NipartstateStateKind::SavedNetworkState,
            ..Default::default()
        }
    }

    pub fn include_secrets(mut self, value: bool) -> Self {
        self.include_secrets = value;
        self
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, JsonDisplay,
)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum NipartstateStateKind {
    /// The current running network state
    #[default]
    RunningNetworkState,
    /// Network state stored in daemon
    SavedNetworkState,
}

#[derive(
    Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonDisplay,
)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub struct NipartstateApplyOption {
    /// Do not verify whether post applied state matches with desired state.
    #[serde(default)]
    pub no_verify: bool,
    /// When set to true, the desire state will not be persistent after OS
    /// reboot. Default to false.
    #[serde(default)]
    pub memory_only: bool,
    /// Whether to invoke DHCP in no-daemon mode. Default to false.
    /// This option makes no effect in daemon mode(via NipartClient).
    #[serde(default)]
    pub dhcp_in_no_daemon: bool,
}

impl NipartstateApplyOption {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn no_verify(mut self) -> Self {
        self.no_verify = true;
        self
    }

    pub fn memory_only(mut self) -> Self {
        self.memory_only = true;
        self
    }

    pub fn dhcp_in_no_daemon(mut self) -> Self {
        self.memory_only = true;
        self
    }
}
