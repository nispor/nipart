// SPDX-License-Identifier: Apache-2.0

use std::net::{IpAddr, SocketAddr};

use serde::{Deserialize, Serialize};

use crate::{
    BaseInterface, ErrorKind, InterfaceType, JsonDisplay,
    JsonDisplayHideSecrets, NipartError, NipartstateInterface,
};

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonDisplayHideSecrets,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
/// Wireguard interface
pub struct WireguardInterface {
    #[serde(flatten)]
    pub base: BaseInterface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wireguard: Option<WireguardConfig>,
}

impl WireguardInterface {
    pub fn new(name: String) -> Self {
        Self {
            base: BaseInterface {
                name: name.to_string(),
                iface_type: InterfaceType::Wireguard,
                ..Default::default()
            },
            wireguard: None,
        }
    }
}

impl Default for WireguardInterface {
    fn default() -> Self {
        Self {
            base: BaseInterface {
                iface_type: InterfaceType::Wireguard,
                ..Default::default()
            },
            wireguard: None,
        }
    }
}

impl NipartstateInterface for WireguardInterface {
    fn base_iface(&self) -> &BaseInterface {
        &self.base
    }

    fn base_iface_mut(&mut self) -> &mut BaseInterface {
        &mut self.base
    }

    fn is_virtual(&self) -> bool {
        true
    }

    fn hide_secrets_iface_specific(&mut self) {
        if let Some(wg_cfg) = self.wireguard.as_mut() {
            wg_cfg.hide_secrets();
        }
    }

    fn sanitize_iface_specfic(
        &mut self,
        current: Option<&Self>,
    ) -> Result<(), NipartError> {
        if let Some(wg_conf) = self.wireguard.as_mut() {
            wg_conf.sanitize(
                current.and_then(|c| c.wireguard.as_ref()),
                self.base.name.as_str(),
            )?;
        } else if current.is_none() {
            return Err(NipartError::new(
                ErrorKind::InvalidArgument,
                format!(
                    "Need wireguard section for creating wireguard interface \
                     {}",
                    self.base.name
                ),
            ));
        }
        Ok(())
    }
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    JsonDisplayHideSecrets,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct WireguardConfig {
    /// Base64 encoded public key, query only option, ignored when apply,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    /// Base64 encoded private key, will be replaced by `<_hidden_>` in
    /// display or debug format.
    /// Will use current value if defined as None or `<_hidden_>`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fwmark: Option<u32>,
    /// If undefined, we use current value. If defined, we override existing
    /// peers with desired list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peers: Option<Vec<WireguardPeerConfig>>,
}

impl WireguardConfig {
    pub fn hide_secrets(&mut self) {
        if self.private_key.is_some() {
            self.private_key =
                Some(crate::NetworkState::HIDE_SECRET_STR.to_string());
        }
        if let Some(peers) = self.peers.as_mut() {
            for peer in peers {
                peer.hide_secrets();
            }
        }
    }

    /// * Discard query only properties
    /// * Need WireguardConfig for creating interface
    /// * Need `private_key` for creating interface
    /// * Discard `private_key` if set to `HIDE_SECRET_STR`
    /// * Need `endpoint` for each peer config
    pub(crate) fn sanitize(
        &mut self,
        current: Option<&Self>,
        iface_name: &str,
    ) -> Result<(), NipartError> {
        self.public_key = None;

        if current.is_none() {
            if self.private_key.is_none() {
                return Err(NipartError::new(
                    ErrorKind::InvalidArgument,
                    format!(
                        "Need private key for creating wireguard interface {}",
                        iface_name,
                    ),
                ));
            }
        } else {
            if self.private_key.as_deref()
                == Some(crate::NetworkState::HIDE_SECRET_STR)
            {
                self.private_key = None;
            }
        }

        if let Some(peers) = self.peers.as_mut() {
            for peer in peers {
                if peer.endpoint.is_none() {
                    return Err(NipartError::new(
                        ErrorKind::InvalidArgument,
                        format!(
                            "Missing mandatory property `endpoint` for \
                             configuration wireguard peer {peer} of interface \
                             {iface_name}",
                        ),
                    ));
                }
                peer.sanitize()?;
            }
        }

        Ok(())
    }
}

// TODO: The more elegant way of hide secret during debug is using Derive like
// `#[serde(skip)]`. But that is way too complex without using deprecated quote
// crate. Let's do the silly non-rust-idiom way for now.
impl std::fmt::Debug for WireguardConfig {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        f.debug_struct("WireguardConfig")
            .field("public_key", &self.public_key)
            .field(
                "private_key",
                if self.private_key.is_some() {
                    &Some(crate::NetworkState::HIDE_SECRET_STR)
                } else {
                    &None::<&str>
                },
            )
            .field("listen_port", &self.listen_port)
            .field("fwmark", &self.fwmark)
            .field("peers", &self.peers)
            .finish()
    }
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    JsonDisplayHideSecrets,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct WireguardPeerConfig {
    /// Mandatory property for apply
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<SocketAddr>,
    /// Base64 encoded public key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    /// Base64 encoded preshared key, will be shown as `<_hidden_>` for Debug
    /// and excluded from Serialize.
    /// If undefined or defined as `<_hidden_>`, will use current value.
    #[serde(skip_serializing)]
    pub preshared_key: Option<String>,
    /// Last handshake in a format of `32 seconds ago`.
    /// Query only property, will be ignore during apply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_handshake: Option<String>,
    /// Query only property for received bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx_bytes: Option<u64>,
    /// Query only property for transmitted bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent_keepalive: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_ips: Option<Vec<WireguardIpAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<u32>,
}

impl WireguardPeerConfig {
    pub fn hide_secrets(&mut self) {
        if self.preshared_key.is_some() {
            self.preshared_key =
                Some(crate::NetworkState::HIDE_SECRET_STR.to_string());
        }
    }

    /// * Discard query only properties
    /// * Discard `preshared_key` if set to `HIDE_SECRET_STR`
    pub(crate) fn sanitize(&mut self) -> Result<(), NipartError> {
        self.rx_bytes = None;
        self.tx_bytes = None;
        self.last_handshake = None;

        if self.preshared_key.as_deref()
            == Some(crate::NetworkState::HIDE_SECRET_STR)
        {
            self.preshared_key = None;
        }

        Ok(())
    }
}

// TODO: The more elegant way of hide secret during debug is using Derive like
// `#[serde(skip)]`. But that is way too complex without using deprecated quote
// crate. Let's do the silly non-rust-idiom way for now.
impl std::fmt::Debug for WireguardPeerConfig {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        f.debug_struct("WireguardPeerConf")
            .field("endpoint", &self.endpoint)
            .field("public_key", &self.public_key)
            .field(
                "preshared_key",
                &self
                    .preshared_key
                    .as_ref()
                    .map(|_| crate::NetworkState::HIDE_SECRET_STR),
            )
            .field("last_handshake", &self.last_handshake)
            .field("rx_bytes", &self.rx_bytes)
            .field("tx_bytes", &self.tx_bytes)
            .field("persistent_keepalive", &self.persistent_keepalive)
            .field("allowed_ips", &self.allowed_ips)
            .field("protocol_version", &self.protocol_version)
            .finish()
    }
}

#[derive(
    Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Copy, JsonDisplay,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct WireguardIpAddress {
    pub ip: IpAddr,
    pub prefix_length: u8,
}
