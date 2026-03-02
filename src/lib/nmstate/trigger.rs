// SPDX-License-Identifier: Apache-2.0

use serde::{
    Deserialize, Deserializer, Serialize, Serializer, ser::SerializeMap,
};

use crate::{
    Interface, InterfaceLinkState, InterfaceType, Interfaces, JsonDisplay,
    NmstateInterface,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, JsonDisplay)]
#[non_exhaustive]
pub enum InterfaceTrigger {
    /// Just use `interface.state` value to bring interface up and down,
    /// No auto action will take afterwards.
    /// This is default value.
    #[default]
    OneShot,
    // TODO: Support delay for up/down action to prevent flipping.
    /// Use link carrier to up and down the interface. In order
    /// to keep tracking future carrier changes of interface, only purge IP
    /// stack when carrier down.
    Carrier,
    /// Bring the interface up when connected to specified SSID.
    /// Bring the interface down when disconnected from specified SSID.
    /// String `*` means any SSID.
    Wifi(Box<String>),
    /// Bring the interface up when specified SSID disconnected.
    /// Bring the interface down when specified SSID connected.
    /// String `*` is not valid, should use `InterfaceTrigger::OneShot`
    /// with `state: down`.
    WifiNot(Box<String>),
}

impl InterfaceTrigger {
    pub fn is_wifi(&self) -> bool {
        matches!(self, Self::Wifi(_) | Self::WifiNot(_))
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
            Self::Carrier => {
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
            Self::Wifi(ssid) => {
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
            Self::WifiNot(ssid) => {
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
                Ok(Self::Wifi(Box::new(
                    <String>::deserialize(v)
                        .map_err(|e| serde::de::Error::custom(e.to_string()))?,
                )))
            } else if let Some(v) = obj.get("wifi-not") {
                Ok(Self::WifiNot(Box::new(
                    <String>::deserialize(v)
                        .map_err(|e| serde::de::Error::custom(e.to_string()))?,
                )))
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
                "carrier" => Ok(Self::Carrier),
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

impl Serialize for InterfaceTrigger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::OneShot => serializer.serialize_str("one-shot"),
            Self::Carrier => serializer.serialize_str("carrier"),
            Self::Wifi(v) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("wifi", v)?;
                map.end()
            }
            Self::WifiNot(v) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("wifi-not", v)?;
                map.end()
            }
        }
    }
}
