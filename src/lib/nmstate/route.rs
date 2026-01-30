// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original file are:
//  * Gris Ge <fge@redhat.com>
//  * Wen Liang <liangwen12year@gmail.com>
//  * Jan Vaclav <jvaclav@redhat.com>
//  * Íñigo Huguet <ihuguet@redhat.com>
//  * Fernando Fernandez Mancera <ffmancera@riseup.net>

use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    net::Ipv4Addr,
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use super::ip::{is_ipv6_addr, sanitize_ip_network};
use crate::{
    ErrorKind, Interfaces, JsonDisplay, NipartError, NipartstateInterface,
};

const DEFAULT_TABLE_ID: u32 = 254; // main route table ID
const LOOPBACK_IFACE_NAME: &str = "lo";

#[derive(
    Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonDisplay,
)]
#[non_exhaustive]
#[serde(deny_unknown_fields)]
/// IP routing status
pub struct Routes {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Running effected routes containing route from universe or link scope,
    /// and only from these protocols:
    ///  * boot (often used by `iproute` command)
    ///  * static
    ///  * ra
    ///  * dhcp
    ///  * mrouted
    ///  * keepalived
    ///  * babel
    ///
    /// Ignored when applying.
    pub running: Option<Vec<RouteEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Static routes containing route from universe or link scope,
    /// and only from these protocols:
    ///  * boot (often used by `iproute` command)
    ///  * static
    ///
    /// When applying, `None` means preserve current routes.
    /// This property is not overriding but adding specified routes to
    /// existing routes. To delete a route entry, please [RouteEntry.state] as
    /// [RouteState::Absent]. Any property of absent [RouteEntry] set to
    /// `None` means wildcard. For example, this [crate::NetworkState] could
    /// remove all routes next hop to interface eth1(showing in yaml):
    /// ```yaml
    /// routes:
    ///   config:
    ///   - next-hop-interface: eth1
    ///     state: absent
    /// ```
    ///
    /// To change a route entry, you need to delete old one and add new one(can
    /// be in single transaction).
    pub config: Option<Vec<RouteEntry>>,
}

impl Routes {
    /// Whether configured routes is empty or undefined.
    pub fn is_empty(&self) -> bool {
        if let Some(rts) = self.config.as_ref() {
            rts.is_empty()
        } else {
            true
        }
    }

    pub(crate) fn validate(&self) -> Result<(), NipartError> {
        // All desire non-absent route should have next hop interface except
        // for route with route type `Blackhole`, `Unreachable`, `Prohibit`.
        if let Some(config_routes) = self.config.as_ref() {
            for route in config_routes.iter() {
                if !route.is_absent() {
                    if !route.is_unicast()
                        && (route.next_hop_iface.is_some()
                            && route.next_hop_iface
                                != Some(LOOPBACK_IFACE_NAME.to_string())
                            || route.next_hop_addr.is_some())
                    {
                        return Err(NipartError::new(
                            ErrorKind::InvalidArgument,
                            format!(
                                "A {:?} Route cannot have a next hop : \
                                 {route:?}",
                                route.route_type.unwrap()
                            ),
                        ));
                    } else if route.next_hop_iface.is_none()
                        && route.is_unicast()
                    {
                        return Err(NipartError::new(
                            ErrorKind::NoSupport,
                            format!(
                                "Route with empty next hop interface is not \
                                 supported: {route:?}"
                            ),
                        ));
                    }
                }
                validate_route_dst(route)?;
            }
        }
        Ok(())
    }

    pub(crate) fn remove_ignored_routes(&mut self) {
        for rts in [self.running.as_mut(), self.config.as_mut()]
            .into_iter()
            .flatten()
        {
            rts.retain(|rt| !rt.is_ignore());
        }
    }

