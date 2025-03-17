// SPDX-License-Identifier: Apache-2.0

mod gen_diff;
mod iface;
mod iface_state;
mod iface_trait;
mod iface_type;
mod ifaces;
mod merged;
mod net_state;
mod revert;
mod value;

#[allow(dead_code)]
pub(crate) mod deserializer;
#[allow(dead_code)]
pub(crate) mod serializer;

pub use self::iface::Interface;
pub use self::iface_state::InterfaceState;
pub use self::iface_trait::{
    NipartChildInterface, NipartControllerInterface, NipartInterface,
};
pub use self::iface_type::InterfaceType;
pub use self::ifaces::{
    BaseInterface, EthernetConfig, EthernetDuplex, EthernetInterface,
    Interfaces, UnknownInterface,
};
pub use self::merged::{MergedInterface, MergedInterfaces, MergedNetworkState};
pub use self::net_state::NetworkState;
