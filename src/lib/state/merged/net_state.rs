// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{MergedInterfaces, NetworkState, NipartApplyOption, NipartError};

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct MergedNetworkState {
    pub ifaces: MergedInterfaces,
    pub option: NipartApplyOption,
}

impl MergedNetworkState {
    pub fn new(
        desired: NetworkState,
        current: NetworkState,
        option: NipartApplyOption,
    ) -> Result<Self, NipartError> {
        Ok(Self {
            ifaces: MergedInterfaces::new(desired.ifaces, current.ifaces)?,
            option,
        })
    }

    pub fn verify(&self, current: &NetworkState) -> Result<(), NipartError> {
        self.ifaces.verify(&current.ifaces)
    }
}
