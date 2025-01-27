// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{
    MergedNetworkState, NetworkCommit, NetworkCommitQueryOption, NetworkState,
    NipartApplyOption, NipartDhcpConfig, NipartDhcpLease, NipartLockEntry,
    NipartLockOption, NipartLogLevel, NipartMonitorEvent, NipartMonitorRule,
    NipartQueryOption, NipartUuid,
};

/// Data for plugin to do initialize task after daemon fully started
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct NipartPostStartData {
    pub current_state: NetworkState,
}

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
    Dhcp,
    QueryAndApply,
    ApplyDhcpLease,
    Ovs,
    Lldp,
    Monitor,
    Commit,
    Locker,
    Logger,
}

impl std::fmt::Display for NipartRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Dhcp => "dhcp",
                Self::QueryAndApply => "query_and_apply",
                Self::Ovs => "ovs",
                Self::Lldp => "lldp",
                Self::Monitor => "monitor",
                Self::Commit => "commit",
                Self::ApplyDhcpLease => "apply_dhcp_lease",
                Self::Locker => "locker",
                Self::Logger => "logger",
            }
        )
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum NipartPluginEvent {
    #[default]
    None,
    Quit,

    QueryPluginInfo,
    QueryPluginInfoReply(NipartPluginInfo),

    ChangeLogLevel(NipartLogLevel),
    QueryLogLevel,
    QueryLogLevelReply(NipartLogLevel),

    /// Query running network state.
    QueryNetState(NipartQueryOption),
    /// Query running network state related to specified network state.
    QueryRelatedNetState(Box<NetworkState>),
    /// Reply with running network state related to specified network state.
    QueryNetStateReply(Box<NetworkState>, u32),

    ApplyNetState(Box<MergedNetworkState>, NipartApplyOption),
    ApplyNetStateReply,

    /// Empty `Vec<String>` means query all interfaces.
    QueryDhcpConfig(Box<Vec<String>>),
    QueryDhcpConfigReply(Box<Vec<NipartDhcpConfig>>),

    ApplyDhcpConfig(Box<Vec<NipartDhcpConfig>>),
    ApplyDhcpConfigReply,

    /// DHCP plugin notify commander on new lease been acquired.
    GotDhcpLease(Box<NipartDhcpLease>),
    /// Commander request responsible plugins to apply DHCP lease.
    ApplyDhcpLease(Box<NipartDhcpLease>),
    ApplyDhcpLeaseReply,

    /// Register a monitor rule to plugin with monitor role.
    /// No reply required.
    RegisterMonitorRule(Box<NipartMonitorRule>),
    /// Remove a monitor rule from monitor plugin.
    /// No reply required.
    RemoveMonitorRule(Box<NipartMonitorRule>),
    /// Monitor plugin notify. No reply required.
    GotMonitorEvent(Box<NipartMonitorEvent>),

    /// Indicate daemon and all plugins are started. Plugin could
    /// use this event to do initialization required for other plugins' help.
    /// No reply required.
    PostStart(Box<NipartPostStartData>),
    /// Store commit.
    /// Because we might have multiple plugins with [NipartRole::Commit],
    /// when creating commit, event sender should prepare all required
    /// properties in [NetworkCommit] and expecting commit plugins reply
    /// identical data on follow query requests.
    /// The NetworkState here is the post commit running network state.
    /// Plugin should reply with [NipartPluginEvent::CreateCommitReply]
    CreateCommit(Box<(NetworkCommit, NetworkState)>),
    /// Ack on commit finished.
    CreateCommitReply,
    /// Remove specified commits. Should only be requested after state in these
    /// commits are reverted. The second argument is the running network state
    /// after specified commits been reverted.
    RemoveCommits(Box<(Vec<NipartUuid>, NetworkState)>),
    /// Reply with new merged saved network state from remaining commits
    RemoveCommitsReply(Box<NetworkState>),
    QueryCommits(NetworkCommitQueryOption),
    QueryCommitsReply(Box<Vec<NetworkCommit>>),
    QueryLastCommitState,
    /// Query the running network state when last commit applied
    QueryLastCommitStateReply(Box<NetworkState>),

    /// Request lock on specified entries, reply required.
    Lock(Box<Vec<(NipartLockEntry, NipartLockOption)>>),
    /// Request unlock on specified entries, no reply required.
    /// Cannot unlock other event's entry.
    Unlock(Box<Vec<NipartLockEntry>>),

    // TBD: do we need to indicate who is currently taking lock when fails
    /// Indicate all requested lock entries has been locked as requested.
    LockReply,
    UnlockReply,
}

