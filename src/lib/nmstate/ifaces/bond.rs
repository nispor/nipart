// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original file are:
//  * Gris Ge <fge@redhat.com>
//  * Fernando Fernandez Mancera <ffmancera@riseup.net>
//  * Rahul Rajesh <rajeshrah22@gmail.com>
//  * Jan Vaclav <jvaclav@redhat.com>
//  * Enrique Llorente <ellorent@redhat.com>
//  * Wen Liang <liangwen12year@gmail.com>
//

use std::{collections::HashMap, net::Ipv6Addr};

use serde::{Deserialize, Serialize};

use crate::{
    BaseInterface, ErrorKind, InterfaceType, JsonDisplay, NipartError,
    NipartstateInterface,
};

/// Bond interface.
///
/// When serializing or deserializing, the [BaseInterface] will
/// be flatted and [BondConfig] stored as `link-aggregation` section. The yaml
/// output [crate::NetworkState] containing an example bond interface:
/// ```yml
/// interfaces:
/// - name: bond99
///   type: bond
///   state: up
///   mac-address: 1A:24:D5:CA:76:54
///   mtu: 1500
///   min-mtu: 68
///   max-mtu: 65535
///   wait-ip: any
///   ipv4:
///     enabled: false
///   ipv6:
///     enabled: false
///   accept-all-mac-addresses: false
///   link-aggregation:
///     mode: balance-rr
///     options:
///       all_slaves_active: dropped
///       arp_all_targets: any
///       arp_interval: 0
///       arp_validate: none
///       downdelay: 0
///       lp_interval: 1
///       miimon: 100
///       min_links: 0
///       packets_per_slave: 1
///       primary_reselect: always
///       resend_igmp: 1
///       updelay: 0
///       use_carrier: true
///     port:
///     - eth1
///     - eth2
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonDisplay)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct BondInterface {
    #[serde(flatten)]
    pub base: BaseInterface,
    // TODO(Gris Ge): We should introduce version based query, so we can change
    // this section name from `link-aggregation` to `bond`. I personally
    // dislike the lengthy `link-aggregation` name.
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "link-aggregation",
        alias = "bond"
    )]
    pub bond: Option<BondConfig>,
}

impl BondInterface {
    pub fn new(name: String, bond: BondConfig) -> Self {
        Self {
            base: BaseInterface {
                name: name.to_string(),
                iface_type: InterfaceType::Bond,
                ..Default::default()
            },
            bond: Some(bond),
        }
    }

    pub(crate) fn mode(&self) -> Option<BondMode> {
        self.bond.as_ref().and_then(|b| b.mode)
    }

    // In kernel code drivers/net/bonding/bond_options.c
    // bond_option_queue_id_set(), kernel is not allowing multiple bond port
    // holding the same queue ID, hence we raise error when queue id overlapped.
    fn check_overlap_queue_id(&self) -> Result<(), NipartError> {
        let mut existing_qids: HashMap<u16, &str> = HashMap::new();
        if let Some(ports_conf) =
            self.bond.as_ref().and_then(|b| b.ports_config.as_deref())
        {
            for port_conf in ports_conf
                .iter()
                .filter(|p| p.queue_id.is_some() && p.queue_id != Some(0))
            {
                if let Some(queue_id) = port_conf.queue_id {
                    if let Some(exist_port_name) = existing_qids.get(&queue_id)
                    {
                        return Err(NipartError::new(
                            ErrorKind::InvalidArgument,
                            format!(
                                "Port {} and {} of Bond {} are sharing the \
                                 same queue-id which is not supported by \
                                 linux kernel yet",
                                exist_port_name,
                                port_conf.name.as_str(),
                                self.base.name.as_str()
                            ),
                        ));
                    } else {
                        existing_qids.insert(queue_id, port_conf.name.as_str());
                    }
                }
            }
        }
        Ok(())
    }

