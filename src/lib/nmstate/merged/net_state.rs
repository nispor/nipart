// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{
    JsonDisplayHideSecrets, MergedInterfaces, MergedRoutes, NetworkState,
    NipartError, NipartstateApplyOption,
};

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Deserialize,
    Serialize,
    JsonDisplayHideSecrets,
)]
#[non_exhaustive]
pub struct MergedNetworkState {
    pub version: Option<u32>,
    pub description: Option<String>,
    pub ifaces: MergedInterfaces,
    pub routes: MergedRoutes,
    pub option: NipartstateApplyOption,
}

impl MergedNetworkState {
    pub fn new(
        desired: NetworkState,
        current: NetworkState,
        option: NipartstateApplyOption,
    ) -> Result<Self, NipartError> {
        let merged_ifaces =
            MergedInterfaces::new(desired.ifaces, current.ifaces)?;
        let merged_routes =
            MergedRoutes::new(desired.routes, current.routes, &merged_ifaces)?;

        Ok(Self {
            version: desired.version,
            description: desired.description.clone(),
            ifaces: merged_ifaces,
            routes: merged_routes,
            option,
        })
    }

    pub fn verify(&self, current: &NetworkState) -> Result<(), NipartError> {
        self.ifaces.verify(&current.ifaces)
    }

    pub fn gen_state_for_apply(&self) -> NetworkState {
        NetworkState {
            ifaces: self.ifaces.gen_state_for_apply(),
            routes: self.routes.gen_state_for_apply(),
            version: self.version,
            description: self.description.clone(),
        }
    }

    pub fn hide_secrets(&mut self) {
        self.ifaces.hide_secrets()
    }
}

impl NetworkState {
    pub fn merge(&mut self, new_state: &Self) -> Result<(), NipartError> {
        *self = Self {
            version: new_state.version.or(self.version),
            description: new_state
                .description
                .clone()
                .or_else(|| self.description.clone()),
            ifaces: self.ifaces.merge(&new_state.ifaces)?,
            routes: self.routes.merge(&new_state.routes)?,
        };
        Ok(())
    }
}
