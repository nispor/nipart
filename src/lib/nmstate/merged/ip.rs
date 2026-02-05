// SPDX-License-Identifier: Apache-2.0

use std::net::IpAddr;

use crate::{InterfaceIpAddr, InterfaceIpv4, InterfaceIpv6};

impl InterfaceIpv4 {
    pub(crate) fn post_merge(&mut self, old: &Self) {
        // Normally, we expect backend to preserve configuration which not
        // mentioned in desire or all auto ip address, but when DHCP switch from
        // ON to OFF, the design of nmstate is expecting dynamic IP address goes
        // static. This should be done by top level code.
        if old.is_auto()
            && old.addresses.is_some()
            && self.is_enabled()
            && !self.is_auto()
            && is_ip_addrs_none_or_all_auto(old.addresses.as_deref())
            && let Some(addrs) = self.addresses.as_mut()
        {
            addrs.as_mut_slice().iter_mut().for_each(|a| {
                a.valid_life_time = None;
                a.preferred_life_time = None;
            });
        }
    }
}

impl InterfaceIpv6 {
    pub(crate) fn post_merge(&mut self, old: &Self) {
        // Normally, we expect backend to preserve configuration which not
        // mentioned in desire, but when DHCP switch from ON to OFF, the design
        // of nmstate is expecting dynamic IP address goes static. This should
        // be done by top level code.
        if old.is_auto()
            && old.addresses.is_some()
            && self.is_enabled()
            && !self.is_auto()
            && is_ip_addrs_none_or_all_auto(old.addresses.as_deref())
            && let Some(addrs) = self.addresses.as_mut()
        {
            addrs.as_mut_slice().iter_mut().for_each(|a| {
                a.valid_life_time = None;
                a.preferred_life_time = None;
            });
        }
    }
}

fn is_ip_addrs_none_or_all_auto(addrs: Option<&[InterfaceIpAddr]>) -> bool {
    addrs.is_none_or(|addrs| {
        addrs.iter().all(|a| {
            if let IpAddr::V6(ip_addr) = a.ip {
                ip_addr.is_unicast_link_local() || a.is_auto()
            } else {
                a.is_auto()
            }
        })
    })
}
