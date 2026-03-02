// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{
    InterfaceType, JsonDisplayHideSecrets, MergedInterfaces, MergedRoutes,
    NetworkState, NipartError, NmstateApplyOption, NmstateInterface,
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
    pub option: NmstateApplyOption,
}

impl MergedNetworkState {
    pub fn new(
        desired: NetworkState,
        current: NetworkState,
        option: NmstateApplyOption,
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

    /// Retains interface which can be bring up in `MergedInterfaces`
    pub fn remove_conditional_activation(&mut self) {
        let mut pending_changes: Vec<(String, InterfaceType)> = Vec::new();
        for merged_iface in self.ifaces.iter().filter(|i| i.for_apply.is_some())
        {
            if merged_iface.merged.is_up()
                && !merged_iface.can_bring_up(&self.ifaces)
            {
                pending_changes.push((
                    merged_iface.merged.name().to_string(),
                    merged_iface.merged.iface_type().clone(),
                ));
            }
        }
        for (iface_name, iface_type) in pending_changes {
            log::trace!(
                "Interface {}/{} is ignored for instant apply because its \
                 trigger condition is not met yet",
                iface_name,
                iface_type
            );
            if iface_type.is_userspace() {
                self.ifaces.user_ifaces.remove(&(iface_name, iface_type));
            } else {
                self.routes
                    .route_changed_ifaces
                    .retain(|n| n != &iface_name);
                self.routes.changed_routes.retain(|rt| {
                    rt.next_hop_iface.as_ref() != Some(&iface_name)
                });
                if let Some(config_rts) = self.routes.desired.config.as_mut() {
                    config_rts.retain(|rt| {
                        rt.next_hop_iface.as_ref() != Some(&iface_name)
                    });
                }
                self.ifaces.kernel_ifaces.remove(&iface_name);
            }
        }
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
