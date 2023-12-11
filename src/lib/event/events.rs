// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use crate::{
    NipartEventCommander, NipartPluginCommonEvent, NipartRole, NipartUserEvent,
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
    /// Group of plugins holding specified [NipartRole]
    Group(NipartRole),
    /// All plugins except commander
    AllPluginNoCommander,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NipartEvent {
    #[serde(skip_deserializing)]
    pub uuid: u128,
    #[serde(skip_deserializing)]
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
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum NipartEventAction {
    OneShot,
    Request,
    Done,
    Cancle,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum NipartEventData {
    User(NipartUserEvent),
    PluginCommon(NipartPluginCommonEvent),
    Commander(NipartEventCommander),
}
