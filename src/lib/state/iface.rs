// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    state::value::copy_undefined_value, BaseInterface, EthernetInterface,
    InterfaceState, InterfaceType, NipartError, UnknownInterface,
};

/// Trait implemented by all type of interfaces.
pub trait NipartInterface:
    std::fmt::Debug + for<'a> Deserialize<'a> + Serialize
{
    fn base_iface(&self) -> &BaseInterface;

    fn base_iface_mut(&mut self) -> &mut BaseInterface;

    fn name(&self) -> &str {
        self.base_iface().name.as_str()
    }

    fn iface_type(&self) -> &InterfaceType {
        &self.base_iface().iface_type
    }

    /// Invoke [BaseInterface::hide_secrets()] and interface specifics
    /// `hide_secrets_iface_specific()`.
    /// Will invoke `hide_secrets_iface_spec()` at the end.
    /// Please do not override this but implement
    /// `hide_secrets_iface_specific()` instead.
    fn hide_secrets(&mut self) {
        self.base_iface_mut().hide_secrets();
        self.hide_secrets_iface_specific();
    }

    fn hide_secrets_iface_specific(&mut self) {}

    fn is_userspace(&self) -> bool {
        false
    }

    fn is_controller(&self) -> bool {
        false
    }

    fn is_ignore(&self) -> bool {
        self.base_iface().state.is_ignore()
    }

    fn is_up(&self) -> bool {
        self.base_iface().state.is_up()
    }

    fn is_absent(&self) -> bool {
        self.base_iface().state == InterfaceState::Absent
    }

    /// Use properties defined in new_state to override Self without
    /// understanding the property meaning and limitation.
    /// Will invoke `merge_iface_specific()` at the end.
    /// Please do not override this function but implement
    /// `merge_iface_specific()` instead.
    fn merge(&mut self, new_state: &Self) -> Result<(), NipartError>
    where
        for<'de> Self: Deserialize<'de>,
    {
        let mut new_value = serde_json::to_value(new_state)?;
        let old_value = serde_json::to_value(&self)?;
        copy_undefined_value(&mut new_value, &old_value);

        *self = serde_json::from_value(new_value)?;
        self.base_iface_mut().merge(new_state.base_iface());
        self.merge_iface_specific(new_state)?;

        Ok(())
    }

    /// Please implemented this function if special merge action required
    /// for certain interface type. Do not need to worry about the merge of
    /// [BaseInterface].
    fn merge_iface_specific(
        &mut self,
        _new_state: &Self,
    ) -> Result<(), NipartError> {
        Ok(())
    }
}

/// Controller Interface
///
/// E.g. Bond, Linux bridge, OVS bridge, VRF
pub trait NipartControllerInterface: NipartInterface {}

/// Interface depend on its parent interface
///
/// E.g VLAN, VxLAN, MacVlan
pub trait NipartChildInterface: NipartInterface {}

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
        Self::Unknown(Box::new(UnknownInterface::default()))
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
            _ => todo!(),
        }
    }
}

impl NipartInterface for Interface {
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

    fn hide_secrets(&mut self) {
        self.base_iface_mut().hide_secrets();
        match self {
            Self::Ethernet(i) => i.hide_secrets(),
            Self::Unknown(i) => i.hide_secrets(),
        }
    }

    fn is_userspace(&self) -> bool {
        match self {
            Self::Ethernet(i) => i.is_userspace(),
            Self::Unknown(i) => i.is_userspace(),
        }
    }
}