impl std::fmt::Display for NipartPluginEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Quit => write!(f, "quit"),
            Self::QueryPluginInfo => write!(f, "query_plugin_info"),
            Self::QueryPluginInfoReply(_) => {
                write!(f, "query_plugin_info_reply")
            }
            Self::ChangeLogLevel(l) => write!(f, "change_log_level:{l}"),
            Self::QueryLogLevel => write!(f, "query_log_level"),
            Self::QueryLogLevelReply(_) => {
                write!(f, "query_log_level_reply")
            }
            Self::QueryNetState(_) => write!(f, "query_netstate"),
            Self::QueryNetStateReply(_, _) => {
                write!(f, "query_netstate_reply")
            }
            Self::QueryRelatedNetState(_) => {
                write!(f, "query_related_netstate")
            }
            Self::ApplyNetState(_, _) => write!(f, "apply_netstate"),
            Self::ApplyNetStateReply => write!(f, "apply_netstate_reply"),
            Self::QueryDhcpConfig(_) => write!(f, "query_dhcp_config"),
            Self::QueryDhcpConfigReply(_) => {
                write!(f, "query_dhcp_config_reply")
            }
            Self::ApplyDhcpConfig(configs) => write!(
                f,
                "apply_dhcp_config:{}",
                configs
                    .as_slice()
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            Self::ApplyDhcpConfigReply => {
                write!(f, "apply_dhcp_config_reply")
            }
            Self::GotDhcpLease(_) => write!(f, "got_dhcp_lease"),
            Self::ApplyDhcpLease(_) => write!(f, "apply_dhcp_lease"),
            Self::ApplyDhcpLeaseReply => {
                write!(f, "apply_dhcp_lease_reply")
            }
            Self::RegisterMonitorRule(rule) => {
                write!(f, "register_monitor_rule:{rule}")
            }
            Self::RemoveMonitorRule(rule) => {
                write!(f, "remove_monitor_rule:{rule}")
            }
            Self::GotMonitorEvent(event) => {
                write!(f, "got_monitor_event:{event}")
            }
            Self::QueryCommits(_) => write!(f, "query_commits"),
            Self::QueryCommitsReply(_) => write!(f, "query_commits_reply"),
            Self::PostStart(_) => write!(f, "post_start"),
            Self::CreateCommit(_) => write!(f, "create_commit"),
            Self::CreateCommitReply => write!(f, "create_commit_reply"),
            Self::RemoveCommits(_) => write!(f, "remove_commits"),
            Self::RemoveCommitsReply(_) => write!(f, "remove_commits_reply"),

            Self::Lock(_) => write!(f, "lock"),
            Self::Unlock(_) => write!(f, "unlock"),
            Self::LockReply => write!(f, "lock_reply"),
            Self::UnlockReply => write!(f, "unlock_reply"),
            Self::QueryLastCommitState => write!(f, "query_last_commit_state"),
            Self::QueryLastCommitStateReply(_) => {
                write!(f, "query_last_commit_state_reply")
            }
        }
    }
}

impl NipartPluginEvent {
    pub fn is_reply(&self) -> bool {
        matches!(
            self,
            Self::QueryPluginInfoReply(_)
                | Self::QueryLogLevelReply(_)
                | Self::QueryNetStateReply(_, _)
                | Self::ApplyNetStateReply
                | Self::QueryDhcpConfigReply(_)
                | Self::ApplyDhcpConfigReply
                | Self::ApplyDhcpLeaseReply
                | Self::GotMonitorEvent(_)
                | Self::QueryCommitsReply(_)
                | Self::CreateCommitReply
                | Self::LockReply
                | Self::UnlockReply
                | Self::QueryLastCommitStateReply(_)
                | Self::RemoveCommitsReply(_)
        )
    }
}
