// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{InterfaceType, NipartCanIpc};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct NipartPluginInfo {
    pub name: String,
    pub version: String,
    pub iface_types: Vec<InterfaceType>,
}

impl NipartPluginInfo {
    pub fn new(
        name: String,
        version: String,
        iface_types: Vec<InterfaceType>,
    ) -> Self {
        Self {
            name,
            version,
            iface_types,
        }
    }
}

impl NipartCanIpc for NipartPluginInfo {
    fn ipc_kind(&self) -> String {
        "nm-plugin-info".to_string()
    }
}
