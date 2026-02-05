// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    ErrorKind, Interface, InterfaceType, Interfaces, JsonDisplayHideSecrets,
    MergedInterface, NipartError, NipartstateInterface,
};

// The max loop count for Interfaces.set_ifaces_up_priority()
// This allows interface with 4 nested levels in any order.
// To support more nested level, user could place top controller at the
// beginning of desire state
const INTERFACES_SET_PRIORITY_MAX_RETRY: u32 = 4;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    JsonDisplayHideSecrets,
)]
#[non_exhaustive]
pub struct MergedInterfaces {
    pub kernel_ifaces: HashMap<String, MergedInterface>,
    pub user_ifaces: HashMap<(String, InterfaceType), MergedInterface>,
    pub insert_order: Vec<(String, InterfaceType)>,
}

impl MergedInterfaces {
    pub fn hide_secrets(&mut self) {
        for merged_iface in self.iter_mut() {
            merged_iface.hide_secrets()
        }
    }

    pub fn new(
        desired: Interfaces,
        current: Interfaces,
    ) -> Result<Self, NipartError> {
        let mut desired = desired;
        let mut current = current;
        let mut insert_order: Vec<(String, InterfaceType)> = Vec::new();

        desired.unify_veth_and_ethernet();
        current.unify_veth_and_ethernet();

        desired.auto_managed_controller_ports(&current);

        let mut kernel_ifaces: HashMap<String, MergedInterface> =
            HashMap::new();
        let mut user_ifaces: HashMap<(String, InterfaceType), MergedInterface> =
            HashMap::new();
        // TODO: Remove ignore interface
        // TODO: Resolve `type: unknown` in desired based on current state
        for mut des_iface in desired.drain() {
            if des_iface.is_ignore() {
                log::info!(
                    "Ignoring interface {} for `state: ignore`",
                    des_iface.name()
                );
                continue;
            }
            insert_order.push((
                des_iface.name().to_string(),
                des_iface.iface_type().clone(),
            ));
            let cur_iface =
                current.remove(des_iface.name(), Some(des_iface.iface_type()));
            des_iface.sanitize(cur_iface.as_ref())?;
            if des_iface.is_userspace() {
                user_ifaces.insert(
                    (
                        des_iface.name().to_string(),
                        des_iface.iface_type().clone(),
                    ),
                    MergedInterface::new(Some(des_iface), cur_iface)?,
                );
            } else {
                kernel_ifaces.insert(
                    des_iface.name().to_string(),
                    MergedInterface::new(Some(des_iface), cur_iface)?,
                );
            }
        }

        for cur_iface in current.drain() {
            if cur_iface.is_userspace() {
                user_ifaces.insert(
                    (
                        cur_iface.name().to_string(),
                        cur_iface.iface_type().clone(),
                    ),
                    MergedInterface::new(None, Some(cur_iface))?,
                );
            } else {
                kernel_ifaces.insert(
                    cur_iface.name().to_string(),
                    MergedInterface::new(None, Some(cur_iface))?,
                );
            }
        }
        let mut ret = Self {
            kernel_ifaces,
            user_ifaces,
            insert_order,
        };

        ret.post_merge_sanitize()?;

        ret._set_up_priority()?;

        Ok(ret)
    }

