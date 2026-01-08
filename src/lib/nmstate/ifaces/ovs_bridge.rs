// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original file are:
//  * Gris Ge <fge@redhat.com>
//  * Fernando Fernandez Mancera <ffmancera@riseup.net>
//  * Ales Musil <amusil@redhat.com>
//  * Jan Vaclav <jvaclav@redhat.com>

use serde::{Deserialize, Serialize};

use crate::{
    BaseInterface, InterfaceType, JsonDisplay, NipartError, NipartstateInterface,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonDisplay)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
/// OpenvSwitch Bridge
pub struct OvsBridgeInterface {
    #[serde(flatten)]
    pub base: BaseInterface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge: Option<OvsBridgeConfig>,
}

impl OvsBridgeInterface {
    pub fn new(base: BaseInterface, bridge: Option<OvsBridgeConfig>) -> Self {
        Self { base, bridge }
    }
}

impl Default for OvsBridgeInterface {
    fn default() -> Self {
        Self {
            base: BaseInterface {
                iface_type: InterfaceType::OvsBridge,
                ..Default::default()
            },
            bridge: None,
        }
    }
}

impl NipartstateInterface for OvsBridgeInterface {
    fn base_iface(&self) -> &BaseInterface {
        &self.base
    }

    fn base_iface_mut(&mut self) -> &mut BaseInterface {
        &mut self.base
    }

    fn is_virtual(&self) -> bool {
        true
    }

    fn hide_secrets_iface_specific(&mut self) {}

    fn sanitize_iface_specfic(
        &mut self,
        _current: Option<&Self>,
    ) -> Result<(), NipartError> {
        Ok(())
    }

    fn include_diff_context_iface_specific(
        &mut self,
        _desired: &Self,
        _current: &Self,
    ) {
        // TODO(Gris Ge): Include full port config if any changed
    }

    fn include_revert_context_iface_specific(
        &mut self,
        _desired: &Self,
        _pre_apply: &Self,
    ) {
        // TODO(Gris Ge): Include full port config if any changed
    }

    fn ports(&self) -> Option<Vec<&str>> {
        if let Some(br_conf) = &self.bridge {
            if let Some(port_confs) = &br_conf.ports {
                let mut port_names = Vec::new();
                for port_conf in port_confs {
                    port_names.push(port_conf.name.as_str());
                }
                return Some(port_names);
            }
        }
        None
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonDisplay,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct OvsBridgeConfig {
    /// Serialize to 'port'. Deserialize from `port` or `ports`.
    pub ports: Option<Vec<OvsBridgePortConfig>>,
}

#[derive(
    Debug, Clone, PartialEq, Default, Eq, Serialize, Deserialize, JsonDisplay,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct OvsBridgePortConfig {
    pub name: String,
}
