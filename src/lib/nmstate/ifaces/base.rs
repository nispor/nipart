// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original file are:
//  * Gris Ge <fge@redhat.com>
//  * Fernando Fernandez Mancera <ffmancera@riseup.net>
//  * Mat Kowalski <mko@redhat.com>

use serde::{Deserialize, Serialize};

use crate::{
    ErrorKind, InterfaceIpv4, InterfaceIpv6, InterfaceState, InterfaceType,
    JsonDisplay, NipartError,
};

#[derive(
    Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonDisplay,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
/// Information shared among all interface types
pub struct BaseInterface {
    pub name: String,
    // TODO(Gris Ge): Introduce `iface_name` property so we can differentiate
    // iface_name along with logical name(profile name).
    #[serde(default, rename = "type")]
    pub iface_type: InterfaceType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iface_index: Option<u32>,
    #[serde(default)]
    pub state: InterfaceState,
    /// In which order should this interface been activated. The smallest
    /// number will be activated first.
    /// Undefined or set to 0 when applying desire state means automatically
    /// decide the correct value.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub up_priority: u32,
    /// Controller interface name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller_type: Option<InterfaceType>,
    /// MAC address in the format: upper case hex string separated by `:` on
    /// every two characters. Case insensitive when applying.
    /// Serialize and deserialize to/from `mac-address`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
    /// MAC address never change after reboots(normally stored in firmware of
    /// network interface). Using the same format as `mac_address` property.
    /// Ignored during apply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permanent_mac_address: Option<String>,
    /// Maximum transmission unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtu: Option<u64>,
    /// Minimum MTU allowed. Ignored during apply.
    /// Serialize and deserialize to/from `min-mtu`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_mtu: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Maximum MTU allowed. Ignored during apply.
    /// Serialize and deserialize to/from `max-mtu`.
    pub max_mtu: Option<u64>,
    /// IPv4 information.
    /// Hided if interface is not allowed to hold IP information(e.g. port of
    /// bond is not allowed to hold IP information).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv4: Option<InterfaceIpv4>,
    /// IPv4 information.
    /// Hided if interface is not allowed to hold IP information(e.g. port of
    /// bond is not allowed to hold IP information).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6: Option<InterfaceIpv6>,
}

impl BaseInterface {
    pub fn hide_secrets(&mut self) {}

    pub fn sanitize(&mut self, current: Option<&Self>) -> Result<(), NipartError> {
        if let Some(ipv4) = self.ipv4.as_mut() {
            ipv4.sanitize(current.and_then(|c| c.ipv4.as_ref()))?;
        }
        if let Some(ipv6) = self.ipv6.as_mut() {
            ipv6.sanitize(current.and_then(|c| c.ipv6.as_ref()))?;
        }
        self.iface_index = None;
        self.validate_mtu(current)?;
        Ok(())
    }

    fn validate_mtu(&self, current: Option<&Self>) -> Result<(), NipartError> {
        if let Some(current) = current {
            if let (Some(desire_mtu), Some(min_mtu), Some(max_mtu)) =
                (self.mtu, current.min_mtu, current.max_mtu)
            {
                if desire_mtu > max_mtu {
                    return Err(NipartError::new(
                        ErrorKind::InvalidArgument,
                        format!(
                            "Desired MTU {} for interface {} is bigger than \
                             maximum allowed MTU {}",
                            desire_mtu, self.name, max_mtu
                        ),
                    ));
                } else if desire_mtu < min_mtu {
                    return Err(NipartError::new(
                        ErrorKind::InvalidArgument,
                        format!(
                            "Desired MTU {} for interface {} is smaller than \
                             minimum allowed MTU {}",
                            desire_mtu, self.name, min_mtu
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn sanitize_before_verify(&mut self, current: &mut Self) {
        self.up_priority = 0;
        if let Some(des_ipv4) = self.ipv4.as_mut()
            && let Some(cur_ipv4) = current.ipv4.as_mut()
        {
            des_ipv4.sanitize_before_verify(cur_ipv4);
        }
        if let Some(des_ipv6) = self.ipv6.as_mut()
            && let Some(cur_ipv6) = current.ipv6.as_mut()
        {
            des_ipv6.sanitize_before_verify(cur_ipv6);
        }
    }

    pub fn clone_name_type_only(&self) -> Self {
        Self {
            name: self.name.clone(),
            iface_type: self.iface_type.clone(),
            state: InterfaceState::Up,
            ..Default::default()
        }
    }

    pub(crate) fn is_absent(&self) -> bool {
        self.state == InterfaceState::Absent
    }

    pub(crate) fn is_up_priority_valid(&self) -> bool {
        if self.has_controller() {
            self.up_priority != 0
        } else {
            true
        }
    }

    fn has_controller(&self) -> bool {
        if let Some(ctrl) = self.controller.as_deref() {
            !ctrl.is_empty()
        } else {
            false
        }
    }

    pub(crate) fn include_extra_for_apply(&mut self, current: Option<&Self>) {
        self.iface_index = current.and_then(|c| c.iface_index);
    }

    pub(crate) fn is_ipv4_enabled(&self) -> bool {
        self.ipv4.as_ref().map(|i| i.is_enabled()) == Some(true)
    }

    pub(crate) fn is_ipv6_enabled(&self) -> bool {
        self.ipv6.as_ref().map(|i| i.is_enabled()) == Some(true)
    }

    pub(crate) fn need_controller(&self) -> bool {
        self.iface_type.need_controller()
    }

    /// Whether this interface can hold IP information or not.
    pub(crate) fn can_have_ip(&self) -> bool {
        (!self.has_controller())
            || self.iface_type == InterfaceType::OvsInterface
            || self.controller_type == Some(InterfaceType::Vrf)
    }
}

impl BaseInterface {
    pub fn new(name: String, iface_type: InterfaceType) -> Self {
        Self {
            name,
            iface_type,
            state: InterfaceState::Up,
            ..Default::default()
        }
    }
}

fn is_zero(d: &u32) -> bool {
    *d == 0
}