    pub(crate) fn mark_route_as_ignored_ifaces(&mut self, ifaces: &Interfaces) {
        let ignored_ifaces: HashSet<&str> = ifaces
            .kernel_ifaces
            .iter()
            .filter_map(|(name, iface)| {
                if iface.is_ignore() {
                    Some(name.as_str())
                } else {
                    None
                }
            })
            .collect();

        for rts in [self.running.as_mut(), self.config.as_mut()]
            .into_iter()
            .flatten()
        {
            for rt in rts {
                if Some(true)
                    == rt
                        .next_hop_iface
                        .as_deref()
                        .map(|iface| ignored_ifaces.contains(&iface))
                {
                    rt.state = Some(RouteState::Ignore);
                }
            }
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonDisplay,
)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
#[derive(Default)]
pub enum RouteState {
    /// Mark a route entry as absent to remove it.
    #[default]
    Absent,
    /// Mark a route as ignored
    Ignore,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonDisplay)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
#[serde(deny_unknown_fields)]
/// Route entry
pub struct RouteEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Only used for delete route when applying.
    pub state: Option<RouteState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Route destination address or network
    /// Mandatory for every non-absent routes.
    pub destination: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "next-hop-interface"
    )]
    /// Route next hop interface name.
    /// Serialize and deserialize to/from `next-hop-interface`.
    /// Mandatory for every non-absent routes except for route with
    /// route type `Blackhole`, `Unreachable`, `Prohibit`.
    pub next_hop_iface: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "next-hop-address"
    )]
    /// Route next hop IP address.
    /// Serialize and deserialize to/from `next-hop-address`.
    /// When setting this as empty string for absent route, it will only delete
    /// routes __without__ `next-hop-address`.
    pub next_hop_addr: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_i64_or_string"
    )]
    /// Route metric. [RouteEntry::USE_DEFAULT_METRIC] for default
    /// setting of network backend.
    pub metric: Option<i64>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    /// Route table id. [RouteEntry::USE_DEFAULT_ROUTE_TABLE] for main
    /// route table 254.
    pub table_id: Option<u32>,

    /// ECMP(Equal-Cost Multi-Path) route weight
    /// The valid range of this property is 1-256.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u16_or_string"
    )]
    pub weight: Option<u16>,
    /// Route type
    /// Serialize and deserialize to/from `route-type`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_type: Option<RouteType>,
    /// Congestion window clamp
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub cwnd: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Route source defines which IP address should be used as the source
    /// for packets routed via a specific route
    pub source: Option<String>,
    /// Initial congestion window size
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub initcwnd: Option<u32>,
    /// Initial receive window size
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub initrwnd: Option<u32>,
    /// MTU
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub mtu: Option<u32>,
    /// Enable quickack will disable disables delayed acknowledgments.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub quickack: Option<bool>,
    /// Maximal Segment Size to advertise for TCP connections
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub advmss: Option<u32>,
    // TODO: Store the routes to the route table specified VRF bind to.
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub vrf_name: Option<String>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonDisplay,
)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
#[serde(deny_unknown_fields)]
pub enum RouteType {
    Blackhole,
    Unreachable,
    Prohibit,
}

const RTN_UNICAST: u8 = 1;
const RTN_BLACKHOLE: u8 = 6;
const RTN_UNREACHABLE: u8 = 7;
const RTN_PROHIBIT: u8 = 8;

impl From<RouteType> for u8 {
    fn from(v: RouteType) -> u8 {
        match v {
            RouteType::Blackhole => RTN_BLACKHOLE,
            RouteType::Unreachable => RTN_UNREACHABLE,
            RouteType::Prohibit => RTN_PROHIBIT,
        }
    }
}

impl RouteEntry {
    pub const USE_DEFAULT_METRIC: i64 = -1;
    pub const USE_DEFAULT_ROUTE_TABLE: u32 = 0;

    pub(crate) fn is_absent(&self) -> bool {
        matches!(self.state, Some(RouteState::Absent))
    }

    pub(crate) fn is_ignore(&self) -> bool {
        matches!(self.state, Some(RouteState::Ignore))
    }

