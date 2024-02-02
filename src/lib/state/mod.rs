// SPDX-License-Identifier: MIT

mod deserializer;
mod dispatch;
mod dns;
mod hostname;
mod ieee8021x;
mod iface;
mod ifaces;
mod ip;
mod json;
mod lldp;
mod mptcp;
mod net_state;
#[allow(deprecated)]
mod ovn;
mod ovs;
mod query_apply;
mod revert;
mod route;
mod route_rule;
mod serializer;
// This one is not copy from nmstate
mod not_synced;

pub use self::dispatch::DispatchConfig;
pub use self::dns::{DnsClientState, DnsState, MergedDnsState};
pub use self::hostname::{HostNameState, MergedHostNameState};
pub use self::ieee8021x::Ieee8021XConfig;
pub use self::iface::{
    Interface, InterfaceIdentifier, InterfaceState, InterfaceType,
    MergedInterface, UnknownInterface,
};
pub use self::ifaces::{
    BaseInterface, BondAdSelect, BondAllPortsActive, BondArpAllTargets,
    BondArpValidate, BondConfig, BondFailOverMac, BondInterface, BondLacpRate,
    BondMode, BondOptions, BondPortConfig, BondPrimaryReselect,
    BondXmitHashPolicy, BridgePortTrunkTag, BridgePortVlanConfig,
    BridgePortVlanMode, BridgePortVlanRange, DummyInterface, EthernetConfig,
    EthernetDuplex, EthernetInterface, EthtoolCoalesceConfig, EthtoolConfig,
    EthtoolFeatureConfig, EthtoolPauseConfig, EthtoolRingConfig, HsrConfig,
    HsrInterface, InfiniBandConfig, InfiniBandInterface, InfiniBandMode,
    Interfaces, IpsecInterface, LibreswanConfig, LinuxBridgeConfig,
    LinuxBridgeInterface, LinuxBridgeMulticastRouterType, LinuxBridgeOptions,
    LinuxBridgePortConfig, LinuxBridgeStpOptions, LoopbackInterface,
    MacSecConfig, MacSecInterface, MacSecValidate, MacVlanConfig,
    MacVlanInterface, MacVlanMode, MacVtapConfig, MacVtapInterface,
    MacVtapMode, MergedInterfaces, OvsBridgeBondConfig, OvsBridgeBondMode,
    OvsBridgeBondPortConfig, OvsBridgeConfig, OvsBridgeInterface,
    OvsBridgeOptions, OvsBridgePortConfig, OvsBridgeStpOptions, OvsDpdkConfig,
    OvsInterface, OvsPatchConfig, SrIovConfig, SrIovVfConfig, VethConfig,
    VlanConfig, VlanInterface, VlanProtocol, VlanRegistrationProtocol,
    VrfConfig, VrfInterface, VxlanConfig, VxlanInterface, XfrmInterface,
};
pub use self::ip::{
    AddressFamily, Dhcpv4ClientId, Dhcpv6Duid, InterfaceIpAddr, InterfaceIpv4,
    InterfaceIpv6, Ipv6AddrGenMode, WaitIp,
};
pub use self::lldp::{
    LldpAddressFamily, LldpChassisId, LldpChassisIdType, LldpConfig,
    LldpMacPhy, LldpMaxFrameSize, LldpMgmtAddr, LldpMgmtAddrs, LldpNeighborTlv,
    LldpPortId, LldpPortIdType, LldpPpvids, LldpSystemCapabilities,
    LldpSystemCapability, LldpSystemDescription, LldpSystemName, LldpVlan,
    LldpVlans,
};
pub use self::mptcp::{MptcpAddressFlag, MptcpConfig};
pub use self::net_state::{MergedNetworkState, NetworkState};
pub use self::ovn::{
    MergedOvnConfiguration, OvnBridgeMapping, OvnBridgeMappingState,
    OvnConfiguration,
};
pub use self::ovs::{
    MergedOvsDbGlobalConfig, OvsDbGlobalConfig, OvsDbIfaceConfig,
};
pub use self::route::MergedRoutes;
pub use self::route::{RouteEntry, RouteState, RouteType, Routes};
pub use self::route_rule::MergedRouteRules;
pub use self::route_rule::{
    RouteRuleAction, RouteRuleEntry, RouteRuleState, RouteRules,
};

pub(crate) use super::{ErrorKind, NipartError};
