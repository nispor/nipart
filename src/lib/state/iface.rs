// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Deserializer, Serialize};

use super::value::get_json_value_difference;
use crate::{
    BaseInterface, ErrorKind, EthernetInterface, InterfaceState, InterfaceType,
    NipartError, NipartInterface, UnknownInterface,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", untagged)]
#[non_exhaustive]
/// Represent a kernel or user space network interface.
pub enum Interface {
    /// Ethernet interface.
    Ethernet(Box<EthernetInterface>),
    /// Unknown interface.
    Unknown(Box<UnknownInterface>),
}

impl Default for Interface {
    fn default() -> Self {
        Self::Unknown(Box::default())
    }
}

impl<'de> Deserialize<'de> for Interface {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut v = serde_json::Value::deserialize(deserializer)?;

        // It is safe to do `v["state"]` here as serde_json will
        // return `json!(null)` for undefined property
        if matches!(
            Option::deserialize(&v["state"])
                .map_err(serde::de::Error::custom)?,
            Some(InterfaceState::Absent)
        ) {
            // Ignore all properties except type if state: absent
            let mut new_value = serde_json::map::Map::new();
            if let Some(n) = v.get("name") {
                new_value.insert("name".to_string(), n.clone());
            }
            if let Some(t) = v.get("type") {
                new_value.insert("type".to_string(), t.clone());
            }
            if let Some(s) = v.get("state") {
                new_value.insert("state".to_string(), s.clone());
            }
            v = serde_json::value::Value::Object(new_value);
        }

        match Option::deserialize(&v["type"])
            .map_err(serde::de::Error::custom)?
        {
            Some(InterfaceType::Ethernet) => {
                let inner = EthernetInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::Ethernet(Box::new(inner)))
            }
            _ => {
                let inner = UnknownInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::Unknown(Box::new(inner)))
            }
        }
    }
}

impl NipartInterface for Interface {
    fn is_virtual(&self) -> bool {
        match self {
            Self::Ethernet(i) => i.is_virtual(),
            Self::Unknown(i) => i.is_virtual(),
        }
    }

    fn is_userspace(&self) -> bool {
        match self {
            Self::Ethernet(i) => i.is_userspace(),
            Self::Unknown(i) => i.is_userspace(),
        }
    }

    fn is_controller(&self) -> bool {
        match self {
            Self::Ethernet(i) => i.is_controller(),
            Self::Unknown(i) => i.is_controller(),
        }
    }

    fn base_iface(&self) -> &BaseInterface {
        match self {
            Self::Ethernet(i) => (*i).base_iface(),
            Self::Unknown(i) => (*i).base_iface(),
        }
    }

    fn base_iface_mut(&mut self) -> &mut BaseInterface {
        match self {
            Self::Ethernet(i) => i.base_iface_mut(),
            Self::Unknown(i) => i.base_iface_mut(),
        }
    }

    fn hide_secrets_iface_specific(&mut self) {
        match self {
            Self::Ethernet(i) => i.hide_secrets_iface_specific(),
            Self::Unknown(i) => i.hide_secrets_iface_specific(),
        }
    }

    fn sanitize_iface_specfic(
        &mut self,
        is_desired: bool,
    ) -> Result<(), NipartError> {
        match self {
            Self::Ethernet(i) => i.sanitize_iface_specfic(is_desired),
            Self::Unknown(i) => i.sanitize_iface_specfic(is_desired),
        }
    }

    fn include_diff_context_iface_specific(
        &mut self,
        desired: &Self,
        current: &Self,
    ) {
        match (self, desired, current) {
            (
                Self::Ethernet(i),
                Self::Ethernet(desired),
                Self::Ethernet(current),
            ) => i.include_diff_context_iface_specific(desired, current),
            (
                Self::Unknown(i),
                Self::Unknown(desired),
                Self::Unknown(current),
            ) => i.include_diff_context_iface_specific(desired, current),
            _ => {
                log::error!(
                    "BUG: Interface::include_diff_context_iface_specific() \
                    Unexpected input desired {desired:?} current {current:?}",
                );
            }
        }
    }

    fn include_revert_context_iface_specific(
        &mut self,
        desired: &Self,
        pre_apply: &Self,
    ) {
        match (self, desired, pre_apply) {
            (
                Self::Ethernet(i),
                Self::Ethernet(desired),
                Self::Ethernet(pre_apply),
            ) => i.include_revert_context_iface_specific(desired, pre_apply),
            (
                Self::Unknown(i),
                Self::Unknown(desired),
                Self::Unknown(pre_apply),
            ) => i.include_revert_context_iface_specific(desired, pre_apply),
            _ => {
                log::error!(
                    "BUG: Interface::include_revert_context_iface_specific() \
                    Unexpected input desired {desired:?} \
                    pre_apply {pre_apply:?}"
                );
            }
        }
    }
}

impl From<BaseInterface> for Interface {
    fn from(base_iface: BaseInterface) -> Self {
        match base_iface.iface_type {
            InterfaceType::Ethernet | InterfaceType::Veth => {
                Interface::Ethernet(Box::new(EthernetInterface::from_base(
                    base_iface,
                )))
            }
            InterfaceType::Unknown(_) => Interface::Unknown(Box::new(
                UnknownInterface::from_base(base_iface),
            )),
            _ => {
                log::warn!(
                    "Unsupported interface type {} for interface {}",
                    base_iface.iface_type,
                    base_iface.name
                );
                Interface::Unknown(Box::new(UnknownInterface::from_base(
                    base_iface,
                )))
            }
        }
    }
}

impl Interface {
    pub(crate) fn clone_name_type_only(&self) -> Self {
        self.base_iface().clone_name_type_only().into()
    }

    pub(crate) fn verify(&self, current: &Self) -> Result<(), NipartError> {
        let self_value = serde_json::to_value(self.clone())?;
        let current_value = serde_json::to_value(current.clone())?;

        if let Some((reference, desire, current)) = get_json_value_difference(
            format!("{}.interface", self.name()),
            &self_value,
            &current_value,
        ) {
            Err(NipartError::new(
                ErrorKind::VerificationError,
                format!(
                    "Verification failure: {reference} desire '{desire}', \
                    current '{current}'"
                ),
            ))
        } else {
            Ok(())
        }
    }
}
