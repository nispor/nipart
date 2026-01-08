// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, str::FromStr};

use zvariant::OwnedObjectPath;

use crate::{ErrorKind, NipartError, WifiConfig};

#[derive(Debug, Clone, Default)]
pub(crate) struct WpaSupNetwork {
    pub(crate) obj_path: OwnedObjectPath,
    pub(crate) ssid: String,
    pub(crate) bssid: Option<String>,
    pub(crate) psk: Option<String>,
    pub(crate) sae_password: Option<String>,
    pub(crate) key_mgmt: Option<String>,
    pub(crate) ieee80211w: Option<i32>,
}

impl From<WpaSupNetwork> for WifiConfig {
    fn from(v: WpaSupNetwork) -> Self {
        WifiConfig {
            ssid: v.ssid.clone(),
            bssid: v.bssid.clone(),
            ..Default::default()
        }
    }
}

impl WpaSupNetwork {
    pub(crate) fn from_value(
        value: zvariant::OwnedValue,
        obj_path: OwnedObjectPath,
    ) -> Result<Self, NipartError> {
        let mut map: HashMap<String, zvariant::OwnedValue> =
            value.try_into().map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!("Invalid DBUS reply, expecting map, error: {e}"),
                )
            })?;

        Ok(Self {
            obj_path,
            bssid: _from_map!(map, "bssid", String::try_from)?
                .map(|b| b.to_uppercase()),
            ssid: _from_map!(map, "ssid", parse_ssid)?.ok_or_else(|| {
                NipartError::new(
                    ErrorKind::Bug,
                    "ssid does not exist in wpa_supplicant DBUS network query \
                     reply"
                        .to_string(),
                )
            })?,
            ieee80211w: _from_map!(map, "ieee80211w", parse_ieee80211w)?,
            key_mgmt: _from_map!(map, "key_mgmt", String::try_from)?,
            ..Default::default()
        })
    }

    pub(crate) fn to_value(&self) -> HashMap<&str, zvariant::Value<'_>> {
        let mut ret = HashMap::new();
        ret.insert("ssid", zvariant::Value::new(self.ssid.clone()));
        if let Some(v) = &self.psk {
            ret.insert("psk", zvariant::Value::new(v.clone()));
        }
        if let Some(v) = &self.sae_password {
            ret.insert("sae_password", zvariant::Value::new(v.clone()));
        }
        if let Some(v) = &self.bssid {
            ret.insert("bssid", zvariant::Value::new(v.to_lowercase()));
        }
        if let Some(v) = self.ieee80211w {
            ret.insert("ieee80211w", zvariant::Value::new(v));
        }
        if let Some(v) = &self.key_mgmt {
            ret.insert("key_mgmt", zvariant::Value::new(v.to_string()));
        }
        ret
    }

    pub(crate) fn change_to_wpa3_psk(&mut self) {
        self.ieee80211w = Some(2);
        self.key_mgmt = Some("SAE FT-SAE".to_string());
        self.sae_password = self.psk.take();
    }
}

fn parse_ssid(value: zvariant::OwnedValue) -> Result<String, NipartError> {
    let quoted = String::try_from(value).map_err(|e| {
        NipartError::new(
            ErrorKind::InvalidArgument,
            format!("Invalid SSID in wpa_supplicant network DBUS reply: {e}"),
        )
    })?;

    if let Some(s) = quoted.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
    {
        Ok(s.to_string())
    } else {
        Ok(quoted.to_string())
    }
}

fn parse_ieee80211w(value: zvariant::OwnedValue) -> Result<i32, NipartError> {
    let i32_str = String::try_from(value).map_err(|e| {
        NipartError::new(
            ErrorKind::InvalidArgument,
            format!(
                "Invalid ieee80211w in wpa_supplicant network DBUS reply: {e}"
            ),
        )
    })?;

    i32::from_str(&i32_str).map_err(|_| {
        NipartError::new(
            ErrorKind::Bug,
            format!(
                "Invalid wpa_supplicant DBUS reply of ieee80211w property: \
                 {i32_str}"
            ),
        )
    })
}
