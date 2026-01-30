// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use super::iface::init_np_iface;
use crate::{
    NipartError,
    nmstate::{BaseInterface, InterfaceIpAddr, InterfaceIpv4, InterfaceIpv6},
};

pub(crate) fn np_ipv4_to_nmstate(
    np_iface: &nispor::Iface,
) -> Option<InterfaceIpv4> {
    if let Some(np_ip) = &np_iface.ipv4 {
        let mut ip = InterfaceIpv4 {
            enabled: Some(!np_ip.addresses.is_empty()),
            ..Default::default()
        };
        if !ip.is_enabled() {
            return Some(ip);
        }
        let mut addresses = Vec::new();
        for np_addr in &np_ip.addresses {
            if np_addr.valid_lft != "forever" {
                ip.dhcp = Some(true);
            }
            match std::net::IpAddr::from_str(np_addr.address.as_str()) {
                Ok(i) => {
                    let addr = InterfaceIpAddr {
                        ip: i,
                        prefix_length: np_addr.prefix_len,
                        valid_life_time: if np_addr.valid_lft != "forever" {
                            Some(np_addr.valid_lft.clone())
                        } else {
                            None
                        },
                        preferred_life_time: if np_addr.preferred_lft
                            != "forever"
                        {
                            Some(np_addr.preferred_lft.clone())
                        } else {
                            None
                        },
                        ..Default::default()
                    };
                    addresses.push(addr);
                }
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
        if ip.dhcp.is_none() {
            ip.dhcp = Some(false);
        }
        Some(ip)
    } else {
        // IP might just disabled
        Some(InterfaceIpv4::default())
    }
}

pub(crate) fn np_ipv6_to_nmstate(
    np_iface: &nispor::Iface,
) -> Option<InterfaceIpv6> {
    if let Some(np_ip) = &np_iface.ipv6 {
        let mut ip = InterfaceIpv6 {
            enabled: Some(!np_ip.addresses.is_empty()),
            ..Default::default()
        };

        if !ip.is_enabled() {
            return Some(ip);
        }
        let mut addresses = Vec::new();
        for np_addr in &np_ip.addresses {
            if np_addr.valid_lft != "forever" {
                if np_addr.prefix_len == 128 {
                    ip.dhcp = Some(true);
                } else {
                    ip.autoconf = Some(true);
                }
            }
            match std::net::IpAddr::from_str(np_addr.address.as_str()) {
                Ok(i) => {
                    let addr = InterfaceIpAddr {
                        ip: i,
                        prefix_length: np_addr.prefix_len,
                        valid_life_time: if np_addr.valid_lft != "forever" {
                            Some(np_addr.valid_lft.clone())
                        } else {
                            None
                        },
                        preferred_life_time: if np_addr.preferred_lft
                            != "forever"
                        {
                            Some(np_addr.preferred_lft.clone())
                        } else {
                            None
                        },
                        ..Default::default()
                    };
                    addresses.push(addr);
                }
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
        if ip.autoconf.is_none() && ip.dhcp.is_none() {
            ip.dhcp = Some(false);
            ip.autoconf = Some(false);
        }
        Some(ip)
    } else {
        // IP might just disabled
        Some(InterfaceIpv6::default())
    }
}

pub(crate) fn apply_iface_ip_changes(
    des_iface: &BaseInterface,
    cur_iface: Option<&BaseInterface>,
) -> Result<Option<nispor::IfaceConf>, NipartError> {
    if des_iface.is_absent() {
        return Ok(None);
    }

    let mut np_iface = init_np_iface(des_iface);

    let init_np_iface = np_iface.clone();

    let empty_iface = des_iface.clone_name_type_only();

    let cur_iface = cur_iface.unwrap_or(&empty_iface);

    if des_iface.ipv4.as_ref() != cur_iface.ipv4.as_ref()
        && let Some(des_ipv4) = des_iface.ipv4.as_ref()
    {
        let mut des_addrs: &[InterfaceIpAddr] = &[];
        if des_ipv4.is_enabled()
            && let Some(d) = des_ipv4.addresses.as_ref()
        {
            des_addrs = d;
        }

        let mut cur_addrs: &[InterfaceIpAddr] = &[];
        if let Some(cur_ipv4) = cur_iface.ipv4.as_ref()
            && cur_ipv4.is_enabled()
                && let Some(c) = cur_ipv4.addresses.as_ref()
            {
                cur_addrs = c;
            }
        let np_addrs = nmstate_ip_addrs_to_nispor(des_addrs, cur_addrs);

        if !np_addrs.is_empty() {
            let mut np_ip_conf = nispor::IpConf::default();
            np_ip_conf.addresses = np_addrs;
            np_iface.ipv4 = Some(np_ip_conf);
        }
    }

    if des_iface.ipv6.as_ref() != cur_iface.ipv6.as_ref()
        && let Some(des_ipv6) = des_iface.ipv6.as_ref()
    {
        let mut des_addrs: &[InterfaceIpAddr] = &[];
        if des_ipv6.is_enabled()
            && let Some(d) = des_ipv6.addresses.as_ref()
        {
            des_addrs = d;
        }

        let mut cur_addrs: &[InterfaceIpAddr] = &[];
        if let Some(cur_ipv6) = cur_iface.ipv6.as_ref()
            && cur_ipv6.is_enabled()
                && let Some(c) = cur_ipv6.addresses.as_ref()
            {
                cur_addrs = c;
            }
        let np_addrs = nmstate_ip_addrs_to_nispor(des_addrs, cur_addrs);

        if !np_addrs.is_empty() {
            let mut np_ip_conf = nispor::IpConf::default();
            np_ip_conf.addresses = np_addrs;
            np_iface.ipv6 = Some(np_ip_conf);
        }
    }

    if np_iface != init_np_iface {
        Ok(Some(np_iface))
    } else {
        Ok(None)
    }
}

fn nmstate_ip_addr_to_nispor(
    ip_addr: &InterfaceIpAddr,
    remove: bool,
) -> nispor::IpAddrConf {
    let mut np_ip_addr = nispor::IpAddrConf::default();
    np_ip_addr.address = ip_addr.ip.to_string();
    np_ip_addr.prefix_len = ip_addr.prefix_length;
    np_ip_addr.preferred_lft = ip_addr
        .preferred_life_time
        .clone()
        .unwrap_or("forever".to_string());
    np_ip_addr.valid_lft = ip_addr
        .valid_life_time
        .clone()
        .unwrap_or("forever".to_string());
    np_ip_addr.remove = remove;

    np_ip_addr
}

fn nmstate_ip_addrs_to_nispor(
    des_addrs: &[InterfaceIpAddr],
    cur_addrs: &[InterfaceIpAddr],
) -> Vec<nispor::IpAddrConf> {
    let mut ret: Vec<nispor::IpAddrConf> = Vec::new();

    if is_appending(des_addrs, cur_addrs) {
        for cur_addr in cur_addrs {
            if !des_addrs.contains(cur_addr) {
                ret.push(nmstate_ip_addr_to_nispor(cur_addr, true));
            }
        }
        for des_addr in des_addrs {
            if !cur_addrs.contains(des_addr) {
                ret.push(nmstate_ip_addr_to_nispor(des_addr, false));
            }
        }
    } else if is_replacing(des_addrs, cur_addrs) {
        for des_addr in des_addrs {
            ret.push(nmstate_ip_addr_to_nispor(des_addr, false));
        }
    } else {
        // Purge all current IP address, so we get expected IP address order.
        for cur_addr in cur_addrs {
            ret.push(nmstate_ip_addr_to_nispor(cur_addr, true));
        }
        for des_addr in des_addrs {
            ret.push(nmstate_ip_addr_to_nispor(des_addr, false));
        }
    }

    ret
}

fn is_appending(
    des_addrs: &[InterfaceIpAddr],
    cur_addrs: &[InterfaceIpAddr],
) -> bool {
    cur_addrs.len() < des_addrs.len()
        && &des_addrs[..cur_addrs.len()] == cur_addrs
}

fn is_replacing(
    des_addrs: &[InterfaceIpAddr],
    cur_addrs: &[InterfaceIpAddr],
) -> bool {
    cur_addrs.len() == des_addrs.len()
        && des_addrs.iter().all(|des_addr| {
            cur_addrs.iter().any(|cur_addr| {
                des_addr.ip == cur_addr.ip
                    && des_addr.prefix_length == cur_addr.prefix_length
            })
        })
}
