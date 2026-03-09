// SPDX-License-Identifier: Apache-2.0

use crate::{MergedNetworkState, NetworkState, NipartError};

impl NetworkState {
    /// Generate NetworkState containing only the properties changed comparing
    /// to `old_state`.
    pub fn gen_diff(&self, old: &Self) -> Result<Self, NipartError> {
        let mut old = old.clone();

        let mut desired = self.clone();
        desired.ifaces.sanitize_for_diff(&mut old.ifaces);

        desired.gen_diff_no_sanitize(old)
    }

    pub(crate) fn gen_diff_no_sanitize(
        self,
        old: Self,
    ) -> Result<Self, NipartError> {
        let mut ret = Self::default();
        let old_version = old.version;
        let old_description = old.description.clone();

        if self.description != old_description {
            ret.description.clone_from(&self.description);
        }
        if self.version != old_version {
            ret.version.clone_from(&self.version);
        } else {
            ret.version = None;
        }

        let merged_state =
            MergedNetworkState::new(self, old, Default::default())?;

        ret.ifaces = merged_state.ifaces.gen_diff()?;
        ret.routes = merged_state.routes.gen_diff();
        Ok(ret)
    }
}
