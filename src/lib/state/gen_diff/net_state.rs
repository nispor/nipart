// SPDX-License-Identifier: Apache-2.0

use crate::{MergedNetworkState, NetworkState, NipartError};

impl NetworkState {
    /// Generate NetworkState containing only the properties changed comparing
    /// to `old_state`.
    pub fn gen_diff(&self, old: &Self) -> Result<Self, NipartError> {
        let mut ret = Self::default();
        let merged_state = MergedNetworkState::new(
            self.clone(),
            old.clone(),
            Default::default(),
        )?;

        if self.description != old.description {
            ret.description.clone_from(&self.description);
        }

        ret.ifaces = merged_state.ifaces.gen_diff()?;
        Ok(ret)
    }
}
