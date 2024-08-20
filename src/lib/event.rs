// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    NetworkCommit, NetworkCommitQueryOption, NetworkState, NipartApplyOption,
    NipartError, NipartLogLevel, NipartPluginEvent, NipartPluginInfo,
    NipartQueryOption, NipartRole,
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
            Self::User => write!(f, "event_address.user"),
            Self::Unicast(v) => write!(f, "event_address.{v}"),
            Self::Daemon => write!(f, "event_address.daemon"),
            Self::Commander => write!(f, "event_address.commander"),
            Self::Dhcp => write!(f, "event_address.dhcp"),
            Self::Track => write!(f, "event_address.track"),
            Self::Group(v) => write!(f, "event_address.group:{v}"),
            Self::AllPlugins => write!(f, "event_address.all_plugins"),
            Self::Locker => write!(f, "event_address.locker"),
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
            "event.uuid:{} event.user:{} event.plugin:{} \
            event.src:{} event.dst:{} event.timeout:{}ms{}",
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
}

impl std::fmt::Display for NipartUserEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::None => "user_event.none",
                Self::Quit => "user_event.quit",
                Self::Error(_) => "user_event.error",
                Self::QueryPluginInfo => "user_event.query_plugin_info",
                Self::QueryPluginInfoReply(_) =>
                    "user_event.query_plugin_info_reply",
                Self::ChangeLogLevel(_) => "user_event.change_log_level",
                Self::QueryLogLevel => "user_event.query_log_level",
                Self::QueryLogLevelReply(_) =>
                    "user_event.query_log_level_reply",
                Self::QueryNetState(_) => "user_event.query_netstate",
                Self::QueryNetStateReply(_) =>
                    "user_event.query_netstate_reply",
                Self::ApplyNetState(_, _) => "user_event.apply_netstate",
                Self::ApplyNetStateReply => "user_event.apply_netstate_reply",
                Self::QueryCommits(_) => "user_event.query_commits",
                Self::QueryCommitsReply(_) => "user_event.query_commits_reply",
            }
        )
    }
}
