// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{BaseInterface, InterfaceType, NipartInterface};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
/// Ethernet(IEEE 802.3) interface.
pub struct EthernetInterface {
    #[serde(flatten)]
    pub base: BaseInterface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ethernet: Option<EthernetConfig>,
}

impl Default for EthernetInterface {
    fn default() -> Self {
        Self {
            base: BaseInterface {
                iface_type: InterfaceType::Ethernet,
                ..Default::default()
            },
            ethernet: None,
        }
    }
}

impl NipartInterface for EthernetInterface {
    fn base_iface(&self) -> &BaseInterface {
        &self.base
    }

    fn base_iface_mut(&mut self) -> &mut BaseInterface {
        &mut self.base
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct EthernetConfig {}
