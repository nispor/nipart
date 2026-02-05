// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original file is:
//  * Gris Ge <fge@redhat.com>
//  * Fernando Fernandez Mancera <ffmancera@riseup.net>
//  * Ales Musil <amusil@redhat.com>
//  * Quique Llorente <ellorent@redhat.com>
//  * Wen Liang <liangwen12year@gmail.com>
//  * Íñigo Huguet <ihuguet@redhat.com>

use serde::{Deserialize, Deserializer, Serialize};

use super::value::get_json_value_difference;
use crate::{
    BaseInterface, BondInterface, DummyInterface, ErrorKind, EthernetInterface,
    InterfaceState, InterfaceType, JsonDisplayHideSecrets,
    LinuxBridgeInterface, LoopbackInterface, NipartError, NipartstateInterface,
    OvsBridgeInterface, OvsInterface, UnknownInterface, VlanInterface,
    WifiCfgInterface, WifiPhyInterface, WireguardInterface,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonDisplayHideSecrets)]
#[serde(rename_all = "kebab-case", untagged)]
#[non_exhaustive]
/// Represent a kernel or user space network interface.
pub enum Interface {
    /// Ethernet interface.
    Ethernet(Box<EthernetInterface>),
    /// OVS Bridge
    OvsBridge(Box<OvsBridgeInterface>),
    /// OVS System Interface
    OvsInterface(Box<OvsInterface>),
    /// Loopback Interface
    Loopback(Box<LoopbackInterface>),
    /// WiFi Interface
    WifiPhy(Box<WifiPhyInterface>),
    /// Pseudo Interface for WiFi configuration
    WifiCfg(Box<WifiCfgInterface>),
    /// Dummy Interface
    Dummy(Box<DummyInterface>),
    /// VLAN Interface
    Vlan(Box<VlanInterface>),
    /// Bond Interface
    Bond(Box<BondInterface>),
    /// Linux Bridge Interface
    LinuxBridge(Box<LinuxBridgeInterface>),
    Wireguard(Box<WireguardInterface>),
    /// Unknown interface.
    Unknown(Box<UnknownInterface>),
}

impl Default for Interface {
    fn default() -> Self {
        Self::Unknown(Box::default())
    }
}

impl<'de> Deserialize<'de> for Interface {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut v = serde_json::Value::deserialize(deserializer)?;

        // It is safe to do `v["state"]` here as serde_json will
        // return `json!(null)` for undefined property
        if matches!(
            Option::deserialize(&v["state"])
                .map_err(serde::de::Error::custom)?,
            Some(InterfaceState::Absent)
        ) {
            // Ignore all properties except type if state: absent
            let mut new_value = serde_json::map::Map::new();
            if let Some(n) = v.get("name") {
                new_value.insert("name".to_string(), n.clone());
            }
            if let Some(t) = v.get("type") {
                new_value.insert("type".to_string(), t.clone());
            }
            if let Some(s) = v.get("state") {
                new_value.insert("state".to_string(), s.clone());
            }
            v = serde_json::value::Value::Object(new_value);
        }

        match Option::deserialize(&v["type"])
            .map_err(serde::de::Error::custom)?
        {
            Some(InterfaceType::Ethernet) | Some(InterfaceType::Veth) => {
                let inner = EthernetInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::Ethernet(Box::new(inner)))
            }
            Some(InterfaceType::OvsBridge) => {
                let inner = OvsBridgeInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::OvsBridge(Box::new(inner)))
            }
            Some(InterfaceType::OvsInterface) => {
                let inner = OvsInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::OvsInterface(Box::new(inner)))
            }
            Some(InterfaceType::Loopback) => {
                let inner = LoopbackInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::Loopback(Box::new(inner)))
            }
            Some(InterfaceType::WifiPhy) => {
                let inner = WifiPhyInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::WifiPhy(Box::new(inner)))
            }
            Some(InterfaceType::WifiCfg) => {
                let inner = WifiCfgInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::WifiCfg(Box::new(inner)))
            }
            Some(InterfaceType::Dummy) => {
                let inner = DummyInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::Dummy(Box::new(inner)))
            }
            Some(InterfaceType::Vlan) => {
                let inner = VlanInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::Vlan(Box::new(inner)))
            }
            Some(InterfaceType::Bond) => {
                let inner = BondInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::Bond(Box::new(inner)))
            }
            Some(InterfaceType::LinuxBridge) => {
                let inner = LinuxBridgeInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::LinuxBridge(Box::new(inner)))
            }
            Some(InterfaceType::Wireguard) => {
                let inner = WireguardInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::Wireguard(Box::new(inner)))
            }
            _ => {
                let inner = UnknownInterface::deserialize(v)
                    .map_err(serde::de::Error::custom)?;
                Ok(Interface::Unknown(Box::new(inner)))
            }
        }
    }
}

