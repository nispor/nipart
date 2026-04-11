// SPDX-License-Identifier: Apache-2.0

use serde::{
    Deserialize, Deserializer, Serialize, Serializer, ser::SerializeMap,
};

use crate::{
    Interface, InterfaceLinkState, InterfaceType, Interfaces, JsonDisplay,
    NipartInterface,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, JsonDisplay)]
#[non_exhaustive]
pub enum InterfaceTrigger {
    /// Just use `interface.state` value to bring interface up and down,
    /// No auto action will take afterwards.
    /// This is default value.
    #[default]
    OneShot,
    /// Use link carrier to up and down the interface. In order
    /// to keep tracking future carrier changes of interface, only purge IP
    /// stack when carrier down.
    /// Optional delay (in seconds) can be specified to debounce rapid
    /// up/down transitions.
    Carrier { delay: Option<u64> },
    /// Bring the interface up when connected to specified SSID.
    /// Bring the interface down when disconnected from specified SSID.
    /// String `*` means any SSID.
    /// Optional delay (in seconds) can be specified to debounce rapid
    /// up/down transitions.
    Wifi {
        ssid: Box<String>,
        delay: Option<u64>,
    },
    /// Bring the interface up when specified SSID disconnected.
    /// Bring the interface down when specified SSID connected.
    /// String `*` is not valid, should use `InterfaceTrigger::OneShot`
    /// with `state: down`.
    /// Optional delay (in seconds) can be specified to debounce rapid
    /// up/down transitions.
    WifiNot {
        ssid: Box<String>,
        delay: Option<u64>,
    },
}

impl InterfaceTrigger {
    /// Returns the debounce delay in seconds if configured.
    pub fn delay(&self) -> Option<u64> {
        match self {
            Self::Carrier { delay } => *delay,
            Self::Wifi { delay, .. } => *delay,
            Self::WifiNot { delay, .. } => *delay,
            _ => None,
        }
    }

    /// Returns true if this is a Carrier trigger (with or without delay).
    pub fn is_carrier(&self) -> bool {
        matches!(self, Self::Carrier { .. })
    }

    pub fn is_wifi(&self) -> bool {
        matches!(self, Self::Wifi { .. } | Self::WifiNot { .. })
    }

    /// Returns the SSID if this is a WiFi trigger.
    pub fn ssid(&self) -> Option<&str> {
        match self {
            Self::Wifi { ssid, .. } => Some(ssid.as_str()),
            Self::WifiNot { ssid, .. } => Some(ssid.as_str()),
            _ => None,
        }
    }

    /// Return `Some(true)` for interface should be up
    /// Return `Some(false)` for interface should be down
    /// Return None for interface not impacted.
    pub fn process(
        &self,
        iface_name: &str,
        iface_type: &InterfaceType,
        cur_ifaces: &Interfaces,
    ) -> Option<bool> {
        match self {
            Self::OneShot => None,
            Self::Carrier { .. } => {
                if let Some(cur_iface) =
                    cur_ifaces.get(iface_name, Some(iface_type))
                {
                    if let Some(link_state) =
                        cur_iface.base_iface().link_state.as_ref()
                    {
                        match link_state {
                            InterfaceLinkState::Up => Some(true),
                            InterfaceLinkState::Down => Some(false),
                            // No action required for other link state.
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    // interface been removed, no action required.
                    None
                }
            }
            Self::Wifi { ssid, .. } => {
                if cur_ifaces.iter().any(|cur_iface| {
                    if let Interface::WifiPhy(wifi_iface) = cur_iface {
                        wifi_iface.ssid() == Some(ssid.as_str())
                    } else {
                        false
                    }
                }) {
                    Some(true)
                } else {
                    Some(false)
                }
            }
            Self::WifiNot { ssid, .. } => {
                if cur_ifaces.iter().any(|cur_iface| {
                    if let Interface::WifiPhy(wifi_iface) = cur_iface {
                        wifi_iface.ssid() == Some(ssid.as_str())
                    } else {
                        false
                    }
                }) {
                    Some(false)
                } else {
                    Some(true)
                }
            }
        }
    }
}

impl<'de> Deserialize<'de> for InterfaceTrigger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let error_msg_prefix = "Expecting 'oneshot', 'carrier', 'wifi', \
                                'wifi-not', for interface trigger";

        let v = serde_json::Value::deserialize(deserializer)?;
        if let Some(obj) = v.as_object() {
            if let Some(v) = obj.get("wifi") {
                let (ssid, delay) = parse_wifi_value::<D>(v)?;
                Ok(Self::Wifi {
                    ssid: Box::new(ssid),
                    delay,
                })
            } else if let Some(v) = obj.get("wifi-not") {
                let (ssid, delay) = parse_wifi_value::<D>(v)?;
                Ok(Self::WifiNot {
                    ssid: Box::new(ssid),
                    delay,
                })
            } else if let Some(v) = obj.get("carrier") {
                let delay = parse_carrier_value::<D>(v)?;
                Ok(Self::Carrier { delay })
            } else {
                Err(serde::de::Error::custom(format!(
                    "{error_msg_prefix}, but got {}",
                    obj.keys()
                        .map(|k| k.as_str())
                        .collect::<Vec<&str>>()
                        .join(" ")
                )))
            }
        } else if let Some(obj_str) = v.as_str() {
            match obj_str {
                "carrier" => Ok(Self::Carrier { delay: None }),
                "one-shot" => Ok(Self::OneShot),
                v => Err(serde::de::Error::custom(format!(
                    "{error_msg_prefix}, but got {v}",
                ))),
            }
        } else {
            Err(serde::de::Error::custom(format!(
                "{error_msg_prefix}, but got not string or map",
            )))
        }
    }
}

