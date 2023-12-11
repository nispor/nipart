// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use crate::NipartPluginInfo;

/// Events should be supported by all plugins.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum NipartEventCommander {
    UpdateAllPluginInfo(Vec<NipartPluginInfo>),
}
