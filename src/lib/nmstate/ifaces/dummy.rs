// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{BaseInterface, InterfaceType, JsonDisplay, NipartstateInterface};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonDisplay)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
/// Dummy interface
pub struct DummyInterface {
    #[serde(flatten)]
    pub base: BaseInterface,
}

impl DummyInterface {
    pub fn new(name: String) -> Self {
        Self {
            base: BaseInterface {
                name: name.to_string(),
                iface_type: InterfaceType::Dummy,
                ..Default::default()
            },
        }
    }
}

impl Default for DummyInterface {
    fn default() -> Self {
        Self {
            base: BaseInterface {
                iface_type: InterfaceType::Dummy,
                ..Default::default()
            },
        }
    }
}

impl NipartstateInterface for DummyInterface {
    fn base_iface(&self) -> &BaseInterface {
        &self.base
    }

    fn base_iface_mut(&mut self) -> &mut BaseInterface {
        &mut self.base
    }

    fn is_virtual(&self) -> bool {
        true
    }
}
