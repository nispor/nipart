// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original file are:
//  * Gris Ge <fge@redhat.com>
//  * Marcelo Guerrero <marguerr@redhat.com>

use serde::{Deserialize, Serialize};

use crate::{
    BaseInterface, ErrorKind, InterfaceType, JsonDisplay, NipartError,
    NipartstateInterface,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonDisplay)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
/// Ethernet(IEEE 802.3) interface.
pub struct EthernetInterface {
    #[serde(flatten)]
    pub base: BaseInterface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ethernet: Option<EthernetConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub veth: Option<VethConfig>,
}

impl EthernetInterface {
    pub fn new(base: BaseInterface, ethernet: Option<EthernetConfig>) -> Self {
        Self {
            base,
            ethernet,
            ..Default::default()
        }
    }

    pub fn new_veth(base: BaseInterface, veth_peer: &str) -> Self {
        Self {
            base,
            veth: Some(VethConfig {
                peer: veth_peer.to_string(),
            }),
            ..Default::default()
        }
    }
}

impl Default for EthernetInterface {
    fn default() -> Self {
        Self {
            base: BaseInterface {
                iface_type: InterfaceType::Ethernet,
                ..Default::default()
            },
            ethernet: None,
            veth: None,
        }
    }
}

impl NipartstateInterface for EthernetInterface {
    fn base_iface(&self) -> &BaseInterface {
        &self.base
    }

    fn base_iface_mut(&mut self) -> &mut BaseInterface {
        &mut self.base
    }

    fn is_virtual(&self) -> bool {
        false
    }

    fn hide_secrets_iface_specific(&mut self) {}

    fn sanitize_iface_specfic(
        &mut self,
        current: Option<&Self>,
    ) -> Result<(), NipartError> {
        if self.is_up() && current.is_none() && self.veth.is_none() {
            return Err(NipartError::new(
                ErrorKind::InvalidArgument,
                format!(
                    "Interface {} does not exist and veth section is not \
                     defined to create it",
                    self.base.name
                ),
            ));
        }
        Ok(())
    }

    fn include_revert_context_iface_specific(
        &mut self,
        _desired: &Self,
        _pre_apply: &Self,
    ) {
        /*
        if let (Interface::Ethernet(desired), Interface::Ethernet(current)) =
            (desired, current)
        {
            if desired.sriov_is_enabled() && !current.sriov_is_enabled() {
                self.ethernet
                    .get_or_insert(EthernetConfig::new())
                    .sr_iov
                    .get_or_insert(SrIovConfig {
                        total_vfs: Some(0),
                        ..Default::default()
                    });
            }
        }
         */
    }

    /// Should be deleted when changing veth peer
    fn need_delete_before_change(&self, current: &Self) -> bool {
        if let Some(des_peer) = self.veth.as_ref().map(|v| v.peer.as_str())
            && let Some(cur_peer) =
                current.veth.as_ref().map(|v| v.peer.as_str())
            && des_peer != cur_peer
        {
            true
        } else {
            false
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, JsonDisplay,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct EthernetConfig {
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "auto-negotiation",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    /// Deserialize and serialize from/to `auto-negotiation`.
    pub auto_neg: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub speed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplex: Option<EthernetDuplex>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonDisplay,
)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum EthernetDuplex {
    /// Deserialize and serialize from/to `full`.
    Full,
    /// Deserialize and serialize from/to `half`.
    Half,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonDisplay)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct VethConfig {
    pub peer: String,
}