    // Remove desired MAC address with warning message when:
    // * New bond desired in mac restricted mode with mac defined
    // * Desire mac address with current interface in mac restricted mode with
    //   desired not changing mac restricted mode
    // The verification process will fail the apply action when desired MAC
    // address differ from port MAC.
    fn sanitize_mac_restricted_mode(&mut self, current: Option<&Self>) {
        let warn_msg = format!(
            "The bond interface {} with fail_over_mac:active and \
             mode:active-backup is determined by its ports, hence ignoring \
             desired MAC address",
            self.base.name.as_str()
        );

        if let Some(current) = current {
            if current.is_mac_restricted_mode()
                && self.base.mac_address.is_some()
                && !self.is_not_mac_restricted_mode_explicitly()
            {
                self.base.mac_address = None;
                log::warn!("{warn_msg}");
            }
        } else if self.is_mac_restricted_mode()
            && self.base.mac_address.is_some()
        {
            self.base.mac_address = None;
            log::warn!("{warn_msg}");
        }
    }

    fn is_mac_restricted_mode(&self) -> bool {
        self.bond
            .as_ref()
            .and_then(|bond_conf| {
                if self.mode() == Some(BondMode::ActiveBackup) {
                    bond_conf.options.as_ref()
                } else {
                    None
                }
            })
            .and_then(|bond_opts| bond_opts.fail_over_mac)
            == Some(BondFailOverMac::Active)
    }

    fn is_not_mac_restricted_mode_explicitly(&self) -> bool {
        (self.mode().is_some() && self.mode() != Some(BondMode::ActiveBackup))
            || ![None, Some(BondFailOverMac::Active)].contains(
                &self
                    .bond
                    .as_ref()
                    .and_then(|bond_conf| bond_conf.options.as_ref())
                    .and_then(|bond_opts| bond_opts.fail_over_mac),
            )
    }
}

impl Default for BondInterface {
    fn default() -> Self {
        Self {
            base: BaseInterface {
                iface_type: InterfaceType::Bond,
                ..Default::default()
            },
            bond: None,
        }
    }
}

impl NipartstateInterface for BondInterface {
    fn base_iface(&self) -> &BaseInterface {
        &self.base
    }

    fn base_iface_mut(&mut self) -> &mut BaseInterface {
        &mut self.base
    }

    fn is_virtual(&self) -> bool {
        true
    }

    fn ports(&self) -> Option<Vec<&str>> {
        self.bond.as_ref().and_then(|bond_conf| {
            bond_conf
                .port
                .as_ref()
                .map(|ports| {
                    ports.as_slice().iter().map(|p| p.as_str()).collect()
                })
                .or_else(|| {
                    bond_conf.ports_config.as_ref().map(|ports| {
                        ports
                            .as_slice()
                            .iter()
                            .map(|p| p.name.as_str())
                            .collect()
                    })
                })
        })
    }

