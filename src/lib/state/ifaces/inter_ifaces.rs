// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{
    Deserialize, Deserializer, Serialize, Serializer, ser::SerializeSeq,
};

use crate::{Interface, InterfaceType, NipartError, NipartInterface};

/// Represent a list of [Interface].
///
/// With special [serde::Deserializer] and [serde::Serializer].  When applying
/// complex nested interface(e.g. bridge over bond over vlan of eth1), the
/// supported maximum nest level is 4.  For 5+ nested
/// level, you need to place controller interface before its ports.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Interfaces {
    /// Holding all interfaces with kernel representative. E.g. ethernet, bond.
    pub kernel_ifaces: HashMap<String, Interface>,
    /// Holding all interfaces which only exist in user space tool.
    /// For example: OVS bridge and IPSec
    pub user_ifaces: HashMap<(String, InterfaceType), Interface>,
    // The insert_order is allowing user to provided ordered interface
    // to support 5+ nested dependency.
    pub(crate) insert_order: Vec<(String, InterfaceType)>,
}

impl<'de> Deserialize<'de> for Interfaces {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut ret = Self::default();

        for iface in <Vec<Interface> as Deserialize>::deserialize(deserializer)?
        {
            ret.push(iface);
        }
        Ok(ret)
    }
}

impl Serialize for Interfaces {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ifaces = self.to_vec();
        let mut seq = serializer.serialize_seq(Some(ifaces.len()))?;
        for iface in ifaces {
            seq.serialize_element(iface)?;
        }
        seq.end()
    }
}

impl Interfaces {
    pub fn is_empty(&self) -> bool {
        self.kernel_ifaces.is_empty() && self.user_ifaces.is_empty()
    }

    /// Extract internal interfaces to `Vec()` and sorted by `up_priority`
    pub fn to_vec(&self) -> Vec<&Interface> {
        let mut ifaces = Vec::new();
        for iface in self.kernel_ifaces.values() {
            ifaces.push(iface);
        }
        for iface in self.user_ifaces.values() {
            ifaces.push(iface);
        }
        ifaces.sort_unstable_by_key(|iface| iface.name());
        // Use sort_by_key() instead of unstable one, do we can alphabet
        // activation order which is required to simulate the OS boot-up.
        ifaces.sort_by_key(|iface| iface.base_iface().up_priority);

        ifaces
    }

    pub(crate) fn hide_secrets(&mut self) {
        for iface in self.iter_mut() {
            iface.base_iface_mut().hide_secrets();
            iface.hide_secrets();
        }
    }

    /// The iteration order is not sorted by `up_priority`
    pub fn iter(&self) -> impl Iterator<Item = &Interface> {
        self.kernel_ifaces.values().chain(self.user_ifaces.values())
    }

    /// The iteration order is not sorted by `up_priority`
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Interface> {
        self.kernel_ifaces
            .values_mut()
            .chain(self.user_ifaces.values_mut())
    }

    /// The iteration order is not sorted by `up_priority`
    pub fn drain(&mut self) -> impl Iterator<Item = Interface> {
        self.kernel_ifaces
            .drain()
            .map(|(_, iface)| iface)
            .chain(self.user_ifaces.drain().map(|(_, iface)| iface))
    }

    /// Search interface based on interface name and interface type.
    /// When iface_type is defined, we validate interface type also.
    /// The first interface matches will be returned.
    pub fn get<'a>(
        &'a self,
        iface_name: &str,
        iface_type: Option<&InterfaceType>,
    ) -> Option<&'a Interface> {
        for iface in self.iter().filter(|i| i.name() == iface_name) {
            if let Some(des_iface_type) = iface_type {
                if des_iface_type == iface.iface_type() {
                    return Some(iface);
                }
            } else {
                return Some(iface);
            }
        }
        None
    }

    pub fn get_mut<'a>(
        &'a mut self,
        iface_name: &str,
        iface_type: Option<&InterfaceType>,
    ) -> Option<&'a mut Interface> {
        for iface in self.iter_mut().filter(|i| i.name() == iface_name) {
            if let Some(des_iface_type) = iface_type {
                if des_iface_type == iface.iface_type() {
                    return Some(iface);
                }
            } else {
                return Some(iface);
            }
        }
        None
    }

    /// Append specified [Interface].
    pub fn push(&mut self, iface: Interface) {
        self.insert_order
            .push((iface.name().to_string(), iface.iface_type().clone()));
        if iface.is_userspace() {
            self.user_ifaces.insert(
                (iface.name().to_string(), iface.iface_type().clone()),
                iface,
            );
        } else {
            self.kernel_ifaces.insert(iface.name().to_string(), iface);
        }
    }

    /// Remove interface based on interface name and interface type.
    /// When iface_type is defined, we validate interface type also.
    /// The first interface matches will be returned.
    pub fn remove(
        &mut self,
        iface_name: &str,
        iface_type: Option<&InterfaceType>,
    ) -> Option<Interface> {
        if let Some(iface_ref) = self.get(iface_name, iface_type) {
            let is_userspace = iface_ref.is_userspace();
            let iface_type = iface_ref.iface_type().clone();

            if is_userspace {
                self.user_ifaces
                    .remove(&(iface_name.to_string(), iface_type))
            } else {
                self.kernel_ifaces.remove(iface_name)
            }
        } else {
            None
        }
    }

    pub fn merge(&mut self, new_ifaces: &Self) -> Result<(), NipartError> {
        for new_iface in new_ifaces.iter() {
            if let Some(iface) =
                self.get_mut(new_iface.name(), Some(new_iface.iface_type()))
            {
                iface.merge(new_iface)?;
            } else {
                self.push(new_iface.clone());
            }
        }
        Ok(())
    }
}
