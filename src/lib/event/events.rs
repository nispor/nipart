// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use crate::{NipartUserEvent, NipartPluginCommonEvent};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NipartEvent {
    pub uuid: u128,
    pub ref_uuid: Option<u128>,
    pub action: NipartEventAction,
    pub data: NipartEventData,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum NipartEventAction {
    Request,
    Done,
    Cancle,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum NipartEventData {
    User(NipartUserEvent),
    PluginCommon(NipartPluginCommonEvent),
}
