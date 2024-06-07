// SPDX-License-Identifier: Apache-2.0

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::NetworkState;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct NetworkCommitQueryOption {
    /// Count of commits to query. 0 means all.
    pub count: u32,

    /// When set to true, only return with persisted [NetworkCommit]. Default
    /// is false.
    pub persisted_only: bool,
}

impl std::fmt::Display for NetworkCommitQueryOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "count:{}", self.count)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NetworkCommit {
    /// Whether this commit is persistently stored
    pub persisted: bool,
    /// Commit ID
    pub id: String,
    /// Time of creation
    pub time: SystemTime,
    /// NetworkState it holds
    pub state: NetworkState,
}

impl Default for NetworkCommit {
    fn default() -> Self {
        Self {
            persisted: false,
            id: String::new(),
            time: SystemTime::UNIX_EPOCH,
            state: NetworkState::default(),
        }
    }
}