macro_rules! gen_sanitize_iface_specfic {
    ( $desired:ident, $current:ident, $($variant:path,)+ ) => {
        match $desired {
            $(
                $variant(i) => {
                    let cur_iface = if let Some($variant(c)) = $current {
                        Some(c)
                    } else {
                        if let Some(current) = $current {
                            return Err(NipartError::new(
                                ErrorKind::Bug,
                                format!(
                                    "current interface holding the same \
                                    interface type as as desired, current {}, \
                                    desired {}", i.iface_type(),
                                    current.iface_type(),
                                ),
                            ));
                        }
                        None
                    };
                    i.sanitize_iface_specfic(cur_iface.map(|v| &**v))
                }
            )+
        }
    };
}

macro_rules! gen_sanitize_before_verify_iface_specfic {
    ( $desired:ident, $current:ident, $($variant:path,)+ ) => {
        match $desired {
            $(
                $variant(i) => {
                    if let $variant(cur_iface) = $current {
                        i.sanitize_before_verify_iface_specfic(cur_iface);
                    };
                }
            )+
        }
    };
}

macro_rules! gen_include_diff_context_iface_specific {
    ( $diff:ident, $desired:ident, $current:ident, $($variant:path,)+ ) => {
        match ($diff, $desired, $current) {
            $(
                (
                    $variant(i),
                    $variant(desired),
                    $variant(current),
                ) => i.include_diff_context_iface_specific(desired, current),
            )+
            (_, desired, current) => {
                log::error!(
                    "BUG: Interface::include_diff_context_iface_specific() \
                     Unexpected desired {:?} current {:?}",
                     desired, current,
                );
            }
        }
    };
}

macro_rules! gen_include_revert_context_iface_specific{
    ( $revert:ident, $desired:ident, $pre_apply:ident, $($variant:path,)+ ) => {
        match ($revert, $desired, $pre_apply) {
            $(
                (
                    $variant(i),
                    $variant(desired),
                    $variant(pre_apply),
                ) => i.include_revert_context_iface_specific(
                    desired,
                    pre_apply,
                ),
            )+
            _ => {
                log::error!(
                    "BUG: Interface::include_revert_context_iface_specific() \
                     Unexpected input desired {:?} pre_apply {:?}",
                     $desired, $pre_apply
                );
            }
        }
    };
}

macro_rules! gen_post_merge_iface_specific{
    ( $merged:ident, $old:ident, $($variant:path,)+ ) => {
        match ($merged, $old) {
            $(
                ($variant(i), $variant(o)) => i.post_merge_iface_specific(o),
            )+
            (merged, old) => {
                log::error!(
                    "BUG: Interface::post_merge_iface_specific() \
                     Unexpected input merged {merged:?} old_state {old:?}"
                );
                Ok(())
            }
        }
    };
}

macro_rules! gen_need_delete_before_change {
    ( $desired:ident, $current:ident, $($variant:path,)+ ) => {
        match ($desired, $current) {
            $(
                ($variant(d), $variant(c)) => d.need_delete_before_change(c),
            )+
            (desired, current) => {
                log::error!(
                    "BUG: Interface::include_revert_context_iface_specific() \
                     Unexpected input merged {desired:?} old_state {current:?}"
                );
                false
            }
        }
    };
}