    /// * Bond mode is mandatory for new bond.
    /// * Sort ports list.
    /// * Change `ad_actor_system` to upper case.
    /// * Check overlap on queue ID.
    /// * Check conflict of `miimon` vs `arp_interval`.
    /// * Validate LACP options.
    /// * Validate `ad_actor_system` is not multicast address.
    /// * Validate `arp_interval`.
    /// * Validate conflict between `miimon` and `arp_interval`.
    /// * Validate `balance_slb` option.
    /// * Validate conflict between `num_grat_arp` and `num_grat_arp`
    fn sanitize_iface_specfic(
        &mut self,
        current: Option<&Self>,
    ) -> Result<(), NipartError> {
        let Some(bond_mode) = self
            .mode()
            .or_else(|| current.as_ref().and_then(|c| c.mode()))
        else {
            return Err(NipartError::new(
                ErrorKind::InvalidArgument,
                format!(
                    "Bond mode is mandatory for creating new bond {}",
                    self.name()
                ),
            ));
        };

        if let Some(bond_conf) = self.bond.as_mut() {
            if let Some(ports) = bond_conf.port.as_mut() {
                ports.sort_unstable();
            }
            if let Some(ports_config) = bond_conf.ports_config.as_mut() {
                ports_config.sort_unstable_by_key(|p| p.name.clone());
            }

            if let Some(bond_opts) = bond_conf.options.as_mut() {
                let cur_bond_opts = current
                    .as_ref()
                    .and_then(|c| c.bond.as_ref())
                    .and_then(|b| b.options.as_ref());
                bond_opts.validate_ad_actor_system_mac_address()?;
                bond_opts.validate_lacp_opts(bond_mode)?;
                bond_opts.validate_arp_interval(cur_bond_opts, bond_mode)?;
                bond_opts.validate_miimon_and_arp_interval()?;
                bond_opts.validate_balance_slb(cur_bond_opts, bond_mode)?;

                if let Some(num_grat_arp) = bond_opts.num_grat_arp
                    && let Some(num_unsol_na) = bond_opts.num_unsol_na
                    && num_grat_arp != num_unsol_na
                {
                    return Err(NipartError::new(
                        ErrorKind::InvalidArgument,
                        format!(
                            "The `num_unsol_na` and `num_grat_arp` are \
                             sharing the same meaning in kernel, but desired \
                             with different values for interface {}: \
                             num_grat_arp {num_grat_arp} vs {num_unsol_na}",
                            self.name(),
                        ),
                    ));
                }

                bond_opts.ad_actor_system = bond_opts
                    .ad_actor_system
                    .as_ref()
                    .map(|mac| mac.to_uppercase());
            }
        }
        self.check_overlap_queue_id()?;
        self.sanitize_mac_restricted_mode(current);
        Ok(())
    }

    // TODO: Include bond port name when bond port config changed.
    fn include_diff_context_iface_specific(
        &mut self,
        _desired: &Self,
        _current: &Self,
    ) {
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonDisplay,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct BondConfig {
    /// Mode is mandatory when create new bond interface.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_enum_string_or_integer"
    )]
    pub mode: Option<BondMode>,
    /// When applying, if defined, it will override current port list.
    /// The verification will not fail on bond options miss-match but an
    /// warning message.
    /// Please refer to [kernel documentation](https://www.kernel.org/doc/Documentation/networking/bonding.txt) for detail
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<BondOptions>,
    /// Deserialize and serialize from/to `port`.
    /// You can also use `ports` for deserializing.
    /// When applying, if defined, it will override current port list.
    #[serde(skip_serializing_if = "Option::is_none", alias = "ports")]
    pub port: Option<Vec<String>>,
    /// Deserialize and serialize from/to `ports-config`.
    /// When applying, if defined, it will override current ports
    /// configuration. Note that `port` is not required to set with
    /// `ports-config`. An error will be raised during apply when the port
    /// names specified in `port` and `ports-config` conflict with each
    /// other.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports_config: Option<Vec<BondPortConfig>>,
}

/// Bond mode
#[derive(
    Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, JsonDisplay,
)]
#[non_exhaustive]
#[derive(Default)]
pub enum BondMode {
    #[serde(rename = "balance-rr", alias = "0")]
    /// Deserialize and serialize from/to `balance-rr`.
    /// You can use integer 0 for deserializing to this mode.
    #[default]
    RoundRobin,
    #[serde(rename = "active-backup", alias = "1")]
    /// Deserialize and serialize from/to `active-backup`.
    /// You can use integer 1 for deserializing to this mode.
    ActiveBackup,
    #[serde(rename = "balance-xor", alias = "2")]
    /// Deserialize and serialize from/to `balance-xor`.
    /// You can use integer 2 for deserializing to this mode.
    XOR,
    #[serde(rename = "broadcast", alias = "3")]
    /// Deserialize and serialize from/to `broadcast`.
    /// You can use integer 3 for deserializing to this mode.
    Broadcast,
    #[serde(rename = "802.3ad", alias = "lacp", alias = "4")]
    /// Deserialize and serialize from/to `802.3ad`.
    /// You can use integer 4, or the alias "lacp" for deserializing to this
    /// mode.
    LACP,
    #[serde(rename = "balance-tlb", alias = "5")]
    /// Deserialize and serialize from/to `balance-tlb`.
    /// You can use integer 5 for deserializing to this mode.
    TLB,
    /// Deserialize and serialize from/to `balance-alb`.
    /// You can use integer 6 for deserializing to this mode.
    #[serde(rename = "balance-alb", alias = "6")]
    ALB,
    #[serde(rename = "unknown")]
    Unknown,
}

