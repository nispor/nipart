// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original file are:
//  * Gris Ge <fge@redhat.com>
//  * Fernando Fernandez Mancera <ffmancera@riseup.net>
//  * Ales Musil <amusil@redhat.com>
//  * Jan Vaclav <jvaclav@redhat.com>

use serde::{Deserialize, Serialize};

use crate::{
    BaseInterface, InterfaceType, JsonDisplay, NipartError,
    NipartstateInterface,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonDisplay)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
/// OpenvSwitch Internal Interface
pub struct OvsInterface {
    #[serde(flatten)]
    pub base: BaseInterface,
}

impl OvsInterface {
    pub fn new(base: BaseInterface) -> Self {
        Self { base }
    }

    pub(crate) fn new_with_name_and_ctrl(name: &str, ctrl_name: &str) -> Self {
        let base = BaseInterface {
            name: name.to_string(),
            iface_type: InterfaceType::OvsInterface,
            controller: Some(ctrl_name.to_string()),
            controller_type: Some(InterfaceType::OvsBridge),
            ..Default::default()
        };
        Self { base }
    }
}

impl Default for OvsInterface {
    fn default() -> Self {
        Self {
            base: BaseInterface {
                iface_type: InterfaceType::OvsInterface,
                ..Default::default()
            },
        }
    }
}

impl NipartstateInterface for OvsInterface {
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
}
