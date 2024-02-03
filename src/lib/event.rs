// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    MergedNetworkState, NetworkState, NipartApplyOption, NipartError,
    NipartLogLevel, NipartPluginInfo, NipartQueryOption, NipartRole,
};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
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
    /// Group of plugins holding specified [NipartRole]
    Group(NipartRole),
    /// All plugins
    AllPlugins,
}

impl std::fmt::Display for NipartEventAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Unicast(v) => write!(f, "{v}"),
            Self::Daemon => write!(f, "daemon"),
            Self::Commander => write!(f, "commander"),
            Self::Group(v) => write!(f, "group:{v}"),
            Self::AllPlugins => write!(f, "all_plugins"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NipartEvent {
    pub uuid: u128,
    pub action: NipartEventAction,
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
            "{} user {} plugin {} src {} dst timeout {}ms {}{}",
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
        action: NipartEventAction,
        user: NipartUserEvent,
        plugin: NipartPluginEvent,
        src: NipartEventAddress,
        dst: NipartEventAddress,
        timeout: u32,
    ) -> Self {
        Self {
            uuid: uuid::Uuid::now_v7().as_u128(),
            action,
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
            NipartEventAction::Done,
            NipartUserEvent::Error(e),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
            crate::DEFAULT_TIMEOUT,
        )
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NipartEventAction {
    OneShot,
    Request,
    Done,
    Cancle,
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
    // TODO: Return applied state and revert state
    ApplyNetStateReply,
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
    CommanderRefreshPlugins(usize),

    QueryPluginInfo,
    QueryPluginInfoReply(NipartPluginInfo),

    ChangeLogLevel(NipartLogLevel),
    QueryLogLevel,
    QueryLogLevelReply(NipartLogLevel),

    QueryNetState(NipartQueryOption),
    QueryNetStateReply(Box<NetworkState>, u32),

    QueryRelatedNetState(Box<NetworkState>),
    QueryRelatedNetStateReply(Box<NetworkState>, u32),

    ApplyNetState(Box<MergedNetworkState>, NipartApplyOption),
    ApplyNetStateReply,
}

impl std::fmt::Display for NipartPluginEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::None => "none",
                Self::Quit => "quit",
                Self::CommanderRefreshPlugins(_) => "commander_refresh_plugins",
                Self::QueryPluginInfo => "query_plugin_info",
                Self::QueryPluginInfoReply(_) => "query_plugin_info_reply",
                Self::ChangeLogLevel(_) => "change_log_level",
                Self::QueryLogLevel => "query_log_level",
                Self::QueryLogLevelReply(_) => "query_log_level_reply",
                Self::QueryNetState(_) => "query_netstate",
                Self::QueryNetStateReply(_, _) => "query_netstate_reply",
                Self::QueryRelatedNetState(_) => "query_related_netstate",
                Self::QueryRelatedNetStateReply(_, _) =>
                    "query_related_netstate_reply",
                Self::ApplyNetState(_, _) => "apply_netstate",
                Self::ApplyNetStateReply => "apply_netstate_reply",
            }
        )
    }
}
