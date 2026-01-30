// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{
    BaseInterface, ErrorKind, InterfaceType, JsonDisplay,
    JsonDisplayHideSecrets, NipartError, NipartstateInterface,
    nmstate::{
        deserializer::{number_as_string, option_number_as_string},
        value::copy_undefined_value,
    },
};

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonDisplayHideSecrets,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
/// WiFi physical interface
pub struct WifiPhyInterface {
    #[serde(flatten)]
    pub base: BaseInterface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifi: Option<WifiConfig>,
}

impl WifiPhyInterface {
    pub fn new(name: String, wifi: WifiConfig) -> Self {
        Self {
            base: BaseInterface {
                name: name.to_string(),
                iface_type: InterfaceType::WifiPhy,
                ..Default::default()
            },
            wifi: Some(wifi),
        }
    }
}

impl Default for WifiPhyInterface {
    fn default() -> Self {
        Self {
            base: BaseInterface {
                iface_type: InterfaceType::WifiPhy,
                ..Default::default()
            },
            wifi: None,
        }
    }
}

impl NipartstateInterface for WifiPhyInterface {
    fn base_iface(&self) -> &BaseInterface {
        &self.base
    }

    fn base_iface_mut(&mut self) -> &mut BaseInterface {
        &mut self.base
    }

    fn is_virtual(&self) -> bool {
        false
    }

    fn sanitize_iface_specfic(
        &mut self,
        _current: Option<&Self>,
    ) -> Result<(), NipartError> {
        let iface_name = self.name().to_string();
        if let Some(wifi_cfg) = self.wifi.as_mut() {
            wifi_cfg.sanitize();
            if let Some(base_iface_name) = wifi_cfg.base_iface.as_ref()
                && base_iface_name != &iface_name
            {
                return Err(NipartError::new(
                    ErrorKind::InvalidArgument,
                    format!(
                        "The wifi-phy interface {iface_name} is holding WIFI \
                         configuration with base-iface pointing to other \
                         interface {base_iface_name}",
                    ),
                ));
            } else {
                wifi_cfg.base_iface = Some(iface_name);
            }
        }
        Ok(())
    }

    fn hide_secrets_iface_specific(&mut self) {
        if let Some(wifi_cfg) = self.wifi.as_mut() {
            wifi_cfg.hide_secrets();
        }
    }

    /// Always include SSID if changed
    fn include_diff_context_iface_specific(
        &mut self,
        desired: &Self,
        current: &Self,
    ) {
        if let Some(desired_wifi) = desired.wifi.as_ref()
            && let Some(current_wifi) = current.wifi.as_ref()
            && !desired_wifi.ssid.is_empty()
            && desired_wifi != current_wifi
        {
            self.wifi = Some(WifiConfig {
                ssid: desired_wifi.ssid.to_string(),
                ..Default::default()
            });
        }
    }