macro_rules! gen_iface_no_arg {
    ( $self:ident, $func:ident, $($variant:path,)+ ) => {
        match $self {
            $(
                $variant(i) => i.$func(),
            )+
        }
    };
}

macro_rules! gen_iface_trait_impl {
    ( $(($func:ident, $return:ty),)+ ) => {
        $(
            fn $func(&self) -> $return {
                gen_iface_no_arg!(
                    self,
                    $func,
                    Self::Ethernet,
                    Self::OvsBridge,
                    Self::OvsInterface,
                    Self::Loopback,
                    Self::WifiPhy,
                    Self::WifiCfg,
                    Self::Dummy,
                    Self::Vlan,
                    Self::Bond,
                    Self::LinuxBridge,
                    Self::Wireguard,
                    Self::Unknown,
                )
            }
        )+
    }
}

macro_rules! gen_iface_trait_impl_mut {
    ( $(($func:ident, $return:ty),)+ ) => {
        $(
            fn $func(&mut self) -> $return {
                gen_iface_no_arg!(
                    self,
                    $func,
                    Self::Ethernet,
                    Self::OvsBridge,
                    Self::OvsInterface,
                    Self::Loopback,
                    Self::WifiPhy,
                    Self::WifiCfg,
                    Self::Dummy,
                    Self::Vlan,
                    Self::Bond,
                    Self::LinuxBridge,
                    Self::Wireguard,
                    Self::Unknown,
                )
            }
        )+
    }
}

impl NipartstateInterface for Interface {
    gen_iface_trait_impl!(
        (is_virtual, bool),
        (base_iface, &BaseInterface),
        (is_controller, bool),
        (ports, Option<Vec<&str>>),
        (parent, Option<&str>),
    );

    gen_iface_trait_impl_mut!(
        (base_iface_mut, &mut BaseInterface),
        (hide_secrets_iface_specific, ()),
    );

    fn sanitize_iface_specfic(
        &mut self,
        current: Option<&Self>,
    ) -> Result<(), NipartError> {
        gen_sanitize_iface_specfic!(
            self,
            current,
            Interface::Ethernet,
            Interface::OvsBridge,
            Interface::OvsInterface,
            Interface::Loopback,
            Interface::WifiPhy,
            Interface::WifiCfg,
            Interface::Dummy,
            Interface::Vlan,
            Interface::Bond,
            Interface::LinuxBridge,
            Interface::Wireguard,
            Interface::Unknown,
        )
    }

    fn sanitize_before_verify_iface_specfic(&mut self, current: &mut Self) {
        gen_sanitize_before_verify_iface_specfic!(
            self,
            current,
            Interface::Ethernet,
            Interface::OvsBridge,
            Interface::OvsInterface,
            Interface::Loopback,
            Interface::WifiPhy,
            Interface::WifiCfg,
            Interface::Dummy,
            Interface::Vlan,
            Interface::Bond,
            Interface::LinuxBridge,
            Interface::Wireguard,
            Interface::Unknown,
        );
    }

    fn include_diff_context_iface_specific(
        &mut self,
        desired: &Self,
        current: &Self,
    ) {
        gen_include_diff_context_iface_specific!(
            self,
            desired,
            current,
            Interface::Ethernet,
            Interface::OvsBridge,
            Interface::OvsInterface,
            Interface::Loopback,
            Interface::WifiPhy,
            Interface::WifiCfg,
            Interface::Dummy,
            Interface::Vlan,
            Interface::Bond,
            Interface::LinuxBridge,
            Interface::Wireguard,
            Interface::Unknown,
        )
    }

