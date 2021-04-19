//    Copyright 2021 Red Hat, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use nispor::{Iface, IfaceState, IfaceType, Ipv4AddrInfo, Ipv6AddrInfo};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NipartIfaceState {
    Up,
    Down,
    Unknown,
}

impl std::convert::From<&IfaceState> for NipartIfaceState {
    fn from(state: &IfaceState) -> Self {
        match state {
            IfaceState::Up => Self::Up,
            IfaceState::Down => Self::Down,
            _ => Self::Unknown,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct NipartIpInfo {
    pub(crate) address: String,
    pub(crate) prefix_len: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    // The renaming seonds for this address be valid, None means forever.
    pub(crate) valid_lft: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // The renaming seonds for this address be preferred. None means forever.
    pub(crate) preferred_lft: Option<u32>,
}

impl std::convert::From<&Ipv4AddrInfo> for NipartIpInfo {
    fn from(ip_info: &Ipv4AddrInfo) -> Self {
        Self {
            address: ip_info.address.clone(),
            prefix_len: ip_info.prefix_len,
            valid_lft: time_str_to_u32(&ip_info.valid_lft),
            preferred_lft: time_str_to_u32(&ip_info.preferred_lft),
        }
    }
}

impl std::convert::From<&Ipv6AddrInfo> for NipartIpInfo {
    fn from(ip_info: &Ipv6AddrInfo) -> Self {
        Self {
            address: ip_info.address.clone(),
            prefix_len: ip_info.prefix_len,
            valid_lft: time_str_to_u32(&ip_info.valid_lft),
            preferred_lft: time_str_to_u32(&ip_info.preferred_lft),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct NipartBaseIface {
    pub(crate) name: String,
    pub(crate) iface_type: String,
    pub(crate) state: NipartIfaceState,
    pub(crate) mtu: i64,
    pub(crate) mac_address: String,
    pub(crate) ipv4: Option<Vec<NipartIpInfo>>,
    pub(crate) ipv6: Option<Vec<NipartIpInfo>>,
}

impl std::convert::From<&Iface> for NipartBaseIface {
    fn from(iface: &Iface) -> Self {
        Self {
            name: iface.name.clone(),
            iface_type: match &iface.iface_type {
                IfaceType::Bond => "bond".into(),
                _ => format!("{:?}", iface.iface_type),
            },
            state: (&iface.state).into(),
            mtu: iface.mtu,
            mac_address: iface.mac_address.clone(),
            ipv4: match &iface.ipv4 {
                Some(ips) => Some(
                    (&ips.addresses).into_iter().map(|i| i.into()).collect(),
                ),
                None => None,
            },
            ipv6: match &iface.ipv6 {
                Some(ips) => Some(
                    (&ips.addresses).into_iter().map(|i| i.into()).collect(),
                ),
                None => None,
            },
        }
    }
}

fn time_str_to_u32(time: &str) -> Option<u32> {
    match time {
        "forever" => None,
        _ => {
            if time.len() > "sec".len() {
                match time[..time.len() - "sec".len()].parse::<u32>() {
                    Ok(i) => Some(i),
                    Err(e) => {
                        eprintln!(
                            "ERROR: invalid time string: {}: {}",
                            time, e
                        );
                        Some(0u32)
                    }
                }
            } else {
                Some(0u32)
            }
        }
    }
}
