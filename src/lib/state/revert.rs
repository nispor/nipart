// SPDX-License-Identifier: Apache-2.0

use crate::{NetworkState, NipartError};

impl NetworkState {
    /// Generate revert state of desired(&self) state
    /// The `pre_apply_state` should be the full running state before applying
    /// specified desired state.
    pub fn generate_revert(
        &self,
        _pre_apply_state: &Self,
    ) -> Result<Self, NipartError> {
        todo!()
    }
}
