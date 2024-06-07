// SPDX-License-Identifier: Apache-2.0

use crate::OvnConfiguration;

impl OvnConfiguration {
    pub fn is_empty(&self) -> bool {
        if let Some(maps) = self.bridge_mappings.as_ref() {
            maps.is_empty()
        } else {
            true
        }
    }
}
