// SPDX-License-Identifier: Apache-2.0

#[allow(dead_code)]
pub(crate) mod deserializer;
mod gen_diff;
mod iface;
mod iface_state;
mod iface_type;
mod ifaces;
mod merged;
mod net_state;
mod revert;
#[allow(dead_code)]
pub(crate) mod serializer;
pub(crate) mod value;

pub use self::iface::{
    Interface, NipartChildInterface, NipartControllerInterface, NipartInterface,
};
pub use self::iface_state::InterfaceState;
pub use self::iface_type::InterfaceType;
pub use self::ifaces::{
    BaseInterface, EthernetConfig, EthernetInterface, Interfaces,
    UnknownInterface,
};
pub use self::merged::{MergedInterface, MergedInterfaces, MergedNetworkState};
pub use self::net_state::NetworkState;
