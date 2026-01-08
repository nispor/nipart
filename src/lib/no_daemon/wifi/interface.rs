// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use zvariant::OwnedObjectPath;

use crate::{ErrorKind, NipartError, WifiAuthType, WifiState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WpaSupInterfaceState {
    Disconnected,
    Inactive,
    Scanning,
    Authenticating,
    Associating,
    Associated,
    FourWayHandshake,
    GroupHandshake,
    Completed,
    #[default]
    Unknown,
}

impl From<String> for WpaSupInterfaceState {
    fn from(s: String) -> Self {
        match s.as_str() {
            "disconnected" => Self::Disconnected,
            "inactive" => Self::Inactive,
            "scanning" => Self::Scanning,
            "authenticating" => Self::Authenticating,
            "associating" => Self::Associating,
            "associated" => Self::Associated,
            "4way_handshake" => Self::FourWayHandshake,
            "group_handshake" => Self::GroupHandshake,
            "completed" => Self::Completed,
            "unknown" => Self::Unknown,
            _ => Self::Unknown,
        }
    }
}

impl From<WpaSupInterfaceState> for WifiState {
    fn from(v: WpaSupInterfaceState) -> Self {
        match v {
            WpaSupInterfaceState::Disconnected
            | WpaSupInterfaceState::Inactive => Self::Disconnected,
            WpaSupInterfaceState::Scanning => Self::Scanning,
            WpaSupInterfaceState::Authenticating
            | WpaSupInterfaceState::Associating
            | WpaSupInterfaceState::Associated
            | WpaSupInterfaceState::FourWayHandshake
            | WpaSupInterfaceState::GroupHandshake => Self::Connecting,
            WpaSupInterfaceState::Completed => Self::Completed,
            WpaSupInterfaceState::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WpaSupInterface {
    pub(crate) obj_path: OwnedObjectPath,
    pub(crate) iface_name: String,
    pub(crate) state: WpaSupInterfaceState,
    pub(crate) cur_auth_mode: Option<String>,
}

impl WpaSupInterface {
    pub(crate) fn new(iface_name: String) -> Self {
        Self {
            iface_name,
            obj_path: OwnedObjectPath::default(),
            state: WpaSupInterfaceState::Unknown,
            cur_auth_mode: None,
        }
    }

    pub(crate) fn to_value(&self) -> HashMap<&str, zvariant::Value<'_>> {
        let mut ret = HashMap::new();
        ret.insert("Ifname", zvariant::Value::new(self.iface_name.clone()));
        ret
    }

    pub(crate) fn from_value(
        mut map: HashMap<String, zvariant::OwnedValue>,
        obj_path: OwnedObjectPath,
    ) -> Result<Self, NipartError> {
        Ok(Self {
            iface_name: _from_map!(map, "Ifname", String::try_from)?
                .ok_or_else(|| {
                    NipartError::new(
                        ErrorKind::Bug,
                        format!(
                            "Ifname does not exist in wpa_spplicant DBUS \
                             interface reply in {:?}",
                            map
                        ),
                    )
                })?,
            state: _from_map!(map, "State", String::try_from)?
                .map(WpaSupInterfaceState::from)
                .unwrap_or_default(),
            cur_auth_mode: _from_map!(
                map,
                "CurrentAuthMode",
                String::try_from
            )?,
            obj_path,
        })
    }

    pub(crate) fn get_cur_auth_mode(&self) -> Option<WifiAuthType> {
        match self.cur_auth_mode.as_deref()? {
            "WPA2-PSK+WPA-PSK" | "WPA2-PSK" | "FT-PSK" | "WPA2-PSK-SHA256" => {
                Some(WifiAuthType::Wpa2Personal)
            }
            "WPA-PSK" => Some(WifiAuthType::Wpa1),
            "NONE" => Some(WifiAuthType::Open),
            "WPA-NONE" => Some(WifiAuthType::Open),
            "FT-EAP"
            | "WPA2-EAP-SHA256"
            | "OSEN"
            | "WPA2-EAP-SUITE-B"
            | "WPA2-EAP-SUITE-B-192"
            | "FT-EAP-SHA384"
            | "WPA2-EAP-SHA384" => Some(WifiAuthType::Enterprise),
            "SAE" | "SAE-EXT-KEY" | "FT-SAE" | "FT-SAE-EXT-KEY" => {
                Some(WifiAuthType::Wpa3Personal)
            }
            "FILS-SHA256" | "FILS-SHA384" | "FT-FILS-SHA256"
            | "FT-FILS-SHA384" => Some(WifiAuthType::Fils),
            "OWE" => Some(WifiAuthType::Wpa3Open),
            "WPS" => Some(WifiAuthType::Wps),
            "DPP" => Some(WifiAuthType::Dpp),
            auth if auth.starts_with("EAP-") => Some(WifiAuthType::Enterprise),
            "INACTIVE" => None,
            auth => {
                log::warn!(
                    "Unknown wpa_supplicant authentication method {auth}"
                );
                None
            }
        }
    }
}
