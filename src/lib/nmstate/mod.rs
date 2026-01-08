// SPDX-License-Identifier: Apache-2.0

mod gen_diff;
mod iface;
mod iface_state;
mod iface_trait;
mod iface_type;
mod ifaces;
mod ip;
mod merged;
mod net_state;
mod revert;
mod route;
mod state_options;
mod value;
mod version;

#[allow(dead_code)]
pub(crate) mod deserializer;
#[allow(dead_code)]
pub(crate) mod serializer;

pub use self::{
    iface::Interface,
    iface_state::InterfaceState,
    iface_trait::NipartstateInterface,
    iface_type::InterfaceType,
    ifaces::{
        BaseInterface, BondAdSelect, BondAllPortActive, BondArpAllTargets,
        BondArpValidate, BondConfig, BondFailOverMac, BondInterface,
        BondLacpRate, BondMode, BondOptions, BondPortConfig,
        BondPrimaryReselect, BondXmitHashPolicy, BridgeVlanConfig,
        BridgeVlanMode, BridgeVlanRange, BridgeVlanTrunkTag, DummyInterface,
        EthernetConfig, EthernetDuplex, EthernetInterface, Interfaces,
        LinuxBridgeConfig, LinuxBridgeInterface,
        LinuxBridgeMulticastRouterType, LinuxBridgeOptions,
        LinuxBridgePortConfig, LinuxBridgeStpOptions, LoopbackInterface,
        OvsBridgeConfig, OvsBridgeInterface, OvsBridgePortConfig, OvsInterface,
        UnknownInterface, VethConfig, VlanConfig, VlanInterface, VlanProtocol,
        VlanQosMapping, VlanRegistrationProtocol, WifiAuthType,
        WifiCfgInterface, WifiConfig, WifiPhyInterface, WifiState,
    },
    ip::{DhcpState, InterfaceIpAddr, InterfaceIpv4, InterfaceIpv6},
    merged::{
        MergedInterface, MergedInterfaces, MergedNetworkState, MergedRoutes,
    },
    net_state::NetworkState,
    route::{RouteEntry, RouteState, RouteType, Routes},
    state_options::{NipartstateApplyOption, NipartstateQueryOption, NipartstateStateKind},
    version::CUR_SCHEMA_VERSION,
};

#[cfg(test)]
mod unit_tests;
