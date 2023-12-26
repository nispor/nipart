// SPDX-License-Identifier: Apache-2.0

mod apply;
mod base_iface;
mod bond;
mod error;
mod ethernet;
mod ethtool;
mod hostname;
mod infiniband;
mod ip;
mod linux_bridge;
mod linux_bridge_port_vlan;
mod mac_vlan;
mod macsec;
mod mptcp;
mod plugin;
mod route;
mod route_rule;
mod show;
mod veth;
mod vlan;
mod vrf;
mod vxlan;

pub(crate) use self::show::nispor_retrieve;
pub(crate) use self::apply::nispor_apply;

use self::plugin::NipartPluginNispor;

use nipart::{NipartError, NipartPlugin};

#[tokio::main]
async fn main() -> Result<(), NipartError> {
    NipartPluginNispor::run().await
}
