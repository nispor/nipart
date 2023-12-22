// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct NipartNetState {}

impl NipartNetState {
    pub fn merge_states(_states: Vec<(NipartNetState, u32)>) -> Self {
        Self::default()
    }
}
