// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use crate::{NipartPluginInfo, NipartQueryStateOption, NipartRole};

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
    /// All plugins except commander
    AllPluginNoCommander,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NipartEvent {
    pub uuid: u128,
    pub ref_uuid: Option<u128>,
    pub action: NipartEventAction,
    pub data: NipartEventData,
    pub src: NipartEventAddress,
    /// None means broadcast to all plugins except commander
    pub dst: NipartEventAddress,
}

impl NipartEvent {
    /// Generate a NipartEvent
    pub fn new(
        action: NipartEventAction,
        data: NipartEventData,
        src: NipartEventAddress,
        dst: NipartEventAddress,
    ) -> Self {
        Self {
            uuid: uuid::Uuid::now_v7().as_u128(),
            ref_uuid: None,
            action,
            data,
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
/// All event name with `User` prefix is reserved for nipart internal usage.
pub enum NipartEventData {
    UserQueryPluginInfo,
    UserQueryPluginInfoReply(Vec<NipartPluginInfo>),
    UserQueryNetState(NipartQueryStateOption),

    UpdateAllPluginInfo(Vec<NipartPluginInfo>),
    QueryPluginInfo,
    QueryPluginInfoReply(NipartPluginInfo),

    PluginQuit,
}
