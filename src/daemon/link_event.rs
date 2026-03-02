// SPDX-License-Identifier: Apache-2.0

use std::time::SystemTime;

use nipart::{InterfaceType, JsonDisplay};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize, JsonDisplay)]
pub(crate) struct NipartLinkEvent {
    pub iface_name: String,
    pub iface_index: u32,
    pub iface_type: InterfaceType,
    pub is_up: bool,
    pub is_delete: bool,
    pub time_stamp: SystemTime,
    /// For WIFI interface only, SSID connected
    pub ssid: Option<String>,
}

impl NipartLinkEvent {
    pub(crate) fn new(
        iface_name: String,
        iface_index: u32,
        iface_type: InterfaceType,
        is_up: bool,
        ssid: Option<String>,
    ) -> Self {
        Self {
            iface_name,
            iface_index,
            iface_type,
            is_up,
            is_delete: false,
            time_stamp: SystemTime::now(),
            ssid,
        }
    }
}
