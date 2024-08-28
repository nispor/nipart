// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    NetworkCommit, NetworkCommitQueryOption, NetworkState, NipartApplyOption,
    NipartError, NipartLogEntry, NipartLogLevel, NipartPluginEvent,
    NipartPluginInfo, NipartQueryOption, NipartRole,
};

#[derive(
    Deserialize, Serialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[non_exhaustive]
pub enum NipartEventAddress {
    /// API user
    User,
    /// Plugin name
    Unicast(String),
    /// Daemon
    Daemon,
    /// Commander,
    Commander,
    /// The chosen dhcp plugin
    Dhcp,
    /// The chosen track plugin
    Track,
    /// Group of plugins holding specified [NipartRole]
    Group(NipartRole),
    /// All plugins
    AllPlugins,
    /// The chosen locker plugin
    Locker,
}

impl std::fmt::Display for NipartEventAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Unicast(v) => write!(f, "{v}"),
            Self::Daemon => write!(f, "daemon"),
            Self::Commander => write!(f, "commander"),
            Self::Dhcp => write!(f, "dhcp"),
            Self::Track => write!(f, "track"),
            Self::Group(v) => write!(f, "group/{v}"),
            Self::AllPlugins => write!(f, "all_plugins"),
            Self::Locker => write!(f, "locker"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NipartEvent {
    pub uuid: u128,
    pub user: NipartUserEvent,
    pub plugin: NipartPluginEvent,
    pub src: NipartEventAddress,
    pub dst: NipartEventAddress,
    /// Timeout in milliseconds
    pub timeout: u32,
    /// When Daemon received event with non-zero `postpone_millis`,
    /// it will postponed the process of this event. Often used for retry.
    pub postpone_millis: u32,
}

impl std::fmt::Display for NipartEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "uuid:{} user:{} plugin:{} \
            src:{} dst:{} timeout:{}ms{}",
            self.uuid,
            self.user,
            self.plugin,
            self.src,
            self.dst,
            self.timeout,
            if self.postpone_millis > 0 {
                format!(" postpone {}ms", self.postpone_millis)
            } else {
                String::new()
            }
        )
    }
}

impl NipartEvent {
    /// Generate a NipartEvent
    pub fn new(
        user: NipartUserEvent,
        plugin: NipartPluginEvent,
        src: NipartEventAddress,
        dst: NipartEventAddress,
        timeout: u32,
    ) -> Self {
        Self {
            uuid: uuid::Uuid::now_v7().as_u128(),
            user,
            plugin,
            src,
            dst,
            timeout,
            postpone_millis: 0,
        }
    }

    pub fn new_with_uuid(
        uuid: u128,
        user: NipartUserEvent,
        plugin: NipartPluginEvent,
        src: NipartEventAddress,
        dst: NipartEventAddress,
        timeout: u32,
    ) -> Self {
        Self {
            uuid,
            user,
            plugin,
            src,
            dst,
            timeout,
            postpone_millis: 0,
        }
    }

    pub fn is_err(&self) -> bool {
        matches!(self.user, NipartUserEvent::Error(_))
    }

    pub fn into_result(self) -> Result<NipartEvent, NipartError> {
        if let NipartUserEvent::Error(e) = self.user {
            Err(e)
        } else {
            Ok(self)
        }
    }

    pub fn is_log(&self) -> bool {
        matches!(self.user, NipartUserEvent::Log(_))
    }

    pub fn emit_log(&self) {
        if let NipartUserEvent::Log(log_entry) = &self.user {
            let log_source = format!("nipart.{}", self.src);
            log_entry.emit_log(log_source.as_str())
        }
    }
}

impl From<NipartError> for NipartEvent {
    fn from(e: NipartError) -> Self {
        Self::new(
            NipartUserEvent::Error(e),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
            crate::DEFAULT_TIMEOUT,
        )
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum NipartUserEvent {
    #[default]
    None,
    Quit,
    Error(NipartError),

    QueryPluginInfo,
    QueryPluginInfoReply(Vec<NipartPluginInfo>),

    ChangeLogLevel(NipartLogLevel),
    QueryLogLevel,
    QueryLogLevelReply(HashMap<String, NipartLogLevel>),

    QueryNetState(NipartQueryOption),
    QueryNetStateReply(Box<NetworkState>),

    ApplyNetState(Box<NetworkState>, NipartApplyOption),
    ApplyNetStateReply,

    QueryCommits(NetworkCommitQueryOption),
    QueryCommitsReply(Box<Vec<NetworkCommit>>),

    /// Plugin or daemon logs to user
    Log(NipartLogEntry),
}

impl std::fmt::Display for NipartUserEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::None => "none",
                Self::Quit => "quit",
                Self::Error(_) => "error",
                Self::QueryPluginInfo => "query_plugin_info",
                Self::QueryPluginInfoReply(_) => "query_plugin_info_reply",
                Self::ChangeLogLevel(_) => "change_log_level",
                Self::QueryLogLevel => "query_log_level",
                Self::QueryLogLevelReply(_) => "query_log_level_reply",
                Self::QueryNetState(_) => "query_netstate",
                Self::QueryNetStateReply(_) => "query_netstate_reply",
                Self::ApplyNetState(_, _) => "apply_netstate",
                Self::ApplyNetStateReply => "apply_netstate_reply",
                Self::QueryCommits(_) => "query_commits",
                Self::QueryCommitsReply(_) => "query_commits_reply",
                Self::Log(_) => "log",
            }
        )
    }
}
