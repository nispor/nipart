// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NipartQueryOption {
    /// Which kind of NetworkState to query
    #[serde(default)]
    pub kind: NipartStateKind,
}

impl NipartQueryOption {
    pub fn running() -> Self {
        Self {
            kind: NipartStateKind::RunningNetworkState,
        }
    }

    pub fn saved() -> Self {
        Self {
            kind: NipartStateKind::SavedNetworkState,
        }
    }

    pub fn post_last_commit() -> Self {
        Self {
            kind: NipartStateKind::PostLastCommitNetworkState,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NipartStateKind {
    /// The current running network state
    #[default]
    RunningNetworkState,
    /// Network state stored in commits
    SavedNetworkState,
    /// The running network state after last commit
    PostLastCommitNetworkState,
}

impl std::fmt::Display for NipartStateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::RunningNetworkState => "running_network_state",
                Self::SavedNetworkState => "saved_network_state",
                Self::PostLastCommitNetworkState =>
                    "post_last_commit_network_state",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NipartApplyOption {
    // Seconds to rollback desired state after applied.
    //pub revert_after: u32,
    /// Do not store desired state to persistent
    pub memory_only: bool,
    /// Do not verify whether post applied state matches with desired state.
    pub no_verify: bool,
    /// Indicate desire state is generate by diff between running state and
    /// saved state, when creating commit, the `pre_apply_state` should
    /// be post state after last commit
    pub is_diff: bool,
}