    /// Whether the desired route (self) matches with another
    /// metric is ignored.
    pub(crate) fn is_match(&self, other: &Self) -> bool {
        if self.destination.as_ref().is_some()
            && self.destination.as_deref() != Some("")
            && self.destination != other.destination
        {
            return false;
        }
        if self.next_hop_iface.as_ref().is_some()
            && self.next_hop_iface != other.next_hop_iface
        {
            return false;
        }

        if self.next_hop_addr.as_ref().is_some()
            && self.next_hop_addr != other.next_hop_addr
        {
            return false;
        }
        if self.table_id.is_some()
            && self.table_id != Some(RouteEntry::USE_DEFAULT_ROUTE_TABLE)
            && self.table_id != other.table_id
        {
            return false;
        }
        if self.weight.is_some() && self.weight != other.weight {
            return false;
        }
        if self.route_type.is_some() && self.route_type != other.route_type {
            return false;
        }
        if self.cwnd.is_some() && self.cwnd != other.cwnd {
            return false;
        }
        if self.source.as_ref().is_some() && self.source != other.source {
            return false;
        }
        if self.initcwnd.is_some() && self.initcwnd != other.initcwnd {
            return false;
        }
        if self.initrwnd.is_some() && self.initrwnd != other.initrwnd {
            return false;
        }
        if self.mtu.is_some() && self.mtu != other.mtu {
            return false;
        }
        if self.quickack.is_some() && self.quickack != other.quickack {
            return false;
        }
        if self.advmss.is_some() && self.advmss != other.advmss {
            return false;
        }
        // if self.vrf_name.is_some() && self.vrf_name != other.vrf_name {
        //    return false;
        // }
        true
    }

    // Return tuple of Vec of all properties with default value unwrapped.
    // Metric is ignored
    fn sort_key(&self) -> (Vec<bool>, Vec<&str>, Vec<u32>) {
        (
            vec![
                // not_absent
                !matches!(self.state, Some(RouteState::Absent)),
                // is_ipv6
                !self
                    .destination
                    .as_ref()
                    .map(|d| is_ipv6_addr(d.as_str()))
                    .unwrap_or_default(),
                self.quickack.unwrap_or_default(),
            ],
            vec![
                self.next_hop_iface
                    .as_deref()
                    .unwrap_or(LOOPBACK_IFACE_NAME),
                self.destination.as_deref().unwrap_or(""),
                self.next_hop_addr.as_deref().unwrap_or(""),
                self.source.as_deref().unwrap_or(""),
                // self.vrf_name.as_deref().unwrap_or(""),
            ],
            vec![
                self.table_id.unwrap_or(DEFAULT_TABLE_ID),
                self.cwnd.unwrap_or_default(),
                self.initcwnd.unwrap_or_default(),
                self.initrwnd.unwrap_or_default(),
                self.mtu.unwrap_or_default(),
                self.weight.unwrap_or_default().into(),
                self.route_type
                    .as_ref()
                    .map(|t| u8::from(*t))
                    .unwrap_or_default()
                    .into(),
                self.advmss.unwrap_or_default(),
            ],
        )
    }

