// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{NipartLogLevel, NipartPluginInfo, NipartRole};

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
    pub ref_uuid: Option<u128>,
    pub action: NipartEventAction,
    pub user: NipartUserEvent,
    pub plugin: NipartPluginEvent,
    pub src: NipartEventAddress,
    pub dst: NipartEventAddress,
}

impl NipartEvent {
    /// Generate a NipartEvent
    pub fn new(
        action: NipartEventAction,
        user: NipartUserEvent,
        plugin: NipartPluginEvent,
        src: NipartEventAddress,
        dst: NipartEventAddress,
    ) -> Self {
        Self {
            uuid: uuid::Uuid::now_v7().as_u128(),
            ref_uuid: None,
            action,
            user,
            plugin,
            src,
            dst,
        }
    }

    pub(crate) fn is_done(&self) -> bool {
        self.action == NipartEventAction::Done
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
    QueryPluginInfo,
    QueryPluginInfoReply(Vec<NipartPluginInfo>),

    ChangeLogLevel(NipartLogLevel),
    QueryLogLevel,
    QueryLogLevelReply(HashMap<String, NipartLogLevel>),
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum NipartPluginEvent {
    #[default]
    None,
    Quit,
    CommanderRefreshPlugins(usize),
    UpdateAllPluginInfo(Vec<NipartPluginInfo>),
    QueryPluginInfo,
    QueryPluginInfoReply(NipartPluginInfo),
    ChangeLogLevel(NipartLogLevel),
    QueryLogLevel,
    QueryLogLevelReply(NipartLogLevel),
}
