// SPDX-License-Identifier: Apache-2.0

#![doc(html_favicon_url = "https://nmstate.io/favicon.png")]
#![doc(html_logo_url = "https://nmstate.io/favicon.png")]

//! Declarative API for Host Network Management
//! Nmstate is a library with an accompanying command line tool that manages
//! host networking settings in a declarative manner. The networking state is
//! described by a pre-defined schema. Reporting of current state and changes to
//! it (desired state) both conform to the schema.
//!
//! Nmstate is aimed to satisfy enterprise needs to manage host networking
//! through a northbound declarative API and multi provider support on the
//! southbound. NetworkManager acts as the main provider supported to provide
//! persistent network configuration after reboot. Kernel mode is also provided
//! as tech-preview to apply network configurations without NetworkManager.
//!
//! The [NetworkState] and its subordinates are all implemented the serde
//! `Deserialize` and  `Serialize`, instead of building up [NetworkState]
//! manually, you may deserialize it from file(e.g. JSON, YAML and etc).
//!
//! ## Features
//! The `nmstate` crate has these cargo features:
//!  * `gen_conf` -- Generate offline network configures.
//!  * `query_apply` -- Query and apply network state.
//!
//! By default, both features are enabled.
//! The `gen_conf` feature is only supported on Linux platform.
//! The `query_apply` feature is supported and tested on both Linux and MacOS.
//!
//! ## Examples
//!
//! To retrieve current network state:
//!
//! ```rust
//! use nmstate::NetworkState;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut net_state = NetworkState::new();
//!     // Use kernel mode
//!     net_state.set_kernel_only(true);
//!     net_state.retrieve()?;
//!     println!("{}", serde_yaml::to_string(&net_state)?);
//!     Ok(())
//! }
//! ```
//!
//! To apply network configuration(e.g. Assign static IP to eth1):
//!
//! ```no_run
//! use nmstate::NetworkState;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut net_state: NetworkState = serde_yaml::from_str(
//!         r#"---
//!         interfaces:
//!           - name: eth1
//!             type: ethernet
//!             state: up
//!             mtu: 1500
//!             ipv4:
//!               address:
//!               - ip: 192.0.2.252
//!                 prefix-length: 24
//!               - ip: 192.0.2.251
//!                 prefix-length: 24
//!               dhcp: false
//!               enabled: true
//!             ipv6:
//!               address:
//!                 - ip: 2001:db8:2::1
//!                   prefix-length: 64
//!                 - ip: 2001:db8:1::1
//!                   prefix-length: 64
//!               autoconf: false
//!               dhcp: false
//!               enabled: true
//!         "#,
//!     )?;
//!     net_state.set_kernel_only(true);
//!     net_state.apply()?;
//!     Ok(())
//! }
//! ```

mod deserializer;
mod dispatch;
mod dns;
mod error;
mod gen_conf;
mod hostname;
mod ieee8021x;
mod iface;
mod ifaces;
mod ip;
mod lldp;
mod mptcp;
mod net_state;
mod nispor;
mod nm;
#[allow(deprecated)]
mod ovn;
mod ovs;
mod ovsdb;
mod policy;
mod query_apply;
mod revert;
mod route;
mod route_rule;
mod serializer;
mod state;
mod statistic;
mod unit_tests;

pub use crate::state::dispatch::DispatchConfig;
pub use crate::state::dns::MergedDnsState;
pub use crate::state::dns::{DnsClientState, DnsState};
pub use crate::state::error::{ErrorKind, NipartError};
pub use crate::state::hostname::HostNameState;
pub use crate::state::hostname::MergedHostNameState;
pub use crate::state::ieee8021x::Ieee8021XConfig;
pub use crate::state::iface::MergedInterface;
pub use crate::state::iface::{
    Interface, InterfaceIdentifier, InterfaceState, InterfaceType,
    UnknownInterface,
};
pub use crate::state::ifaces::MergedInterfaces;
pub use crate::state::ifaces::{
    BaseInterface, BondAdSelect, BondAllPortsActive, BondArpAllTargets,
    BondArpValidate, BondConfig, BondFailOverMac, BondInterface, BondLacpRate,
    BondMode, BondOptions, BondPortConfig, BondPrimaryReselect,
    BondXmitHashPolicy, BridgePortTrunkTag, BridgePortVlanConfig,
    BridgePortVlanMode, BridgePortVlanRange, DummyInterface, EthernetConfig,
    EthernetDuplex, EthernetInterface, EthtoolCoalesceConfig, EthtoolConfig,
    EthtoolFeatureConfig, EthtoolPauseConfig, EthtoolRingConfig, HsrConfig,
    HsrInterface, HsrProtocol, InfiniBandConfig, InfiniBandInterface,
    InfiniBandMode, Interfaces, IpsecInterface, LibreswanConfig,
    LinuxBridgeConfig, LinuxBridgeInterface, LinuxBridgeMulticastRouterType,
    LinuxBridgeOptions, LinuxBridgePortConfig, LinuxBridgeStpOptions,
    LoopbackInterface, MacSecConfig, MacSecInterface, MacSecValidate,
    MacVlanConfig, MacVlanInterface, MacVlanMode, MacVtapConfig,
    MacVtapInterface, MacVtapMode, OvsBridgeBondConfig, OvsBridgeBondMode,
    OvsBridgeBondPortConfig, OvsBridgeConfig, OvsBridgeInterface,
    OvsBridgeOptions, OvsBridgePortConfig, OvsBridgeStpOptions, OvsDpdkConfig,
    OvsInterface, OvsPatchConfig, SrIovConfig, SrIovVfConfig, VethConfig,
    VlanConfig, VlanInterface, VlanProtocol, VlanRegistrationProtocol,
    VrfConfig, VrfInterface, VxlanConfig, VxlanInterface, XfrmInterface,
};
pub use crate::state::ip::{
    AddressFamily, Dhcpv4ClientId, Dhcpv6Duid, InterfaceIpAddr, InterfaceIpv4,
    InterfaceIpv6, Ipv6AddrGenMode, WaitIp,
};
pub use crate::state::lldp::{
    LldpAddressFamily, LldpChassisId, LldpChassisIdType, LldpConfig,
    LldpMacPhy, LldpMaxFrameSize, LldpMgmtAddr, LldpMgmtAddrs, LldpNeighborTlv,
    LldpPortId, LldpPortIdType, LldpPpvids, LldpSystemCapabilities,
    LldpSystemCapability, LldpSystemDescription, LldpSystemName, LldpVlan,
    LldpVlans,
};
pub use crate::state::mptcp::{MptcpAddressFlag, MptcpConfig};
pub use crate::state::net_state::MergedNetworkState;
pub use crate::state::net_state::NetworkState;
pub use crate::state::ovn::MergedOvnConfiguration;
pub use crate::state::ovn::{
    OvnBridgeMapping, OvnBridgeMappingState, OvnConfiguration,
};
pub use crate::state::ovs::MergedOvsDbGlobalConfig;
pub use crate::state::ovs::{OvsDbGlobalConfig, OvsDbIfaceConfig};
pub use crate::state::policy::{
    NetworkCaptureRules, NetworkPolicy, NetworkStateTemplate,
};
pub use crate::state::route::MergedRoutes;
pub use crate::state::route::{RouteEntry, RouteState, RouteType, Routes};
pub use crate::state::route_rule::MergedRouteRules;
pub use crate::state::route_rule::{
    RouteRuleAction, RouteRuleEntry, RouteRuleState, RouteRules,
};
pub use crate::state::statistic::{NmstateFeature, NmstateStatistic};
