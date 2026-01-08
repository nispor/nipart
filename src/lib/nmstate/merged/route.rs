// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original file are:
//  * Gris Ge <fge@redhat.com>
//  * Wen Liang <liangwen12year@gmail.com>
//  * Jan Vaclav <jvaclav@redhat.com>
//  * Íñigo Huguet <ihuguet@redhat.com>
//  * Fernando Fernandez Mancera <ffmancera@riseup.net>

use std::collections::{HashMap, HashSet, hash_map::Entry};

use serde::{Deserialize, Serialize};

use crate::{
    ErrorKind, JsonDisplay, MergedInterfaces, NipartError, NipartstateInterface,
    RouteEntry, RouteState, Routes,
};

const LOOPBACK_IFACE_NAME: &str = "lo";

#[derive(
    Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, JsonDisplay,
)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct MergedRoutes {
    // When all routes next hop to a interface are all marked as absent,
    // the `MergedRoutes.merged` will not have entry for this interface, but
    // interface name is found in `MergedRoutes.route_changed_ifaces`.
    // For backend use incremental route changes, please use
    // `MergedRoutes.changed_routes`.
    pub merged: HashMap<String, Vec<RouteEntry>>,
    pub route_changed_ifaces: Vec<String>,
    // The `changed_routes` contains desired new routes and also including
    // current routes been marked as absent. Not including desired route equal
    // to current route.
    pub changed_routes: Vec<RouteEntry>,
    pub desired: Routes,
    pub current: Routes,
}

impl MergedRoutes {
    pub fn new(
        mut desired: Routes,
        current: Routes,
        merged_ifaces: &MergedInterfaces,
    ) -> Result<Self, NipartError> {
        desired.remove_ignored_routes();
        desired.validate()?;

        let mut desired_routes = Vec::new();
        if let Some(rts) = desired.config.as_ref() {
            for rt in rts {
                let mut rt = rt.clone();
                rt.sanitize()?;
                desired_routes.push(rt);
            }
        }

        let mut changed_ifaces: HashSet<&str> = HashSet::new();
        let mut changed_routes: HashSet<RouteEntry> = HashSet::new();

        let ifaces_marked_as_absent: Vec<&str> = merged_ifaces
            .kernel_ifaces
            .values()
            .filter(|i| i.merged.is_absent())
            .map(|i| i.merged.name())
            .collect();

        let ifaces_with_ipv4_disabled: Vec<&str> = merged_ifaces
            .kernel_ifaces
            .values()
            .filter(|i| !i.merged.base_iface().is_ipv4_enabled())
            .map(|i| i.merged.name())
            .collect();

        let ifaces_with_ipv6_disabled: Vec<&str> = merged_ifaces
            .kernel_ifaces
            .values()
            .filter(|i| !i.merged.base_iface().is_ipv6_enabled())
            .map(|i| i.merged.name())
            .collect();

        // Interface has route added.
        for rt in desired_routes
            .as_slice()
            .iter()
            .filter(|rt| !rt.is_absent())
        {
            if let Some(via) = rt.next_hop_iface.as_ref() {
                if ifaces_marked_as_absent.contains(&via.as_str()) {
                    return Err(NipartError::new(
                        ErrorKind::InvalidArgument,
                        format!(
                            "The next hop interface of desired Route '{rt}' \
                             has been marked as absent"
                        ),
                    ));
                }
                if rt.is_ipv6()
                    && ifaces_with_ipv6_disabled.contains(&via.as_str())
                {
                    return Err(NipartError::new(
                        ErrorKind::InvalidArgument,
                        format!(
                            "The next hop interface of desired Route '{rt}' \
                             has been marked as IPv6 disabled"
                        ),
                    ));
                }
                if (!rt.is_ipv6())
                    && ifaces_with_ipv4_disabled.contains(&via.as_str())
                {
                    return Err(NipartError::new(
                        ErrorKind::InvalidArgument,
                        format!(
                            "The next hop interface of desired Route '{rt}' \
                             has been marked as IPv4 disabled"
                        ),
                    ));
                }
                changed_ifaces.insert(via.as_str());
            } else if rt.route_type.is_some() {
                changed_ifaces.insert(LOOPBACK_IFACE_NAME);
            }
        }

        // Interface has route deleted.
        for absent_rt in
            desired_routes.as_slice().iter().filter(|rt| rt.is_absent())
        {
            if let Some(cur_rts) = current.config.as_ref() {
                for rt in cur_rts {
                    if absent_rt.is_match(rt) {
                        if let Some(via) = rt.next_hop_iface.as_ref() {
                            changed_ifaces.insert(via.as_str());
                        } else {
                            changed_ifaces.insert(LOOPBACK_IFACE_NAME);
                        }
                    }
                }
            }
        }

        let mut merged_routes: Vec<RouteEntry> = Vec::new();

        if let Some(cur_rts) = current.config.as_ref() {
            for rt in cur_rts {
                if let Some(via) = rt.next_hop_iface.as_ref() {
                    // We include current route to merged_routes when it is
                    // not marked as absent due to absent interface or disabled
                    // ip stack or route state:absent.
                    if ifaces_marked_as_absent.contains(&via.as_str())
                        || (rt.is_ipv6()
                            && ifaces_with_ipv6_disabled
                                .contains(&via.as_str()))
                        || (!rt.is_ipv6()
                            && ifaces_with_ipv4_disabled
                                .contains(&via.as_str()))
                        || desired_routes
                            .as_slice()
                            .iter()
                            .filter(|r| r.is_absent())
                            .any(|absent_rt| absent_rt.is_match(rt))
                    {
                        let mut new_rt = rt.clone();
                        new_rt.state = Some(RouteState::Absent);
                        changed_routes.insert(new_rt);
                    } else {
                        merged_routes.push(rt.clone());
                    }
                }
            }
        }

        // Append desired routes
        for rt in desired_routes
            .as_slice()
            .iter()
            .filter(|rt| !rt.is_absent())
        {
            if let Some(cur_rts) = current.config.as_ref() {
                if !cur_rts.as_slice().iter().any(|cur_rt| cur_rt.is_match(rt))
                {
                    changed_routes.insert(rt.clone());
                }
            } else {
                changed_routes.insert(rt.clone());
            }
            merged_routes.push(rt.clone());
        }

        merged_routes.sort_unstable();
        merged_routes.dedup();

        let mut merged: HashMap<String, Vec<RouteEntry>> = HashMap::new();

        for rt in merged_routes {
            if let Some(via) = rt.next_hop_iface.as_ref() {
                let rts: &mut Vec<RouteEntry> =
                    match merged.entry(via.to_string()) {
                        Entry::Occupied(o) => o.into_mut(),
                        Entry::Vacant(v) => v.insert(Vec::new()),
                    };
                rts.push(rt);
            } else if rt.route_type.is_some() {
                let rts: &mut Vec<RouteEntry> =
                    match merged.entry(LOOPBACK_IFACE_NAME.to_string()) {
                        Entry::Occupied(o) => o.into_mut(),
                        Entry::Vacant(v) => v.insert(Vec::new()),
                    };
                rts.push(rt);
            }
        }

        let route_changed_ifaces: Vec<String> =
            changed_ifaces.iter().map(|i| i.to_string()).collect();

        let mut ret = Self {
            merged,
            desired,
            current,
            route_changed_ifaces,
            changed_routes: changed_routes.drain().collect(),
        };

        ret.remove_routes_to_ignored_ifaces(merged_ifaces);

        Ok(ret)
    }