    fn post_merge_iface_specific(
        &mut self,
        old: &Self,
    ) -> Result<(), NipartError> {
        if let Some(wifi) = self.wifi.as_mut()
            && let Some(old_wifi) = old.wifi.as_ref()
        {
            wifi.post_merge(old_wifi);
        }
        Ok(())
    }
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    JsonDisplay,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub enum WifiState {
    /// BSS disconnected
    Disconnected,
    /// Scanning SSID
    Scanning,
    /// SSID Found, trying to associate and authenticate with a BSS/SSID
    Connecting,
    /// Data connection is fully configured
    Completed,
    #[default]
    Unknown,
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Default,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonDisplay,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub enum WifiAuthType {
    /// No authentication
    #[serde(rename = "OPEN")]
    Open,
    /// WEP (depreacated)
    #[serde(rename = "WEP")]
    Wep,
    /// WPA 1(deprecated)
    #[serde(rename = "WPA1")]
    Wpa1,
    /// WPS(deprecated)
    #[serde(rename = "WPS")]
    Wps,
    /// WPA 2 Pre-share Key
    #[serde(rename = "WPA2-PSK")]
    Wpa2Personal,
    /// WPA 2/3 EAP(Extensible Authentication Protocol)
    /// Including OSEN(OSU Server-Only Authenticated L2 Encryption Network)
    #[serde(rename = "EAP")]
    Enterprise,
    /// WPA 3 Pre-share key using SAE(Simultaneous Authentication of Equals)
    #[serde(rename = "WPA3-PSK")]
    Wpa3Personal,
    /// WPA 3 open network using OWE(Opportunistic Wireless Encryption)
    #[serde(rename = "WPA3-OPEN")]
    Wpa3Open,
    /// IEEE 802.11ai -- Fast Initial Link Setup
    #[serde(rename = "FILS")]
    Fils,
    /// Device Provisioning Protoco, also known as Easy Connect.
    #[serde(rename = "DPP")]
    Dpp,
    #[default]
    Unknown,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonDisplayHideSecrets,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
/// Pseudo Interface for WiFi Configuration
pub struct WifiCfgInterface {
    #[serde(flatten)]
    pub base: BaseInterface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifi: Option<WifiConfig>,
}

impl WifiCfgInterface {
    pub fn new(base: BaseInterface) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }

    pub fn parent(&self) -> Option<&str> {
        self.wifi.as_ref().and_then(|w| w.base_iface.as_deref())
    }
}

impl From<WifiConfig> for WifiCfgInterface {
    fn from(config: WifiConfig) -> Self {
        Self {
            base: BaseInterface::new(
                config.ssid.to_string(),
                InterfaceType::WifiCfg,
            ),
            wifi: Some(config),
        }
    }
}

impl Default for WifiCfgInterface {
    fn default() -> Self {
        Self {
            base: BaseInterface {
                iface_type: InterfaceType::WifiCfg,
                ..Default::default()
            },
            wifi: None,
        }
    }
}

impl NipartstateInterface for WifiCfgInterface {
    fn base_iface(&self) -> &BaseInterface {
        &self.base
    }

    fn base_iface_mut(&mut self) -> &mut BaseInterface {
        &mut self.base
    }

    fn is_virtual(&self) -> bool {
        true
    }

    fn sanitize_iface_specfic(
        &mut self,
        _current: Option<&Self>,
    ) -> Result<(), NipartError> {
        if let Some(wifi_cfg) = self.wifi.as_mut() {
            wifi_cfg.sanitize();
        }
        Ok(())
    }

    fn hide_secrets_iface_specific(&mut self) {
        if let Some(wifi_cfg) = self.wifi.as_mut() {
            wifi_cfg.hide_secrets();
        }
    }

    fn sanitize_before_verify_iface_specfic(&mut self, current: &mut Self) {
        // The IP stack and WIFI password is for daemon storage only and cannot
        // be query during applying(only stored after apply succeeded), hence
        // we remove ipv4 and ipv6 for `wifi-cfg` interface.
        self.base.ipv4 = None;
        self.base.ipv6 = None;
        if let Some(wifi_cfg) = self.wifi.as_mut() {
            wifi_cfg.remove_secrets();
        }
        current.hide_secrets_iface_specific()
    }

    /// Always include SSID if changed
    fn include_diff_context_iface_specific(
        &mut self,
        desired: &Self,
        current: &Self,
    ) {
        if let Some(desired_wifi) = desired.wifi.as_ref()
            && let Some(current_wifi) = current.wifi.as_ref()
            && !desired_wifi.ssid.is_empty()
            && desired_wifi != current_wifi
        {
            self.wifi = Some(WifiConfig {
                ssid: desired_wifi.ssid.to_string(),
                ..Default::default()
            });
        }
    }

    fn post_merge_iface_specific(
        &mut self,
        old: &Self,
    ) -> Result<(), NipartError> {
        if let Some(wifi) = self.wifi.as_mut()
            && let Some(old_wifi) = old.wifi.as_ref()
        {
            wifi.post_merge(old_wifi);
        }
        Ok(())
    }
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    JsonDisplayHideSecrets,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct WifiConfig {
    /// SSID (Service Set Identifier)
    #[serde(default, deserialize_with = "number_as_string")]
    pub ssid: String,
    /// WiFi state. For query only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<WifiState>,
    /// Authentication type
    /// When querying network state, it only contains single value for current
    /// authentication type.
    /// When showing WIFI scan results, it contains the authentication types
    /// supported by AP.
    /// Ignored when applying.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_types: Option<Vec<WifiAuthType>>,
    /// WiFi generation, e.g. 6 for WiFi-6. For query only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<u32>,
    /// BSSID (Basic Service Set Identifier), if defined, will only connect to
    /// desired AP.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bssid: Option<String>,
    /// Password for authentication
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "option_number_as_string"
    )]
    pub password: Option<String>,
    /// Whether this WiFi configuration only for specified interface or not.
    /// If undefined, it means any WiFi network interface can be used for
    /// connecting this WiFi.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_iface: Option<String>,
    /// WiFi frequency in MHz. For query only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_mhz: Option<u32>,
    /// Receive bitrate in 1mb/s. For query only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx_bitrate_mb: Option<u32>,
    /// Transmit bitrate in 1mb/s. For query only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_bitrate_mb: Option<u32>,
    /// Signal in dBm. For query only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_dbm: Option<i16>,
    /// Signal in percentage. For query only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_percent: Option<u8>,
}

// Align with Microsoft `WLAN_ASSOCIATION_ATTRIBUTES`
const NOISE_FLOOR_DBM: i16 = -100;
const SIGNAL_MAX_DBM: i16 = -50;

impl WifiConfig {
    /// Use `signal_dbm` to calculate out `signal_percent`
    pub fn sanitize_signal(&mut self) {
        if let Some(s) = self.signal_dbm {
            self.signal_percent = Some(Self::signal_dbm_to_percent(s));
        }
    }

    pub fn signal_dbm_to_percent(dbm: i16) -> u8 {
        let dbm = dbm.clamp(NOISE_FLOOR_DBM, SIGNAL_MAX_DBM);
        (100.0f64 * (NOISE_FLOOR_DBM - dbm) as f64
            / (NOISE_FLOOR_DBM - SIGNAL_MAX_DBM) as f64) as u8
    }

