// SPDX-License-Identifier: Apache-2.0

use crate::{MergedNetworkState, NetworkState, NipartError};

impl NetworkState {
    /// Generate revert state of desired(&self) state
    /// The `pre_apply_state` should be the full running state before applying
    /// specified desired state.
    pub fn generate_revert(
        &self,
        pre_apply_state: &Self,
    ) -> Result<Self, NipartError> {
        let merged_state = MergedNetworkState::new(
            self.clone(),
            pre_apply_state.clone(),
            Default::default(),
        )?;
        Ok(Self {
            ifaces: merged_state.ifaces.generate_revert()?,
            ..Default::default()
        })
    }
}