/// Bond specific configurations
///
/// Please refer to [kernel documentation](https://www.kernel.org/doc/Documentation/networking/bonding.txt)
/// for detail.
#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, JsonDisplay,
)]
// Do not use serde rename `kebab-case` here, because we need to align with
// linux kernel option name.
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct BondOptions {
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u16_or_string"
    )]
    pub ad_actor_sys_prio: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ad_actor_system: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_enum_string_or_integer"
    )]
    pub ad_select: Option<BondAdSelect>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u16_or_string"
    )]
    pub ad_user_port_key: Option<u16>,
    /// Equal to kernel bond option `all_slaves_active`.
    /// Deserialize from `all_ports_active` or `all_slaves_active`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_enum_string_or_integer",
        rename = "all_slaves_active",
        alias = "all_ports_active"
    )]
    pub all_ports_active: Option<BondAllPortActive>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_enum_string_or_integer"
    )]
    pub arp_all_targets: Option<BondArpAllTargets>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub arp_interval: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arp_ip_target: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_enum_string_or_integer"
    )]
    pub arp_validate: Option<BondArpValidate>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub downdelay: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_enum_string_or_integer"
    )]
    pub fail_over_mac: Option<BondFailOverMac>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_enum_string_or_integer"
    )]
    pub lacp_rate: Option<BondLacpRate>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub lp_interval: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub miimon: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub min_links: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u8_or_string"
    )]
    pub num_grat_arp: Option<u8>,
    /// Identical to [BondOptions.num_grat_arp]
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u8_or_string"
    )]
    pub num_unsol_na: Option<u8>,
    /// Equal to kernel bond option `packets_per_slave`.
    ///
    /// This property deserialize from `packet_per_port` or
    /// `packets_per_slave`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string",
        rename = "packets_per_slave",
        alias = "packets_per_port"
    )]
    pub packets_per_port: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub peer_notif_delay: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_enum_string_or_integer"
    )]
    pub primary_reselect: Option<BondPrimaryReselect>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub resend_igmp: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub tlb_dynamic_lb: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub updelay: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub use_carrier: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_enum_string_or_integer"
    )]
    pub xmit_hash_policy: Option<BondXmitHashPolicy>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string",
        alias = "balance-slb"
    )]
    pub balance_slb: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u8_or_string"
    )]
    pub arp_missed_max: Option<u8>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub lacp_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ns_ip6_target: Option<Vec<Ipv6Addr>>,
}

impl BondOptions {
    pub fn new() -> Self {
        Self::default()
    }

    fn validate_ad_actor_system_mac_address(&self) -> Result<(), NipartError> {
        if let Some(ad_actor_system) = &self.ad_actor_system {
            if ad_actor_system.to_uppercase().starts_with("01:00:5E") {
                let e = NipartError::new(
                    ErrorKind::InvalidArgument,
                    "The ad_actor_system bond option cannot be an IANA \
                     multicast address(prefix with 01:00:5E)"
                        .to_string(),
                );
                log::error!("{e}");
                return Err(e);
            }
        }
        Ok(())
    }

    fn validate_miimon_and_arp_interval(&self) -> Result<(), NipartError> {
        if let (Some(miimon), Some(arp_interval)) =
            (self.miimon, self.arp_interval)
        {
            if miimon > 0 && arp_interval > 0 {
                let e = NipartError::new(
                    ErrorKind::InvalidArgument,
                    "Bond miimon and arp interval are not compatible options."
                        .to_string(),
                );
                log::error!("{e}");
                return Err(e);
            }
        }
        Ok(())
    }