    /// Set all query only properties to None
    /// Change BSSID to upper case
    pub(crate) fn sanitize(&mut self) {
        self.state = None;
        self.generation = None;
        self.frequency_mhz = None;
        self.tx_bitrate_mb = None;
        self.rx_bitrate_mb = None;
        self.signal_dbm = None;
        self.signal_percent = None;
        if let Some(mac) = self.bssid.as_ref() {
            self.bssid = Some(mac.to_uppercase());
        }
    }

    pub fn hide_secrets(&mut self) {
        if self.password.is_some() {
            self.password =
                Some(crate::NetworkState::HIDE_PASSWORD_STR.to_string());
        }
    }

    pub fn remove_secrets(&mut self) {
        self.password = None;
    }

    pub(crate) fn merge(&self, new: &Self) -> Result<Self, NipartError> {
        let mut new_value = serde_json::to_value(new)?;
        let old_value = serde_json::to_value(self)?;
        copy_undefined_value(&mut new_value, &old_value);

        let mut merged: Self = serde_json::from_value(new_value)?;
        merged.post_merge(self);

        Ok(merged)
    }

    pub(crate) fn post_merge(&mut self, old: &Self) {
        if self.ssid.is_empty() {
            self.ssid = old.ssid.to_string();
        }
    }
}

impl std::fmt::Debug for WifiConfig {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", WifiConfigHideSecrets::from(self))
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct WifiConfigHideSecrets {
    state: Option<WifiState>,
    generation: Option<u32>,
    ssid: String,
    bssid: Option<String>,
    password: Option<String>,
    auth_types: Option<Vec<WifiAuthType>>,
    base_iface: Option<String>,
    frequency_mhz: Option<u32>,
    rx_bitrate_mb: Option<u32>,
    tx_bitrate_mb: Option<u32>,
    signal_dbm: Option<i16>,
    signal_percent: Option<u8>,
}

impl From<&WifiConfig> for WifiConfigHideSecrets {
    fn from(v: &WifiConfig) -> Self {
        let WifiConfig {
            ssid,
            bssid,
            password,
            auth_types,
            base_iface,
            state,
            generation,
            frequency_mhz,
            rx_bitrate_mb,
            tx_bitrate_mb,
            signal_dbm,
            signal_percent,
        } = v.clone();
        Self {
            password: if password.is_some() {
                Some(crate::NetworkState::HIDE_PASSWORD_STR.to_string())
            } else {
                None
            },
            ssid,
            bssid,
            base_iface,
            auth_types,
            state,
            generation,
            frequency_mhz,
            rx_bitrate_mb,
            tx_bitrate_mb,
            signal_dbm,
            signal_percent,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Interface, NetworkState};

    #[test]
    fn test_hide_secrets_in_debug_wificfg() {
        let wifi_cfg = WifiConfig {
            ssid: "test-wifi".into(),
            password: Some("12345678".into()),
            ..Default::default()
        };
        let debug_str = format!("{:?}", wifi_cfg);
        println!("Debug string {:?}", debug_str);
        assert!(!debug_str.contains("12345678"));
    }

    #[test]
    fn test_hide_secrets_in_display_wificfg() {
        let wifi_cfg = WifiConfig {
            ssid: "test-wifi".into(),
            password: Some("12345678".into()),
            ..Default::default()
        };
        let debug_str = format!("{}", wifi_cfg);
        println!("Display string {}", debug_str);
        assert!(!debug_str.contains("12345678"));
    }

    #[test]
    fn test_hide_secrets_in_display_wifiiface() {
        let wifi_iface = WifiPhyInterface {
            base: Default::default(),
            wifi: Some(WifiConfig {
                ssid: "test-wifi".into(),
                password: Some("12345678".into()),
                ..Default::default()
            }),
        };
        let debug_str = format!("{}", wifi_iface);
        println!("Display string {}", debug_str);
        assert!(!debug_str.contains("12345678"));
    }

    #[test]
    fn test_hide_secrets_in_display_iface() {
        let iface = Interface::WifiPhy(Box::new(WifiPhyInterface {
            base: Default::default(),
            wifi: Some(WifiConfig {
                ssid: "test-wifi".into(),
                password: Some("12345678".into()),
                ..Default::default()
            }),
        }));
        let debug_str = format!("{}", iface);
        println!("Display string {}", debug_str);
        assert!(!debug_str.contains("12345678"));
    }

    #[test]
    fn test_hide_secrets_in_display_net_state() {
        let iface = Interface::WifiPhy(Box::new(WifiPhyInterface {
            base: Default::default(),
            wifi: Some(WifiConfig {
                ssid: "test-wifi".into(),
                password: Some("12345678".into()),
                ..Default::default()
            }),
        }));
        let mut net_state = NetworkState::new();
        net_state.ifaces.push(iface);
        let debug_str = format!("{}", net_state);
        println!("Display string {}", debug_str);
        assert!(!debug_str.contains("12345678"));
    }
}
