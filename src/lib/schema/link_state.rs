// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::JsonDisplay;

#[derive(
    Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonDisplay,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
/// Interface link carrier state
pub enum InterfaceLinkState {
    /// Up, ready to send/receive packets
    Up,
    /// Not up but pending an external event
    Dormant,
    /// Down
    Down,
    /// Down due to state of lower layer(OSI hardware layer)
    LowerLayerDown,
    /// Test mode
    Testing,
    #[default]
    Unknown,
}