    pub(crate) fn get_iface<'a>(
        &'a self,
        iface_name: &str,
        iface_type: InterfaceType,
    ) -> Option<&'a MergedInterface> {
        if iface_type.is_userspace() {
            self.user_ifaces.get(&(iface_name.to_string(), iface_type))
        } else {
            self.kernel_ifaces.get(iface_name)
        }
    }

    fn _set_up_priority(&mut self) -> Result<(), NipartError> {
        for _ in 0..INTERFACES_SET_PRIORITY_MAX_RETRY {
            if self.set_ifaces_up_priority() {
                return Ok(());
            }
        }
        log::error!(
            "Failed to set up priority: please order the interfaces in desire \
             state to place controller before its ports"
        );
        Err(NipartError::new(
            ErrorKind::InvalidArgument,
            "Failed to set up priority: nmstate only support nested interface \
             up to 4 levels. To support more nest level, please order the \
             interfaces in desire state to place controller before its ports"
                .to_string(),
        ))
    }

    pub fn iter(&self) -> impl Iterator<Item = &MergedInterface> {
        self.user_ifaces.values().chain(self.kernel_ifaces.values())
    }

    pub fn gen_state_for_apply(&self) -> Interfaces {
        let kernel_ifaces: HashMap<String, Interface> = self
            .kernel_ifaces
            .iter()
            .filter_map(|(name, iface)| {
                iface
                    .for_apply
                    .as_ref()
                    .map(|i| (name.to_string(), i.clone()))
            })
            .collect();

        let user_ifaces: HashMap<(String, InterfaceType), Interface> = self
            .user_ifaces
            .iter()
            .filter_map(|((name, iface_type), iface)| {
                iface.for_apply.as_ref().map(|i| {
                    ((name.to_string(), iface_type.clone()), i.clone())
                })
            })
            .collect();

        Interfaces {
            kernel_ifaces,
            user_ifaces,
            ..Default::default()
        }
    }

    pub(crate) fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut MergedInterface> {
        self.user_ifaces
            .values_mut()
            .chain(self.kernel_ifaces.values_mut())
    }

    pub(crate) fn verify(
        &self,
        current: &Interfaces,
    ) -> Result<(), NipartError> {
        let mut merged = self.clone();
        let mut current = current.clone();

        current.unify_veth_and_ethernet();
        for des_iface in merged.iter_mut().filter(|i| i.is_desired()) {
            let iface = if let Some(i) = des_iface.for_verify.as_mut() {
                i
            } else {
                continue;
            };
            iface.hide_secrets();

            if iface.is_absent() || (iface.is_virtual() && iface.is_down()) {
                if let Some(cur_iface) =
                    current.get(iface.name(), Some(iface.iface_type()))
                {
                    verify_desire_absent_but_found_in_current(
                        iface, cur_iface,
                    )?;
                }
            } else if let Some(cur_iface) =
                current.get_mut(iface.name(), Some(iface.iface_type()))
            {
                iface.sanitize_before_verify(cur_iface);
                // Do not verify physical interface with state:down
                if iface.is_up() {
                    iface.verify(cur_iface)?;
                }
            } else if iface.is_up() {
                return Err(NipartError::new(
                    ErrorKind::VerificationError,
                    format!(
                        "Failed to find desired interface {} {:?}",
                        iface.name(),
                        iface.iface_type()
                    ),
                ));
            }
        }
        Ok(())
    }

    fn post_merge_sanitize(&mut self) -> Result<(), NipartError> {
        for iface in self
            .kernel_ifaces
            .values_mut()
            .chain(self.user_ifaces.values_mut())
        {
            if iface.merged.iface_type() == &InterfaceType::Loopback {
                iface.post_merge_sanitize_loopback();
            }
            if iface.merged.iface_type() == &InterfaceType::Vlan {
                iface.post_merge_sanitize_vlan();
            }
        }

        self.post_merge_sanitize_veth();
        self.post_merge_sanitize_wifi();
        self.post_merge_sanitize_controller_and_port()?;

        Ok(())
    }

    // Return True if we have all up_priority fixed.
    pub(crate) fn set_ifaces_up_priority(&mut self) -> bool {
        // Return true when all interface has correct priority.
        let mut ret = true;
        let mut pending_changes: HashMap<String, u32> = HashMap::new();
        // Use the push order to allow user providing help on dependency order

        for (iface_name, iface_type) in &self.insert_order {
            let iface = match self.get_iface(iface_name, iface_type.clone()) {
                Some(i) => {
                    if let Some(i) = i.for_apply.as_ref() {
                        i
                    } else {
                        continue;
                    }
                }
                None => continue,
            };
            if !iface.is_up() {
                continue;
            }

            if iface.base_iface().is_up_priority_valid() {
                continue;
            }

            if let Some(ref ctrl_name) = iface.base_iface().controller {
                if ctrl_name.is_empty() {
                    continue;
                }
                let ctrl_iface = self
                    .get_iface(
                        ctrl_name,
                        iface
                            .base_iface()
                            .controller_type
                            .clone()
                            .unwrap_or_default(),
                    )
                    .and_then(|i| i.for_apply.as_ref());
                if let Some(ctrl_iface) = ctrl_iface {
                    if let Some(ctrl_pri) = pending_changes.remove(ctrl_name) {
                        pending_changes.insert(ctrl_name.to_string(), ctrl_pri);
                        pending_changes
                            .insert(iface_name.to_string(), ctrl_pri + 1);
                    } else if ctrl_iface.base_iface().is_up_priority_valid() {
                        pending_changes.insert(
                            iface_name.to_string(),
                            ctrl_iface.base_iface().up_priority + 1,
                        );
                    } else {
                        // Its controller does not have valid up priority yet.
                        log::debug!(
                            "Controller {ctrl_name} of {iface_name} is has no \
                             up priority"
                        );
                        ret = false;
                    }
                } else {
                    // Interface has no controller defined in desire
                    continue;
                }
            } else {
                continue;
            }
        }

        // If not remaining unknown up_priority, we set up the parent/child
        // up_priority
        if ret {
            for (iface_name, iface_type) in &self.insert_order {
                let iface = match self.get_iface(iface_name, iface_type.clone())
                {
                    Some(i) => {
                        if let Some(i) = i.for_apply.as_ref() {
                            i
                        } else {
                            continue;
                        }
                    }
                    None => continue,
                };
                if !iface.is_up() {
                    continue;
                }
                if let Some(parent) = iface.parent() {
                    let parent_priority = pending_changes.get(parent).cloned();
                    if let Some(parent_priority) = parent_priority {
                        pending_changes.insert(
                            iface_name.to_string(),
                            parent_priority + 1,
                        );
                    } else if let Some(parent_iface) = self
                        .kernel_ifaces
                        .get(parent)
                        .and_then(|i| i.for_apply.as_ref())
                        && parent_iface.base_iface().is_up_priority_valid()
                    {
                        pending_changes.insert(
                            iface_name.to_string(),
                            parent_iface.base_iface().up_priority + 1,
                        );
                    }
                }
            }
        }

        if !pending_changes.is_empty() {
            log::debug!(
                "Pending kernel up priority changes {pending_changes:?}"
            );
            for (iface_name, priority) in pending_changes.iter() {
                if let Some(iface) = self
                    .kernel_ifaces
                    .get_mut(iface_name)
                    .and_then(|i| i.for_apply.as_mut())
                {
                    iface.base_iface_mut().up_priority = *priority;
                }
            }
        }

        ret
    }
}

