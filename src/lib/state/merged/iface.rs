// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{Interface, NipartError, NipartInterface};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MergedInterface {
    pub desired: Option<Interface>,
    pub current: Option<Interface>,
    pub merged: Interface,
    pub for_apply: Option<Interface>,
    pub for_verify: Option<Interface>,
}

impl MergedInterface {
    pub fn new(
        desired: Option<Interface>,
        current: Option<Interface>,
    ) -> Result<Self, NipartError> {
        let merged = match (&desired, &current) {
            (Some(desired), Some(current)) => {
                let mut merged = current.clone();
                merged.merge(desired)?;
                merged
            }
            (Some(state), None) | (None, Some(state)) => state.clone(),
            _ => {
                log::warn!(
                    "BUG: MergedInterface:new() got both desired \
                    and current set to None"
                );
                Interface::default()
            }
        };
        Ok(Self {
            for_apply: desired.clone(),
            for_verify: desired.clone(),
            desired,
            current,
            merged,
        })
    }

    pub(crate) fn is_desired(&self) -> bool {
        self.desired.is_some()
    }

    pub(crate) fn is_changed(&self) -> bool {
        self.for_apply.is_some()
    }
}
