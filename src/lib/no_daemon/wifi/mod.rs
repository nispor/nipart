// SPDX-License-Identifier: Apache-2.0

#[macro_use]
mod dbus_macros;

mod apply;
mod bss;
mod dbus;
mod interface;
mod network;
mod query;
mod scan;
mod wifi_nispor;

#[derive(Debug)]
pub(crate) struct NipartWpaConn;