    fn include_revert_context_iface_specific(
        &mut self,
        desired: &Self,
        pre_apply: &Self,
    ) {
        gen_include_revert_context_iface_specific!(
            self,
            desired,
            pre_apply,
            Interface::Ethernet,
            Interface::OvsBridge,
            Interface::OvsInterface,
            Interface::Loopback,
            Interface::WifiPhy,
            Interface::WifiCfg,
            Interface::Dummy,
            Interface::Vlan,
            Interface::Bond,
            Interface::LinuxBridge,
            Interface::Wireguard,
            Interface::Unknown,
        )
    }

    fn post_merge_iface_specific(
        &mut self,
        old_state: &Self,
    ) -> Result<(), NipartError> {
        gen_post_merge_iface_specific!(
            self,
            old_state,
            Interface::Ethernet,
            Interface::OvsBridge,
            Interface::OvsInterface,
            Interface::Loopback,
            Interface::WifiPhy,
            Interface::WifiCfg,
            Interface::Dummy,
            Interface::Vlan,
            Interface::Bond,
            Interface::LinuxBridge,
            Interface::Wireguard,
            Interface::Unknown,
        )
    }

    fn need_delete_before_change(&self, current: &Self) -> bool {
        gen_need_delete_before_change!(
            self,
            current,
            Interface::Ethernet,
            Interface::OvsBridge,
            Interface::OvsInterface,
            Interface::Loopback,
            Interface::WifiPhy,
            Interface::WifiCfg,
            Interface::Dummy,
            Interface::Vlan,
            Interface::Bond,
            Interface::LinuxBridge,
            Interface::Wireguard,
            Interface::Unknown,
        )
    }
}

impl From<BaseInterface> for Interface {
    fn from(base_iface: BaseInterface) -> Self {
        let mut iface = match &base_iface.iface_type {
            InterfaceType::Ethernet => Interface::Ethernet(Default::default()),
            InterfaceType::Hsr => todo!(),
            InterfaceType::Bond => Interface::Bond(Default::default()),
            InterfaceType::LinuxBridge => {
                Interface::LinuxBridge(Default::default())
            }
            InterfaceType::Dummy => Interface::Dummy(Default::default()),
            InterfaceType::Loopback => Interface::Loopback(Default::default()),
            InterfaceType::MacVlan => todo!(),
            InterfaceType::MacVtap => todo!(),
            InterfaceType::OvsBridge => {
                Interface::OvsBridge(Default::default())
            }
            InterfaceType::OvsInterface => {
                Interface::OvsInterface(Default::default())
            }
            InterfaceType::Veth => todo!(),
            InterfaceType::Vlan => Interface::Vlan(Default::default()),
            InterfaceType::Vrf => todo!(),
            InterfaceType::Vxlan => todo!(),
            InterfaceType::InfiniBand => todo!(),
            InterfaceType::Tun => todo!(),
            InterfaceType::MacSec => todo!(),
            InterfaceType::Ipsec => todo!(),
            InterfaceType::Xfrm => todo!(),
            InterfaceType::IpVlan => todo!(),
            InterfaceType::WifiPhy => Interface::WifiPhy(Default::default()),
            InterfaceType::WifiCfg => Interface::WifiCfg(Default::default()),
            InterfaceType::Wireguard => {
                Interface::Wireguard(Default::default())
            }
            InterfaceType::Unknown(_) => Interface::Unknown(Default::default()),
        };
        *iface.base_iface_mut() = base_iface;
        iface
    }
}

impl Interface {
    pub(crate) fn clone_name_type_only(&self) -> Self {
        self.base_iface().clone_name_type_only().into()
    }

    pub(crate) fn verify(&self, current: &Self) -> Result<(), NipartError> {
        let self_value = serde_json::to_value(self.clone())?;
        let current_value = serde_json::to_value(current.clone())?;

        if let Some((reference, desire, current)) = get_json_value_difference(
            format!("{}.interface", self.name()),
            &self_value,
            &current_value,
        ) {
            Err(NipartError::new(
                ErrorKind::VerificationError,
                format!(
                    "Verification failure: {reference} desire '{desire}', \
                     current '{current}'"
                ),
            ))
        } else {
            Ok(())
        }
    }
}
