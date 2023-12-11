// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use crate::NipartQueryStateOption;

/// Events should be supported by all plugins.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum NipartPluginCommonEvent {
    QueryPluginInfo,
    Quit,
}
