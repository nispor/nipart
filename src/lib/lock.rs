// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::InterfaceType;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[non_exhaustive]
pub enum NipartLockEntry {
    Interface(Box<(String, InterfaceType)>),
    Dns,
    Route,
    RouteRule,
}

impl std::fmt::Display for NipartLockEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Interface(v) => write!(f, "lock.iface:{}/{}", v.0, v.1),
            Self::Dns => write!(f, "lock.dns"),
            Self::Route => write!(f, "lock.route"),
            Self::RouteRule => write!(f, "lock.route_rule"),
        }
    }
}

impl NipartLockEntry {
    pub fn new_iface(iface_name: String, iface_type: InterfaceType) -> Self {
        Self::Interface(Box::new((iface_name, iface_type)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[non_exhaustive]
pub struct NipartLockOption {
    pub timeout_seconds: u32,
}

impl std::fmt::Display for NipartLockOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lock_option.timeout:{}s", self.timeout_seconds)
    }
}

impl NipartLockOption {
    pub fn new(timeout_seconds: u32) -> Self {
        Self { timeout_seconds }
    }
}
