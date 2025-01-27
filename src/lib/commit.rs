// SPDX-License-Identifier: Apache-2.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    ErrorKind, NetworkState, NipartConnection, NipartError, NipartEvent,
    NipartEventAddress, NipartPluginEvent, NipartUserEvent, NipartUuid,
};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct NetworkCommitQueryOption {
    /// Only query most recent commits with specified count, 0 means all
    pub count: u32,
    /// Include commits with this UUID only, empty Vec means all commits
    /// When not emtpy, the `NetworkCommitQueryOption.count` will be ignored.
    pub uuids: Vec<NipartUuid>,
}

impl std::fmt::Display for NetworkCommitQueryOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "count: {}", self.count)?;
        if self.uuids.is_empty() {
            write!(f, " uuids: any")?;
        } else {
            let uuids: Vec<String> = self
                .uuids
                .as_slice()
                .iter()
                .map(|u| u.to_string())
                .collect();
            write!(f, " uuids: [{}]", uuids.join(","))?;
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NetworkCommit {
    /// Commit UUID
    pub uuid: NipartUuid,
    /// Description notes for this commit
    pub description: String,
    // TODO: Show it as human friendly string, maybe use chrono time.
    /// Time of creation.
    /// Commit ordering should be depend on this time, plugin should use its
    /// own way to preserving the order of commit creation. And API user
    /// should use Vec order in the reply of commit querying.
    pub time: DateTime<Utc>,
    /// Desired NetworkState
    pub desired_state: NetworkState,
    /// The revert state of this commit.
    pub revert_state: NetworkState,
}

impl Default for NetworkCommit {
    fn default() -> Self {
        Self {
            uuid: NipartUuid::VOID,
            time: DateTime::<Utc>::MIN_UTC,
            desired_state: NetworkState::default(),
            description: String::new(),
            revert_state: NetworkState::default(),
        }
    }
}

impl NetworkCommit {
    pub fn new(
        desired_state: NetworkState,
        pre_apply_state: &NetworkState,
    ) -> Self {
        let uuid = NipartUuid::new();
        let description = if desired_state.description.is_empty() {
            String::new()
        } else {
            desired_state.description.clone()
        };
        let revert_state = match desired_state.generate_revert(pre_apply_state)
        {
            Ok(mut s) => {
                s.description = format!("Revert of {}", uuid);
                s
            }
            Err(e) => {
                log::error!(
                    "BUG: NetworkState::generate_revert() fails with {e}"
                );
                NetworkState::default()
            }
        };
        Self {
            uuid: NipartUuid::new(),
            time: Utc::now(),
            description,
            revert_state,
            desired_state,
        }
    }
}

impl NipartConnection {
    /// The last commit is placed at the end of the Vec.
    pub async fn query_commits(
        &mut self,
        option: NetworkCommitQueryOption,
    ) -> Result<Vec<NetworkCommit>, NipartError> {
        if option.count != 0 && !option.uuids.is_empty() {
            log::warn!(
                "NetworkCommitQueryOption.count will be ignored when \
                NetworkCommitQueryOption.uuids is not empty()"
            );
        }
        let request = NipartEvent::new(
            NipartUserEvent::QueryCommits(option),
            NipartPluginEvent::None,
            NipartEventAddress::User,
            NipartEventAddress::Daemon,
            self.timeout,
        );
        self.send(&request).await?;
        let event = self.recv_reply(request.uuid, self.timeout).await?;
        if let NipartUserEvent::QueryCommitsReply(s) = event.user {
            Ok(*s)
        } else {
            Err(NipartError::new(
                ErrorKind::Bug,
                format!("Invalid reply {event:?} for QueryCommits"),
            ))
        }
    }

    pub async fn remove_commits(
        &mut self,
        uuids: &[NipartUuid],
    ) -> Result<NetworkState, NipartError> {
        let request = NipartEvent::new(
            NipartUserEvent::RemoveCommits(Box::new(uuids.to_vec())),
            NipartPluginEvent::None,
            NipartEventAddress::User,
            NipartEventAddress::Daemon,
            self.timeout,
        );
        self.send(&request).await?;
        let event = self.recv_reply(request.uuid, self.timeout).await?;
        if let NipartUserEvent::RemoveCommitsReply(s) = event.user {
            Ok(*s)
        } else {
            Err(NipartError::new(
                ErrorKind::Bug,
                format!("Invalid reply {event:?} for RemoveCommits"),
            ))
        }
    }
}
