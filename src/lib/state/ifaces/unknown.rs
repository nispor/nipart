// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Deserializer, Serialize};

use crate::{BaseInterface, NipartError, NipartInterface};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
/// Holder for interface with unknown interface type defined.
/// During apply action, nmstate can resolve unknown interface to first
/// found interface type.
pub struct UnknownInterface {
    #[serde(flatten)]
    pub base: BaseInterface,
    #[serde(flatten)]
    pub(crate) other: serde_json::Value,
}

impl NipartInterface for UnknownInterface {
    fn base_iface(&self) -> &BaseInterface {
        &self.base
    }

    fn base_iface_mut(&mut self) -> &mut BaseInterface {
        &mut self.base
    }

    /// Not sure is kernel interface or user space interface, return true
    /// always.
    fn is_userspace(&self) -> bool {
        true
    }

    /// Not sure is kernel interface or user space interface, return true
    /// always.
    fn is_virtual(&self) -> bool {
        true
    }

    /// Unknown interface cannot be controller
    fn is_controller(&self) -> bool {
        false
    }

    fn hide_secrets_iface_specific(&mut self) {}

    fn sanitize_iface_specfic(
        &mut self,
        _is_desired: bool,
    ) -> Result<(), NipartError> {
        Ok(())
    }

    fn include_diff_context_iface_specific(
        &mut self,
        _desired: &Self,
        _current: &Self,
    ) {
    }

    fn include_revert_context_iface_specific(
        &mut self,
        _desired: &Self,
        _pre_apply: &Self,
    ) {
    }
}

impl<'de> Deserialize<'de> for UnknownInterface {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut ret = UnknownInterface::default();
        let mut v = serde_json::Map::deserialize(deserializer)?;
        let mut base_value = serde_json::map::Map::new();
        if let Some(n) = v.remove("name") {
            base_value.insert("name".to_string(), n);
        }
        if let Some(s) = v.remove("state") {
            base_value.insert("state".to_string(), s);
        }
        // The BaseInterface will only have name and state
        // These two properties are also stored in `other` for serializing
        ret.base = BaseInterface::deserialize(
            serde_json::value::Value::Object(base_value),
        )
        .map_err(serde::de::Error::custom)?;
        ret.other = serde_json::Value::Object(v);
        Ok(ret)
    }
}