fn parse_wifi_value<'de, D>(
    v: &serde_json::Value,
) -> Result<(String, Option<u64>), D::Error>
where
    D: Deserializer<'de>,
{
    if let Some(obj) = v.as_object() {
        let ssid = obj
            .get("ssid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                serde::de::Error::custom(
                    "WiFi trigger requires 'ssid' field".to_string(),
                )
            })?
            .to_string();
        let delay = obj.get("delay").and_then(|v| v.as_u64());
        Ok((ssid, delay))
    } else if let Some(s) = v.as_str() {
        Ok((s.to_string(), None))
    } else {
        Err(serde::de::Error::custom(
            "WiFi trigger value should be a string or object with 'ssid' and \
             optional 'delay'"
                .to_string(),
        ))
    }
}

fn parse_carrier_value<'de, D>(
    v: &serde_json::Value,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    if let Some(delay) = v.as_u64() {
        Ok(Some(delay))
    } else if v.is_null() {
        Ok(None)
    } else {
        Err(serde::de::Error::custom(
            "Carrier value should be a number or null".to_string(),
        ))
    }
}

impl Serialize for InterfaceTrigger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::OneShot => serializer.serialize_str("one-shot"),
            Self::Carrier { delay } => {
                if let Some(delay) = delay {
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry("carrier", delay)?;
                    map.end()
                } else {
                    serializer.serialize_str("carrier")
                }
            }
            Self::Wifi { ssid, delay } => {
                let mut map = serializer.serialize_map(Some(1))?;
                if let Some(delay) = delay {
                    let mut wifi_obj = serde_json::Map::new();
                    wifi_obj.insert(
                        "ssid".to_string(),
                        serde_json::Value::String(ssid.to_string()),
                    );
                    wifi_obj.insert(
                        "delay".to_string(),
                        serde_json::Value::Number((*delay).into()),
                    );
                    map.serialize_entry(
                        "wifi",
                        &serde_json::Value::Object(wifi_obj),
                    )?;
                } else {
                    map.serialize_entry("wifi", ssid)?;
                }
                map.end()
            }
            Self::WifiNot { ssid, delay } => {
                let mut map = serializer.serialize_map(Some(1))?;
                if let Some(delay) = delay {
                    let mut wifi_obj = serde_json::Map::new();
                    wifi_obj.insert(
                        "ssid".to_string(),
                        serde_json::Value::String(ssid.to_string()),
                    );
                    wifi_obj.insert(
                        "delay".to_string(),
                        serde_json::Value::Number((*delay).into()),
                    );
                    map.serialize_entry(
                        "wifi-not",
                        &serde_json::Value::Object(wifi_obj),
                    )?;
                } else {
                    map.serialize_entry("wifi-not", ssid)?;
                }
                map.end()
            }
        }
    }
}