    fn remove_routes_to_ignored_ifaces(
        &mut self,
        merged_ifaces: &MergedInterfaces,
    ) {
        let ignored_ifaces: Vec<&str> = merged_ifaces
            .kernel_ifaces
            .values()
            .filter_map(|merged_iface| {
                if merged_iface.merged.is_ignore() {
                    Some(merged_iface.merged.name())
                } else {
                    None
                }
            })
            .collect();

        for iface in ignored_ifaces.as_slice() {
            self.merged.remove(*iface);
        }
        self.route_changed_ifaces
            .retain(|n| !ignored_ifaces.contains(&n.as_str()));
    }

    pub(crate) fn is_changed(&self) -> bool {
        !self.route_changed_ifaces.is_empty()
    }

    pub(crate) fn gen_state_for_apply(&self) -> Routes {
        Routes {
            running: None,
            config: Some(self.changed_routes.clone()),
        }
    }
}

impl Routes {
    /// Return new Routes data contains the merged data.
    pub(crate) fn merge(&self, new_routes: &Self) -> Result<Self, NipartError> {
        new_routes.validate()?;

        if let Some(new_routes) = new_routes.config.as_ref() {
            let mut route_sets: HashSet<RouteEntry> = HashSet::new();
            for new_route in new_routes.iter().filter(|r| !r.is_absent()) {
                route_sets.insert(new_route.clone());
            }
            if let Some(old_routes) = self.config.as_ref() {
                for old_route in old_routes {
                    if new_routes
                        .iter()
                        .any(|r| r.is_absent() && r.is_match(old_route))
                    {
                        let mut absent_route = old_route.clone();
                        absent_route.state = Some(RouteState::Absent);
                        route_sets.insert(absent_route);
                    } else {
                        route_sets.insert(old_route.clone());
                    }
                }
            }
            let mut routes: Vec<RouteEntry> = route_sets.into_iter().collect();
            routes.sort_unstable();

            Ok(Routes {
                config: Some(routes),
                ..Default::default()
            })
        } else {
            Ok(self.clone())
        }
    }
}
