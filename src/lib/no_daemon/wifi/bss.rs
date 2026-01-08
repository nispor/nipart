// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

use zvariant::OwnedObjectPath;

use crate::{ErrorKind, NipartError, WifiAuthType, WifiConfig};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WpaSupBss {
    pub(crate) obj_path: OwnedObjectPath,
    pub(crate) iface_name: String,
    pub(crate) ssid: Option<String>,
    pub(crate) bssid: Option<Vec<u8>>,
    pub(crate) mode: Option<String>,
    pub(crate) frequency_mhz: Option<u16>,
    pub(crate) signal_dbm: Option<i16>,
    pub(crate) wpa1: Option<WpaSupBssWpa1>,
    /// Robust Security Network defined by 802.11i, used for WPA2 and WPA3
    pub(crate) rsn: Option<WpaSupBssRsn>,
    pub(crate) ies: Option<Vec<u8>>,
    pub(crate) generation: Option<u32>,
}

impl WpaSupBss {
    pub(crate) fn from_value(
        mut map: HashMap<String, zvariant::OwnedValue>,
        obj_path: OwnedObjectPath,
    ) -> Result<Self, NipartError> {
        Ok(Self {
            obj_path,
            iface_name: String::new(),
            ssid: _from_map!(map, "SSID", parse_ssid)?,
            bssid: _from_map!(map, "BSSID", Vec::<u8>::try_from)?,
            mode: _from_map!(map, "Mode", String::try_from)?,
            frequency_mhz: _from_map!(map, "Frequency", u16::try_from)?,
            signal_dbm: _from_map!(map, "Signal", i16::try_from)?,
            wpa1: _from_map!(map, "WPA", WpaSupBssWpa1::try_from)?,
            rsn: _from_map!(map, "RSN", WpaSupBssRsn::try_from)?,
            ies: _from_map!(map, "IEs", Vec::<u8>::try_from)?,
            generation: None,
        })
    }

    pub(crate) fn is_wpa3_psk(&self) -> bool {
        self.get_auth_types().contains(&WifiAuthType::Wpa3Personal)
    }

    pub(crate) fn get_auth_types(&self) -> Vec<WifiAuthType> {
        let mut ret: HashSet<WifiAuthType> = HashSet::new();
        if let Some(mgmt_suits) = self
            .rsn
            .as_ref()
            .and_then(|r| r.key_management_suits.as_ref())
        {
            for mgmt_suit in mgmt_suits {
                ret.insert(match mgmt_suit.as_str() {
                    "wpa-psk" | "wpa-ft-psk" | "wpa-psk-sha256" => {
                        WifiAuthType::Wpa2Personal
                    }
                    "wpa-eap"
                    | "wpa-ft-eap"
                    | "wpa-eap-sha256"
                    | "wpa-eap-suite-b"
                    | "wpa-eap-suite-b-192"
                    | "wpa-eap-sha384" => WifiAuthType::Enterprise,
                    "wpa-fils-sha256" | "wpa-fils-sha384"
                    | "wpa-ft-fils-sha256" | "wpa-ft-fils-sha384" => {
                        WifiAuthType::Fils
                    }
                    "sae" | "sae-ext-key" | "ft-sae" | "ft-sae-ext-key" => {
                        WifiAuthType::Wpa3Personal
                    }
                    "owe" => WifiAuthType::Wpa3Open,
                    "wpa-none" => WifiAuthType::Open,
                    _ => continue,
                });
            }
        }
        ret.into_iter().collect()
    }
}

fn parse_ssid(value: zvariant::OwnedValue) -> Result<String, NipartError> {
    let bytes = Vec::<u8>::try_from(value).map_err(|e| {
        NipartError::new(
            ErrorKind::InvalidArgument,
            format!("Invalid SSID in wpa_supplicant BSS DBUS reply: {e}"),
        )
    })?;

    String::from_utf8(bytes).map_err(|e| {
        NipartError::new(
            ErrorKind::InvalidArgument,
            format!(
                "Invalid SSID in wpa_supplicant BSS DBUS reply, not UTF-8: {e}"
            ),
        )
    })
}

impl From<WpaSupBss> for WifiConfig {
    fn from(bss: WpaSupBss) -> WifiConfig {
        let mut ret = WifiConfig {
            ssid: bss.ssid.clone().unwrap_or_default(),
            frequency_mhz: bss.frequency_mhz.map(|f| f.into()),
            signal_dbm: bss.signal_dbm,
            bssid: bss.bssid.as_ref().map(|b| mac_to_string(b.as_slice())),
            auth_types: Some(bss.get_auth_types()),
            generation: bss.generation,
            ..Default::default()
        };
        ret.sanitize_signal();
        ret
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WpaSupBssWpa1 {
    pub(crate) key_management_suits: Option<Vec<String>>,
    pub(crate) pairwise_cipher_suits: Option<Vec<String>>,
    pub(crate) group_cipher: Option<String>,
}

impl TryFrom<zvariant::OwnedValue> for WpaSupBssWpa1 {
    type Error = NipartError;

    fn try_from(v: zvariant::OwnedValue) -> Result<Self, NipartError> {
        let error_msg = format!(
            "Expecting map for WPA(for WPA1) reply of BSS: but got {v:?}"
        );
        let mut map = HashMap::<String, zvariant::OwnedValue>::try_from(v)
            .map_err(|_| NipartError::new(ErrorKind::Bug, error_msg))?;
        Ok(Self {
            key_management_suits: _from_map!(
                map,
                "KeyMgmt",
                Vec::<String>::try_from
            )?,
            pairwise_cipher_suits: _from_map!(
                map,
                "Pairwise",
                Vec::<String>::try_from
            )?,
            group_cipher: _from_map!(map, "Group", String::try_from)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WpaSupBssRsn {
    pub(crate) key_management_suits: Option<Vec<String>>,
    pub(crate) pairwise_cipher_suits: Option<Vec<String>>,
    pub(crate) group_cipher: Option<String>,
    pub(crate) mgmt_group_cipher: Option<String>,
}

impl TryFrom<zvariant::OwnedValue> for WpaSupBssRsn {
    type Error = NipartError;

    fn try_from(v: zvariant::OwnedValue) -> Result<Self, NipartError> {
        let error_msg =
            format!("Expecting map for RSN reply of BSS: but got {v:?}");
        let mut map = HashMap::<String, zvariant::OwnedValue>::try_from(v)
            .map_err(|_| NipartError::new(ErrorKind::Bug, error_msg))?;
        Ok(Self {
            key_management_suits: _from_map!(
                map,
                "KeyMgmt",
                Vec::<String>::try_from
            )?,
            pairwise_cipher_suits: _from_map!(
                map,
                "Pairwise",
                Vec::<String>::try_from
            )?,
            group_cipher: _from_map!(map, "Group", String::try_from)?,
            mgmt_group_cipher: _from_map!(map, "MgmtGroup", String::try_from)?,
        })
    }
}

fn mac_to_string(data: &[u8]) -> String {
    let mut rt = String::new();
    for (i, m) in data.iter().enumerate() {
        write!(rt, "{m:02x}").ok();
        if i != data.len() - 1 {
            rt.push(':');
        }
    }
    rt
}
