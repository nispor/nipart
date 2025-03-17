// SPDX-License-Identifier: Apache-2.0

mod base;
mod ethernet;
mod inter_ifaces;
mod unknown;

pub use self::base::BaseInterface;
pub use self::ethernet::{EthernetConfig, EthernetDuplex, EthernetInterface};
pub use self::inter_ifaces::Interfaces;
pub use self::unknown::UnknownInterface;
