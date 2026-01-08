// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{
    CUR_SCHEMA_VERSION, ErrorKind, Interfaces, JsonDisplayHideSecrets, NipartError,
    Routes,
};

#[derive(
    Clone, Debug, PartialEq, Eq, Deserialize, Serialize, JsonDisplayHideSecrets,
)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct NetworkState {
    /// Please set it to 1 explicitly
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Description for the whole desire state.
    pub description: Option<String>,
    /// Routes
    #[serde(default)]
    pub routes: Routes,
    /// Network interfaces
    #[serde(default, rename = "interfaces")]
    pub ifaces: Interfaces,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            version: Some(CUR_SCHEMA_VERSION),
            description: None,
            ifaces: Default::default(),
            routes: Default::default(),
        }
    }
}

impl NetworkState {
    pub const HIDE_PASSWORD_STR: &str = "<_password_hidden_by_nmstate>";
    /// Nipartstate cannot retrieve password
    pub const UNKNOWN_PASSWRD_STR: &str = "<_password_unknown_to_nmstate>";

    /// Return a network state with secrets only leaving self without any
    /// secrets.
    pub fn hide_secrets(&mut self) -> Self {
        let old = self.clone();
        self.ifaces.hide_secrets();
        old.gen_diff_no_sanitize(self.clone()).unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self == &Self {
            version: self.version,
            ..Default::default()
        } || (self.ifaces.is_empty() && self.routes.is_empty())
    }

    pub fn new() -> Self {
        Self::default()
    }

    /// Wrapping function of [serde_yaml::from_str()] with error mapped to
    /// [NipartError].
    pub fn new_from_yaml(net_state_yaml: &str) -> Result<Self, NipartError> {
        match serde_yaml::from_str(net_state_yaml) {
            Ok(s) => Ok(s),
            Err(e) => Err(NipartError::new(
                ErrorKind::InvalidArgument,
                format!("Invalid YAML string: {e}"),
            )),
        }
    }
}
