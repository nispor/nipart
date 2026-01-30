// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, author of original file is:
//  * Gris Ge <fge@redhat.com>

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};

use crate::{
    BaseInterface, ErrorKind, InterfaceIpAddr, InterfaceIpv4, InterfaceIpv6,
    InterfaceState, InterfaceType, JsonDisplay, NipartError,
    NipartstateInterface,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonDisplay)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
/// Holder for interface with unknown interface type defined.
/// During apply action, nmstate can resolve unknown interface to first
/// found interface type.
pub struct LoopbackInterface {
    #[serde(flatten)]
    pub base: BaseInterface,
}

impl LoopbackInterface {
    pub fn new(base: BaseInterface) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }
}

impl Default for LoopbackInterface {
    fn default() -> Self {
        Self {
            base: BaseInterface {
                name: "lo".into(),
                iface_type: InterfaceType::Loopback,
                state: InterfaceState::Up,
                mtu: Some(65536),
                ipv4: Some(InterfaceIpv4 {
                    enabled: Some(true),
                    dhcp: Some(false),
                    addresses: Some(vec![InterfaceIpAddr {
                        ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
                        prefix_length: 8,
                        ..Default::default()
                    }]),
                    ..Default::default()
                }),
                ipv6: Some(InterfaceIpv6 {
                    enabled: Some(true),
                    autoconf: Some(false),
                    dhcp: Some(false),
                    addresses: Some(vec![InterfaceIpAddr {
                        ip: IpAddr::V6(Ipv6Addr::LOCALHOST),
                        prefix_length: 128,
                        ..Default::default()
                    }]),
                }),
                ..Default::default()
            },
        }
    }
}

impl NipartstateInterface for LoopbackInterface {
    fn base_iface(&self) -> &BaseInterface {
        &self.base
    }

    fn base_iface_mut(&mut self) -> &mut BaseInterface {
        &mut self.base
    }

    fn is_virtual(&self) -> bool {
        true
    }

    /// * Loopback interface should always have 127.0.0.1 and ::1 IP address
    ///   regardless what user desired.
    fn sanitize_iface_specfic(
        &mut self,
        _current: Option<&Self>,
    ) -> Result<(), NipartError> {
        let default_ipv4_addr = InterfaceIpAddr {
            ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            prefix_length: 8,
            ..Default::default()
        };
        let default_ipv6_addr = InterfaceIpAddr {
            ip: IpAddr::V6(Ipv6Addr::LOCALHOST),
            prefix_length: 128,
            ..Default::default()
        };
        if let Some(ipv4) = self.base.ipv4.as_mut() {
            if !ipv4.is_enabled() {
                return Err(NipartError::new(
                    ErrorKind::InvalidArgument,
                    "Disabling IPv4 on loopback interface is not allowed"
                        .to_string(),
                ));
            }
            if let Some(addrs) = ipv4.addresses.as_mut()
                && !addrs.contains(&default_ipv4_addr) {
                    log::info!(
                        "Appending 127.0.0.1/8 address to desired IPv4 \
                         addresses of loopback"
                    );
                    addrs.push(default_ipv4_addr);
                }
        }

        if let Some(ipv6) = self.base.ipv6.as_mut() {
            if !ipv6.is_enabled() {
                return Err(NipartError::new(
                    ErrorKind::InvalidArgument,
                    "Disabling IPv6 on loopback interface is not allowed"
                        .to_string(),
                ));
            }
            if let Some(addrs) = ipv6.addresses.as_mut()
                && !addrs.contains(&default_ipv6_addr) {
                    log::info!(
                        "Appending ::1/128 address to desired IPv6 addresses \
                         of loopback"
                    );
                    addrs.push(default_ipv6_addr);
                }
        }
        Ok(())
    }
}
