// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use nipart::{InterfaceIpAddr, InterfaceIpv4, InterfaceIpv6};

use crate::mptcp::get_mptcp_flags;

pub(crate) fn np_ipv4_to_nipart(
    np_iface: &nispor::Iface,
    running_config_only: bool,
) -> Option<InterfaceIpv4> {
    if let Some(np_ip) = &np_iface.ipv4 {
        let mut ip = InterfaceIpv4::default();
        if np_ip.addresses.is_empty() {
            ip.enabled = false;
            ip.enabled_defined = true;
            return Some(ip);
        }
        ip.enabled = true;
        ip.enabled_defined = true;
        let mut addresses = Vec::new();
        for np_addr in &np_ip.addresses {
            if np_addr.valid_lft != "forever" {
                ip.dhcp = Some(true);
                if running_config_only {
                    continue;
                }
            }
            match std::net::IpAddr::from_str(np_addr.address.as_str()) {
                Ok(i) => addresses.push({
                    let mut addr = InterfaceIpAddr::default();
                    addr.ip = i;
                    addr.prefix_length = np_addr.prefix_len;
                    addr.mptcp_flags = Some(get_mptcp_flags(
                        np_iface,
                        np_addr.address.as_str(),
                    ));
                    addr.valid_life_time = if np_addr.valid_lft != "forever" {
                        Some(np_addr.valid_lft.clone())
                    } else {
                        None
                    };
                    addr.preferred_life_time =
                        if np_addr.preferred_lft != "forever" {
                            Some(np_addr.preferred_lft.clone())
                        } else {
                            None
                        };
                    addr
                }),
                Err(e) => {
                    log::warn!(
                        "BUG: nispor got invalid IP address {}, error {}",
                        np_addr.address.as_str(),
                        e
                    );
                }
            }
        }
        ip.addresses = Some(addresses);
        Some(ip)
    } else {
        // IP might just disabled
        let mut ip = InterfaceIpv4::default();
        ip.enabled = false;
        ip.enabled_defined = true;
        Some(ip)
    }
}

pub(crate) fn np_ipv6_to_nipart(
    np_iface: &nispor::Iface,
    running_config_only: bool,
) -> Option<InterfaceIpv6> {
    if let Some(np_ip) = &np_iface.ipv6 {
        let mut ip = InterfaceIpv6::default();
        if np_ip.addresses.is_empty() {
            ip.enabled = false;
            ip.enabled_defined = true;
            return Some(ip);
        }
        ip.enabled = true;
        ip.enabled_defined = true;
        if let Some(token) = np_ip.token.as_ref() {
            ip.token = Some(token.to_string());
        }

        let mut addresses = Vec::new();
        for np_addr in &np_ip.addresses {
            if np_addr.valid_lft != "forever" {
                ip.autoconf = Some(true);
                if running_config_only {
                    continue;
                }
            }
            match std::net::IpAddr::from_str(np_addr.address.as_str()) {
                Ok(i) => addresses.push({
                    let mut addr = InterfaceIpAddr::default();
                    addr.ip = i;
                    addr.prefix_length = np_addr.prefix_len;
                    addr.mptcp_flags = Some(get_mptcp_flags(
                        np_iface,
                        np_addr.address.as_str(),
                    ));
                    addr.valid_life_time = if np_addr.valid_lft != "forever" {
                        Some(np_addr.valid_lft.clone())
                    } else {
                        None
                    };
                    addr.preferred_life_time =
                        if np_addr.preferred_lft != "forever" {
                            Some(np_addr.preferred_lft.clone())
                        } else {
                            None
                        };
                    addr
                }),
                Err(e) => {
                    log::warn!(
                        "BUG: nispor got invalid IP address {}, error {}",
                        np_addr.address.as_str(),
                        e
                    );
                }
            }
        }
        ip.addresses = Some(addresses);
        Some(ip)
    } else {
        // IP might just disabled
        let mut ip = InterfaceIpv6::default();
        ip.enabled = false;
        ip.enabled_defined = true;
        Some(ip)
    }
}

pub(crate) fn nipart_ipv4_to_np(
    npt_ipv4: Option<&InterfaceIpv4>,
) -> nispor::IpConf {
    let mut np_ip_conf = nispor::IpConf::default();
    if let Some(npt_ipv4) = npt_ipv4 {
        for npt_addr in npt_ipv4.addresses.as_deref().unwrap_or_default() {
            np_ip_conf.addresses.push({
                let mut ip_conf = nispor::IpAddrConf::default();
                ip_conf.address = npt_addr.ip.to_string();
                ip_conf.prefix_len = npt_addr.prefix_length;
                ip_conf
            });
        }
    }
    np_ip_conf
}

pub(crate) fn nipart_ipv6_to_np(
    npt_ipv6: Option<&InterfaceIpv6>,
) -> nispor::IpConf {
    let mut np_ip_conf = nispor::IpConf::default();
    if let Some(npt_ipv6) = npt_ipv6 {
        for npt_addr in npt_ipv6.addresses.as_deref().unwrap_or_default() {
            np_ip_conf.addresses.push({
                let mut ip_conf = nispor::IpAddrConf::default();
                ip_conf.address = npt_addr.ip.to_string();
                ip_conf.prefix_len = npt_addr.prefix_length;
                ip_conf
            });
        }
    }
    np_ip_conf
}