    pub(crate) fn sanitize(&mut self) -> Result<(), NipartError> {
        if let Some(dst) = self.destination.as_ref() {
            if dst.is_empty() {
                self.destination = None;
            } else {
                let new_dst = sanitize_ip_network(dst)?;
                if dst != &new_dst {
                    log::warn!(
                        "Route destination {dst} sanitized to {new_dst}"
                    );
                    self.destination = Some(new_dst);
                }
            }
        }
        if let Some(via) = self.next_hop_addr.as_ref() {
            let new_via = format!("{}", via.parse::<std::net::IpAddr>()?);
            if via != &new_via {
                log::warn!(
                    "Route next-hop-address {via} sanitized to {new_via}"
                );
                self.next_hop_addr = Some(new_via);
            }
        }
        if let Some(src) = self.source.as_ref() {
            let new_src = format!(
                "{}",
                src.parse::<std::net::IpAddr>().map_err(|e| {
                    NipartError::new(
                        ErrorKind::InvalidArgument,
                        format!("Failed to parse IP address '{src}': {e}"),
                    )
                })?
            );
            if src != &new_src {
                log::info!("Route source address {src} sanitized to {new_src}");
                self.source = Some(new_src);
            }
        }
        if let Some(weight) = self.weight {
            if !(1..=256).contains(&weight) {
                return Err(NipartError::new(
                    ErrorKind::InvalidArgument,
                    format!(
                        "Invalid ECMP route weight {weight}, should be in the \
                         range of 1 to 256"
                    ),
                ));
            }
            if let Some(dst) = self.destination.as_deref()
                && is_ipv6_addr(dst) {
                    return Err(NipartError::new(
                        ErrorKind::NoSupport,
                        "IPv6 ECMP route with weight is not supported yet"
                            .to_string(),
                    ));
                }
        }
        if let Some(cwnd) = self.cwnd
            && cwnd == 0 {
                return Err(NipartError::new(
                    ErrorKind::InvalidArgument,
                    "The value of 'cwnd' cannot be 0".to_string(),
                ));
            }
        if self.mtu == Some(0) {
            return Err(NipartError::new(
                ErrorKind::InvalidArgument,
                "The value of 'mtu' cannot be 0".to_string(),
            ));
        }
        if self.advmss == Some(0) {
            return Err(NipartError::new(
                ErrorKind::InvalidArgument,
                "The value of 'advmss' cannot be 0".to_string(),
            ));
        }
        Ok(())
    }

    pub(crate) fn is_ipv6(&self) -> bool {
        self.destination.as_ref().map(|d| is_ipv6_addr(d.as_str()))
            == Some(true)
    }

    pub(crate) fn is_unicast(&self) -> bool {
        self.route_type.is_none()
            || u8::from(self.route_type.unwrap()) == RTN_UNICAST
    }
}

// For Vec::dedup()
impl PartialEq for RouteEntry {
    fn eq(&self, other: &Self) -> bool {
        self.sort_key() == other.sort_key()
    }
}

// For Vec::sort_unstable()
impl Ord for RouteEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

// For ord
impl Eq for RouteEntry {}

// For ord
impl PartialOrd for RouteEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for RouteEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.sort_key().hash(state);
    }
}

// Validating if the route destination network is valid,
// 0.0.0.0/8 and its subnet cannot be used as the route destination network
// for unicast route
fn validate_route_dst(route: &RouteEntry) -> Result<(), NipartError> {
    if let Some(dst) = route.destination.as_deref()
        && !is_ipv6_addr(dst) {
            let ip_net: Vec<&str> = dst.split('/').collect();
            let ip_addr = Ipv4Addr::from_str(ip_net[0])?;
            if ip_addr.octets()[0] == 0 {
                if dst.contains('/') {
                    let prefix = match ip_net[1].parse::<i32>() {
                        Ok(p) => p,
                        Err(_) => {
                            return Err(NipartError::new(
                                ErrorKind::InvalidArgument,
                                format!(
                                    "The prefix of the route destination \
                                     network '{dst}' is invalid"
                                ),
                            ));
                        }
                    };
                    if prefix >= 8 && route.is_unicast() {
                        let e = NipartError::new(
                            ErrorKind::InvalidArgument,
                            "0.0.0.0/8 and its subnet cannot be used as the \
                             route destination for unicast route, please use \
                             the default gateway 0.0.0.0/0 instead"
                                .to_string(),
                        );
                        log::error!("{e}");
                        return Err(e);
                    }
                } else if route.is_unicast() {
                    let e = NipartError::new(
                        ErrorKind::InvalidArgument,
                        "0.0.0.0/8 and its subnet cannot be used as the route \
                         destination for unicast route, please use the \
                         default gateway 0.0.0.0/0 instead"
                            .to_string(),
                    );
                    log::error!("{e}");
                    return Err(e);
                }
            }
            return Ok(());
        }
    Ok(())
}
