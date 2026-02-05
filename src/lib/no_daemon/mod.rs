// SPDX-License-Identifier: Apache-2.0

mod apply;
mod base_iface;
mod bond;
mod dhcp;
mod ethernet;
mod iface;
mod inter_ifaces;
mod ip;
mod linux_bridge;
mod linux_bridge_vlan;
mod ovs;
mod query;
mod route;
mod vlan;
mod watcher;
mod wifi;
mod wireguard;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct NipartNoDaemon {}
