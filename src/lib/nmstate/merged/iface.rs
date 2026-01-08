// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original file are:
//  * Gris Ge <fge@redhat.com>
//  * Fernando Fernandez Mancera <ffmancera@riseup.net>
//  * Wen Liang <liangwen12year@gmail.com>
//  * Íñigo Huguet <ihuguet@redhat.com>
//  * Quique Llorente <ellorent@redhat.com>

use serde::{Deserialize, Serialize};

use crate::{
    ErrorKind, Interface, InterfaceState, InterfaceType, JsonDisplay, NipartError,
    NipartstateInterface,
};

#[derive(
    Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonDisplay,
)]
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
            (Some(desired), Some(current)) => current.merge(desired)?,
            (Some(state), None) | (None, Some(state)) => state.clone(),
            _ => {
                log::warn!(
                    "BUG: MergedInterface:new() got both desired and current \
                     set to None"
                );
                Interface::default()
            }
        };
        let for_apply = if let Some(desired) = desired.as_ref() {
            let mut ret = desired.clone();
            ret.base_iface_mut().include_extra_for_apply(
                current.as_ref().map(|c| c.base_iface()),
            );
            Some(ret)
        } else {
            None
        };

        Ok(Self {
            for_apply,
            for_verify: desired.clone(),
            desired,
            current,
            merged,
        })
    }

    pub(crate) fn is_desired(&self) -> bool {
        self.desired.is_some()
    }

    pub fn is_changed(&self) -> bool {
        self.for_apply.is_some()
    }

    pub fn hide_secrets(&mut self) {
        for s in [
            self.desired.as_mut(),
            self.current.as_mut(),
            Some(&mut self.merged),
            self.for_apply.as_mut(),
            self.for_verify.as_mut(),
        ]
        .iter_mut()
        .flatten()
        {
            s.hide_secrets()
        }
    }

    pub fn mark_as_changed(&mut self) {
        self.for_apply = self.current.as_ref().map(|iface| {
            Interface::from(iface.base_iface().clone_name_type_only())
        });
    }

    pub(crate) fn apply_ctrller_change(
        &mut self,
        ctrl_name: String,
        ctrl_type: InterfaceType,
        ctrl_state: InterfaceState,
    ) -> Result<(), NipartError> {
        if self.merged.base_iface().need_controller() && ctrl_name.is_empty() {
            if let Some(org_ctrl) = self
                .current
                .as_ref()
                .and_then(|c| c.base_iface().controller.as_ref())
            {
                if Some(true) == self.for_apply.as_ref().map(|i| i.is_up()) {
                    return Err(NipartError::new(
                        ErrorKind::InvalidArgument,
                        format!(
                            "Interface {} cannot live without controller, but \
                             it is detached from original controller \
                             {org_ctrl}, cannot apply desired `state:up`",
                            self.merged.name()
                        ),
                    ));
                }
            }
        }

        if !self.is_desired() {
            self.mark_as_changed();
            if ctrl_state == InterfaceState::Up {
                self.merged.base_iface_mut().state = InterfaceState::Up;
                if let Some(apply_iface) = self.for_apply.as_mut() {
                    apply_iface.base_iface_mut().state = InterfaceState::Up;
                }
            }
            log::info!(
                "Include interface {} to edit as its controller required so",
                self.merged.name()
            );
        }
        let Some(apply_iface) = self.for_apply.as_mut() else {
            return Err(NipartError::new(
                ErrorKind::Bug,
                format!(
                    "Reached unreachable code: apply_ctrller_change() \
                     self.for_apply still None: {self:?}"
                ),
            ));
        };

        // Some interface cannot live without controller
        if self.merged.base_iface().need_controller() && ctrl_name.is_empty() {
            if let Some(org_ctrl) = self
                .current
                .as_ref()
                .and_then(|c| c.base_iface().controller.as_ref())
            {
                log::info!(
                    "Interface {} cannot live without controller, marking as \
                     absent as it has been detached from its original \
                     controller {org_ctrl}",
                    self.merged.name(),
                );
            }
            self.merged.base_iface_mut().state = InterfaceState::Absent;
            apply_iface.base_iface_mut().state = InterfaceState::Absent;
            if let Some(verify_iface) = self.for_verify.as_mut() {
                verify_iface.base_iface_mut().state = InterfaceState::Absent;
            }
        } else {
            log::info!(
                "Changing controller of interface {}/{} to: {}/{}",
                apply_iface.name(),
                apply_iface.iface_type(),
                ctrl_name,
                ctrl_type
            );
            apply_iface.base_iface_mut().controller =
                Some(ctrl_name.to_string());
            apply_iface.base_iface_mut().controller_type =
                Some(ctrl_type.clone());
            self.merged.base_iface_mut().controller = Some(ctrl_name);
            self.merged.base_iface_mut().controller_type = Some(ctrl_type);
            if !self.merged.base_iface().can_have_ip() {
                self.merged.base_iface_mut().ipv4 = None;
                self.merged.base_iface_mut().ipv6 = None;
            }
        }
        Ok(())
    }
}