fn verify_desire_absent_but_found_in_current(
    des_iface: &Interface,
    cur_iface: &Interface,
) -> Result<(), NipartError> {
    if cur_iface.is_virtual() {
        // Virtual interface should be deleted by absent action
        Err(NipartError::new(
            ErrorKind::VerificationError,
            format!(
                "Absent/Down interface {}/{} still found as {:?}",
                des_iface.name(),
                des_iface.iface_type(),
                cur_iface
            ),
        ))
    } else {
        // Hard to predict real hardware state due to backend variety.
        Ok(())
    }
}

impl Interfaces {
    pub fn unify_veth_and_ethernet(&mut self) {
        for iface in self
            .kernel_ifaces
            .values_mut()
            .filter(|i| i.iface_type() == &InterfaceType::Veth)
        {
            iface.base_iface_mut().iface_type = InterfaceType::Ethernet;
        }
    }

    pub(crate) fn merge(&self, new_ifaces: &Self) -> Result<Self, NipartError> {
        let mut ret = Self::default();
        for new_iface in new_ifaces.iter() {
            if let Some(old_iface) =
                self.get(new_iface.name(), Some(new_iface.iface_type()))
            {
                ret.push(old_iface.merge(new_iface)?);
            } else {
                ret.push(new_iface.clone());
            }
        }

        for old_iface in self.iter().filter(|old_iface| {
            new_ifaces
                .get(old_iface.name(), Some(old_iface.iface_type()))
                .is_none()
        }) {
            ret.push(old_iface.clone());
        }

        ret.post_merge_veth(new_ifaces);

        Ok(ret)
    }
}
