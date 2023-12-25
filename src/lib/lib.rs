// SPDX-License-Identifier: Apache-2.0

mod config;
mod error;
mod event;
mod ipc;
mod logging;
mod plugin;
mod plugin_ipc;
mod plugin_trait;
mod query;
// TODO: Currently we are copy code from nmstate, hence suppressed warnings,
//       Need to clean up the code once detached from nmstate code base
#[allow(dead_code)]
mod state;

pub use self::config::NipartNetConfig;
pub use self::error::{ErrorKind, NipartError};
pub use self::event::{
    NipartEvent, NipartEventAction, NipartEventAddress, NipartPluginEvent,
    NipartUserEvent,
};
pub use self::ipc::NipartConnection;
pub use self::logging::NipartLogLevel;
pub use self::plugin::{NipartPluginInfo, NipartRole};
pub use self::plugin_ipc::NipartConnectionListener;
pub use self::plugin_trait::NipartPlugin;
pub use self::query::{NipartQueryConfigOption, NipartQueryStateOption};
pub use self::state::{
    AddressFamily, BaseInterface, BondAdSelect, BondAllPortsActive,
    BondArpAllTargets, BondArpValidate, BondConfig, BondFailOverMac,
    BondInterface, BondLacpRate, BondMode, BondOptions, BondPortConfig,
    BondPrimaryReselect, BondXmitHashPolicy, BridgePortTrunkTag,
    BridgePortVlanConfig, BridgePortVlanMode, BridgePortVlanRange,
    Dhcpv4ClientId, Dhcpv6Duid, DummyInterface, EthernetConfig, EthernetDuplex,
    EthernetInterface, EthtoolCoalesceConfig, EthtoolConfig,
    EthtoolFeatureConfig, EthtoolPauseConfig, EthtoolRingConfig,
    InfiniBandConfig, InfiniBandInterface, InfiniBandMode, Interface,
    InterfaceIdentifier, InterfaceIpAddr, InterfaceIpv4, InterfaceIpv6,
    InterfaceState, InterfaceType, Interfaces, IpsecInterface, Ipv6AddrGenMode,
    LibreswanConfig, LinuxBridgeConfig, LinuxBridgeInterface,
    LinuxBridgeMulticastRouterType, LinuxBridgeOptions, LinuxBridgePortConfig,
    LinuxBridgeStpOptions, LldpAddressFamily, LldpChassisId, LldpChassisIdType,
    LldpConfig, LldpMacPhy, LldpMaxFrameSize, LldpMgmtAddr, LldpMgmtAddrs,
    LldpNeighborTlv, LldpPortId, LldpPortIdType, LldpPpvids,
    LldpSystemCapabilities, LldpSystemCapability, LldpSystemDescription,
    LldpSystemName, LldpVlan, LldpVlans, LoopbackInterface, MacSecConfig,
    MacSecInterface, MacSecValidate, MacVlanConfig, MacVlanInterface,
    MacVlanMode, MacVtapConfig, MacVtapInterface, MacVtapMode,
    MergedInterfaces, MergedRouteRules, MergedRoutes, NetworkState,
    OvsBridgeBondConfig, OvsBridgeBondMode, OvsBridgeBondPortConfig,
    OvsBridgeConfig, OvsBridgeInterface, OvsBridgeOptions, OvsBridgePortConfig,
    OvsBridgeStpOptions, OvsDpdkConfig, OvsInterface, OvsPatchConfig,
    RouteRuleAction, RouteRuleEntry, RouteRuleState, RouteRules, RouteType,
    SrIovConfig, SrIovVfConfig, UnknownInterface, VethConfig, VlanConfig,
    VlanInterface, VlanProtocol, VlanRegistrationProtocol, VrfConfig,
    VrfInterface, VxlanConfig, VxlanInterface, WaitIp, XfrmInterface,
};