    fn validate_balance_slb(
        &self,
        current: Option<&Self>,
        mode: BondMode,
    ) -> Result<(), NipartError> {
        if self
            .balance_slb
            .or_else(|| current.and_then(|c| c.balance_slb))
            == Some(true)
        {
            let xmit_hash_policy = self
                .xmit_hash_policy
                .or_else(|| current.and_then(|c| c.xmit_hash_policy));
            if mode != BondMode::XOR
                || xmit_hash_policy != Some(BondXmitHashPolicy::VlanSrcMac)
            {
                return Err(NipartError::new(
                    ErrorKind::InvalidArgument,
                    "To enable balance-slb, bond mode should be balance-xor \
                     and xmit_hash_policy: 'vlan+srcmac'"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    fn validate_lacp_opts(&self, mode: BondMode) -> Result<(), NipartError> {
        if mode != BondMode::LACP {
            if self.lacp_rate.is_some() {
                return Err(NipartError::new(
                    ErrorKind::InvalidArgument,
                    "The `lacp_rate` option is only valid for bond in \
                     '802.3ad' mode"
                        .to_string(),
                ));
            }
            if self.lacp_active.is_some() {
                return Err(NipartError::new(
                    ErrorKind::InvalidArgument,
                    "The `lacp_active` option is only valid for bond in \
                     '802.3ad' mode"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    // `arp_interval` should bigger than 0 when `arp_ip_target` or
    // `ns_ip6_target` defined.
    // The `arp_interval` cannot be used in `802.3ad`, `balance-tlb`
    // `balance-alb` mode.
    fn validate_arp_interval(
        &self,
        current: Option<&Self>,
        mode: BondMode,
    ) -> Result<(), NipartError> {
        if let Some(arp_interval) = self
            .arp_interval
            .or_else(|| current.and_then(|c| c.arp_interval))
        {
            if arp_interval > 0 {
                if mode == BondMode::LACP
                    || mode == BondMode::TLB
                    || mode == BondMode::ALB
                {
                    return Err(NipartError::new(
                        ErrorKind::InvalidArgument,
                        "The `arp_interval` option is invalid for bond in \
                         '802.3ad', 'balance-tlb' or 'balance-alb' mode"
                            .to_string(),
                    ));
                }
            } else {
                if let Some(arp_ip_target) = self.arp_ip_target.as_ref() {
                    if !arp_ip_target.is_empty() {
                        return Err(NipartError::new(
                            ErrorKind::InvalidArgument,
                            "The `arp_ip_target` option is only valid when \
                             'arp_interval' is enabled(>0)."
                                .to_string(),
                        ));
                    }
                }
                if let Some(ns_ip6_target) = self.ns_ip6_target.as_ref() {
                    if !ns_ip6_target.is_empty() {
                        return Err(NipartError::new(
                            ErrorKind::InvalidArgument,
                            "The `ns_ip6_target` option is only valid when \
                             'arp_interval' is enabled(>0)."
                                .to_string(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct BondPortConfig {
    /// name is mandatory when specifying the ports configuration.
    pub name: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_i32_or_string"
    )]
    /// Deserialize and serialize from/to `priority`.
    /// When applying, if defined, it will override the current bond port
    /// priority. The verification will fail if bonding mode is not
    /// active-backup(1) or balance-tlb (5) or balance-alb (6).
    pub priority: Option<i32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u16_or_string"
    )]
    /// Deserialize and serialize from/to `queue-id`.
    pub queue_id: Option<u16>,
}

impl std::fmt::Display for BondPortConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BondPortConfig {{ name: {}, priority: {}, queue_id: {} }}",
            self.name,
            self.priority.unwrap_or_default(),
            self.queue_id.unwrap_or_default()
        )
    }
}

impl BondPortConfig {
    pub(crate) fn is_name_only(&self) -> bool {
        matches!(
            self,
            &Self {
                name: _,
                priority: None,
                queue_id: None
            }
        )
    }
}

/// Specifies the 802.3ad aggregation selection logic to use.
#[derive(
    Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, JsonDisplay,
)]
#[non_exhaustive]
#[serde(rename_all = "kebab-case")]
pub enum BondAdSelect {
    /// Deserialize and serialize from/to `stable`.
    #[serde(alias = "0")]
    Stable,
    /// Deserialize and serialize from/to `bandwidth`.
    #[serde(alias = "1")]
    Bandwidth,
    /// Deserialize and serialize from/to `count`.
    #[serde(alias = "2")]
    Count,
}

#[derive(
    Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, JsonDisplay,
)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
/// Option specifying the rate in which we'll ask our link partner to transmit
/// LACPDU packets in 802.3ad mode
pub enum BondLacpRate {
    /// Request partner to transmit LACPDUs every 30 seconds.
    /// Serialize to `slow`.
    /// Deserialize from 0 or `slow`.
    #[serde(alias = "0")]
    Slow,
    /// Request partner to transmit LACPDUs every 1 second
    /// Serialize to `fast`.
    /// Deserialize from 1 or `fast`.
    #[serde(alias = "1")]
    Fast,
}

#[derive(
    Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, JsonDisplay,
)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
/// Equal to kernel `all_slaves_active` option.
/// Specifies that duplicate frames (received on inactive ports) should be
/// dropped (0) or delivered (1).
pub enum BondAllPortActive {
    /// Drop the duplicate frames
    /// Serialize to `dropped`.
    /// Deserialize from 0 or `dropped`.
    #[serde(alias = "0")]
    Dropped,
    /// Deliver the duplicate frames
    /// Serialize to `delivered`.
    /// Deserialize from 1 or `delivered`.
    #[serde(alias = "1")]
    Delivered,
}

/// The `arp_all_targets` kernel bond option.
///
/// Specifies the quantity of arp_ip_target that must be reachable in order for
/// the ARP monitor to consider a port as being up. This option affects only
/// active-backup mode for ports with arp_validation enabled.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, JsonDisplay)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum BondArpAllTargets {
    /// consider the port up only when any of the `arp_ip_target` is reachable
    #[serde(alias = "0")]
    Any,
    /// consider the port up only when all of the `arp_ip_target` are
    /// reachable
    #[serde(alias = "1")]
    All,
}

/// The `arp_validate` kernel bond option.
///
/// Specifies whether or not ARP probes and replies should be validated in any
/// mode that supports arp monitoring, or whether non-ARP traffic should be
/// filtered (disregarded) for link monitoring purposes.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, JsonDisplay)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum BondArpValidate {
    /// No validation or filtering is performed.
    /// Serialize to `none`.
    /// Deserialize from 0 or `none`.
    #[serde(alias = "0")]
    None,
    /// Validation is performed only for the active port.
    /// Serialize to `active`.
    /// Deserialize from 1 or `active`.
    #[serde(alias = "1")]
    Active,
    /// Validation is performed only for backup ports.
    /// Serialize to `backup`.
    /// Deserialize from 2 or `backup`.
    #[serde(alias = "2")]
    Backup,
    /// Validation is performed for all ports.
    /// Serialize to `all`.
    /// Deserialize from 3 or `all`.
    #[serde(alias = "3")]
    All,
    /// Filtering is applied to all ports. No validation is performed.
    /// Serialize to `filter`.
    /// Deserialize from 4 or `filter`.
    #[serde(alias = "4")]
    Filter,
    /// Filtering is applied to all ports, validation is performed only for
    /// the active port.
    /// Serialize to `filter_active`.
    /// Deserialize from 5 or `filter-active`.
    #[serde(alias = "5")]
    FilterActive,
    /// Filtering is applied to all ports, validation is performed only for
    /// backup port.
    /// Serialize to `filter_backup`.
    /// Deserialize from 6 or `filter_backup`.
    #[serde(alias = "6")]
    FilterBackup,
}

/// The `fail_over_mac` kernel bond option.
///
/// Specifies whether active-backup mode should set all ports to the same MAC
/// address at port attachment (the traditional behavior), or, when enabled,
/// perform special handling of the bond's MAC address in accordance with the
/// selected policy.
#[derive(
    Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, JsonDisplay,
)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum BondFailOverMac {
    /// This setting disables fail_over_mac, and causes bonding to set all
    /// ports of an active-backup bond to the same MAC address at attachment
    /// time.
    /// Serialize to `none`.
    /// Deserialize from 0 or `none`.
    #[serde(alias = "0")]
    None,
    /// The "active" fail_over_mac policy indicates that the MAC address of the
    /// bond should always be the MAC address of the currently active port.
    /// The MAC address of the ports is not changed; instead, the MAC address
    /// of the bond changes during a failover.
    /// Serialize to `active`.
    /// Deserialize from 1 or `active`.
    #[serde(alias = "1")]
    Active,
    /// The "follow" fail_over_mac policy causes the MAC address of the bond to
    /// be selected normally (normally the MAC address of the first port added
    /// to the bond). However, the second and subsequent ports are not set to
    /// this MAC address while they are in a backup role; a port is programmed
    /// with the bond's MAC address at failover time (and the formerly active
    /// port receives the newly active port's MAC address).
    /// Serialize to `follow`.
    /// Deserialize from 2 or `follow`.
    #[serde(alias = "2")]
    Follow,
}

/// The `primary_reselect` kernel bond option.
///
/// Specifies the reselection policy for the primary port. This affects how the
/// primary port is chosen to become the active port when failure of the active
/// port or recovery of the primary port occurs. This option is designed to
/// prevent flip-flopping between the primary port and other ports.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, JsonDisplay)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum BondPrimaryReselect {
    ///The primary port becomes the active port whenever it comes back up.
    /// Serialize to `always`.
    /// Deserialize from 0 or `always`.
    #[serde(alias = "0")]
    Always,
    /// The primary port becomes the active port when it comes back up, if the
    /// speed and duplex of the primary port is better than the speed and
    /// duplex of the current active port.
    /// Serialize to `better`.
    /// Deserialize from 1 or `better`.
    #[serde(alias = "1")]
    Better,
    /// The primary port becomes the active port only if the current active
    /// port fails and the primary port is up.
    /// Serialize to `failure`.
    /// Deserialize from 2 or `failure`.
    #[serde(alias = "2")]
    Failure,
}

/// The `xmit_hash_policy` kernel bond option.
///
/// Selects the transmit hash policy to use for port selection in balance-xor,
/// 802.3ad, and tlb modes.
#[derive(
    Deserialize, Serialize, Debug, PartialEq, Eq, Clone, Copy, JsonDisplay,
)]
#[non_exhaustive]
pub enum BondXmitHashPolicy {
    #[serde(rename = "layer2", alias = "0")]
    /// Serialize to `layer2`.
    /// Deserialize from 0 or `layer2`.
    Layer2,
    #[serde(rename = "layer3+4", alias = "1")]
    /// Serialize to `layer3+4`.
    /// Deserialize from 1 or `layer3+4`.
    Layer34,
    #[serde(rename = "layer2+3", alias = "2")]
    /// Serialize to `layer2+3`.
    /// Deserialize from 2 or `layer2+3`.
    Layer23,
    #[serde(rename = "encap2+3", alias = "3")]
    /// Serialize to `encap2+3`.
    /// Deserialize from 3 or `encap2+3`.
    Encap23,
    #[serde(rename = "encap3+4", alias = "4")]
    /// Serialize to `encap3+4`.
    /// Deserialize from 4 or `encap3+4`.
    Encap34,
    #[serde(rename = "vlan+srcmac", alias = "5")]
    /// Serialize to `vlan+srcmac`.
    /// Deserialize from 5 or `vlan+srcmac`.
    VlanSrcMac,
}
