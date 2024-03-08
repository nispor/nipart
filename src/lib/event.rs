// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    MergedNetworkState, NetworkState, NipartApplyOption, NipartDhcpConfig,
    NipartDhcpLease, NipartError, NipartLogLevel, NipartPluginInfo,
    NipartQueryOption, NipartRole,
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
    /// The chosen dhcp plugin
    Dhcp,
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
            Self::Dhcp => write!(f, "dhcp"),
            Self::Group(v) => write!(f, "group:{v}"),
            Self::AllPlugins => write!(f, "all_plugins"),
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
            "{} user:{} plugin:{} src:{} dst:{} timeout:{}ms{}",
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

    QueryPluginInfo,
    QueryPluginInfoReply(NipartPluginInfo),

    ChangeLogLevel(NipartLogLevel),
    QueryLogLevel,
    QueryLogLevelReply(NipartLogLevel),

    QueryNetState(NipartQueryOption),
    QueryRelatedNetState(Box<NetworkState>),
    QueryNetStateReply(Box<NetworkState>, u32),

    ApplyNetState(Box<MergedNetworkState>, NipartApplyOption),
    ApplyNetStateReply,

    /// Empty `Vec<String>` means query all interfaces
    QueryDhcpConfig(Box<Vec<String>>),
    QueryDhcpConfigReply(Box<Vec<NipartDhcpConfig>>),

    ApplyDhcpConfig(Box<Vec<NipartDhcpConfig>>),
    ApplyDhcpConfigReply,

    /// DHCP plugin notify commander on new lease been acquired
    GotDhcpLease(Box<NipartDhcpLease>),
    /// Commander request responsible plugins to apply DHCP lease
    ApplyDhcpLease(Box<NipartDhcpLease>),
    ApplyDhcpLeaseReply,
}

impl std::fmt::Display for NipartPluginEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::None => "none",
                Self::Quit => "quit",
                Self::QueryPluginInfo => "query_plugin_info",
                Self::QueryPluginInfoReply(_) => "query_plugin_info_reply",
                Self::ChangeLogLevel(_) => "change_log_level",
                Self::QueryLogLevel => "query_log_level",
                Self::QueryLogLevelReply(_) => "query_log_level_reply",
                Self::QueryNetState(_) => "query_netstate",
                Self::QueryNetStateReply(_, _) => "query_netstate_reply",
                Self::QueryRelatedNetState(_) => "query_related_netstate",
                Self::ApplyNetState(_, _) => "apply_netstate",
                Self::ApplyNetStateReply => "apply_netstate_reply",
                Self::QueryDhcpConfig(_) => "query_dhcp_config",
                Self::QueryDhcpConfigReply(_) => "query_dhcp_config_reply",
                Self::ApplyDhcpConfig(_) => "apply_dhcp_config",
                Self::ApplyDhcpConfigReply => "apply_dhcp_config_reply",
                Self::GotDhcpLease(_) => "got_dhcp_lease",
                Self::ApplyDhcpLease(_) => "apply_dhcp_lease",
                Self::ApplyDhcpLeaseReply => "apply_dhcp_lease_reply",
            }
        )
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
        )
    }
}
