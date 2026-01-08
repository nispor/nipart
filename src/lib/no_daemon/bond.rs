// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original file(nispor/bond.rs)
// are:
//  * Gris Ge <fge@redhat.com>
//  * Fernando Fernandez Mancera <ffmancera@riseup.net>
//  * Wen Liang <liangwen12year@gmail.com>

use std::{net::Ipv4Addr, str::FromStr};

use crate::{
    BaseInterface, BondAdSelect, BondAllPortActive, BondArpAllTargets,
    BondArpValidate, BondConfig, BondFailOverMac, BondInterface, BondLacpRate,
    BondMode, BondOptions, BondPortConfig, BondPrimaryReselect,
    BondXmitHashPolicy, ErrorKind, MergedInterfaces, NipartError, NipartstateInterface,
};

impl From<nispor::BondMode> for BondMode {
    fn from(v: nispor::BondMode) -> Self {
        match v {
            nispor::BondMode::BalanceRoundRobin => BondMode::RoundRobin,
            nispor::BondMode::ActiveBackup => BondMode::ActiveBackup,
            nispor::BondMode::BalanceXor => BondMode::XOR,
            nispor::BondMode::Broadcast => BondMode::Broadcast,
            nispor::BondMode::Ieee8021AD => BondMode::LACP,
            nispor::BondMode::BalanceTlb => BondMode::TLB,
            nispor::BondMode::BalanceAlb => BondMode::ALB,
            _ => BondMode::Unknown,
        }
    }
}

impl From<BondMode> for nispor::BondMode {
    fn from(v: BondMode) -> Self {
        match v {
            BondMode::RoundRobin => Self::BalanceRoundRobin,
            BondMode::ActiveBackup => Self::ActiveBackup,
            BondMode::XOR => Self::BalanceXor,
            BondMode::Broadcast => Self::Broadcast,
            BondMode::LACP => Self::Ieee8021AD,
            BondMode::TLB => Self::BalanceTlb,
            BondMode::ALB => Self::BalanceAlb,
            _ => {
                log::warn!("Unknown bond mode {v}, treat as active-backup");
                Self::ActiveBackup
            }
        }
    }
}

impl From<BondArpAllTargets> for nispor::BondModeArpAllTargets {
    fn from(v: BondArpAllTargets) -> Self {
        match v {
            BondArpAllTargets::All => Self::All,
            BondArpAllTargets::Any => Self::Any,
        }
    }
}

impl From<BondArpValidate> for nispor::BondArpValidate {
    fn from(v: BondArpValidate) -> Self {
        match v {
            BondArpValidate::None => Self::None,
            BondArpValidate::Active => Self::Active,
            BondArpValidate::Backup => Self::Backup,
            BondArpValidate::All => Self::All,
            BondArpValidate::Filter => Self::Filter,
            BondArpValidate::FilterActive => Self::FilterActive,
            BondArpValidate::FilterBackup => Self::FilterBackup,
        }
    }
}

impl From<BondPrimaryReselect> for nispor::BondPrimaryReselect {
    fn from(v: BondPrimaryReselect) -> Self {
        match v {
            BondPrimaryReselect::Always => Self::Always,
            BondPrimaryReselect::Better => Self::Better,
            BondPrimaryReselect::Failure => Self::Failure,
        }
    }
}

impl From<BondFailOverMac> for nispor::BondFailOverMac {
    fn from(v: BondFailOverMac) -> Self {
        match v {
            BondFailOverMac::None => Self::None,
            BondFailOverMac::Active => Self::Active,
            BondFailOverMac::Follow => Self::Follow,
        }
    }
}

impl From<BondXmitHashPolicy> for nispor::BondXmitHashPolicy {
    fn from(v: BondXmitHashPolicy) -> Self {
        match v {
            BondXmitHashPolicy::Layer2 => Self::Layer2,
            BondXmitHashPolicy::Layer34 => Self::Layer34,
            BondXmitHashPolicy::Layer23 => Self::Layer23,
            BondXmitHashPolicy::Encap23 => Self::Encap23,
            BondXmitHashPolicy::Encap34 => Self::Encap34,
            BondXmitHashPolicy::VlanSrcMac => Self::VlanSrcMac,
        }
    }
}

impl From<BondAllPortActive> for nispor::BondAllPortActive {
    fn from(v: BondAllPortActive) -> Self {
        match v {
            BondAllPortActive::Dropped => Self::Dropped,
            BondAllPortActive::Delivered => Self::Delivered,
        }
    }
}

impl From<BondLacpRate> for nispor::BondLacpRate {
    fn from(v: BondLacpRate) -> Self {
        match v {
            BondLacpRate::Fast => Self::Fast,
            BondLacpRate::Slow => Self::Slow,
        }
    }
}

impl From<BondAdSelect> for nispor::BondAdSelect {
    fn from(v: BondAdSelect) -> Self {
        match v {
            BondAdSelect::Stable => Self::Stable,
            BondAdSelect::Bandwidth => Self::Bandwidth,
            BondAdSelect::Count => Self::Count,
        }
    }
}

impl From<&nispor::BondInfo> for BondConfig {
    fn from(np_bond: &nispor::BondInfo) -> Self {
        BondConfig {
            mode: Some(np_bond.mode.into()),
            options: Some(np_bond_options_to_nmstate(np_bond)),
            port: Some(
                np_bond
                    .ports
                    .as_slice()
                    .iter()
                    .map(|iface_name| iface_name.to_string())
                    .collect(),
            ),
            ports_config: Some(
                np_bond
                    .ports
                    .as_slice()
                    .iter()
                    .map(|iface_name| BondPortConfig {
                        name: iface_name.to_string(),
                        ..Default::default()
                    })
                    .collect(),
            ),
        }
    }
}

impl From<&BondPortConfig> for nispor::BondPortConf {
    fn from(port_conf: &BondPortConfig) -> Self {
        let mut ret = Self::default();
        ret.queue_id = port_conf.queue_id;
        ret.prio = port_conf.priority;
        ret
    }
}

/// Special cases:
///  * Change bond mode need to detach all ports and in down state.
pub(crate) fn apply_bond_conf(
    mut np_iface: nispor::IfaceConf,
    iface: &BondInterface,
    cur_iface: Option<&BondInterface>,
    merged_ifaces: &MergedInterfaces,
) -> Result<Vec<nispor::IfaceConf>, NipartError> {
    let mut ret = Vec::new();
    if let Some(cur_iface) = cur_iface
        && is_bond_mode_changed(iface, cur_iface)
    {
        log::info!(
            "Interface {} is changing bond mode require detaching all bond \
             ports and bring bond down",
            iface.name()
        );
        // Detach bond ports
        if let Some(cur_ports) = cur_iface.ports() {
            for port in cur_ports {
                let mut port_np_iface = nispor::IfaceConf::default();
                port_np_iface.name = port.to_string();
                port_np_iface.state = nispor::IfaceState::Up;
                port_np_iface.controller = Some(String::new());
                ret.push(port_np_iface);
            }
        }
        // Change bond mode
        let mut bond_mode_np_iface = nispor::IfaceConf::default();
        bond_mode_np_iface.iface_type = Some(nispor::IfaceType::Bond);
        bond_mode_np_iface.name = iface.name().to_string();
        let mut np_bond_conf = nispor::BondConf::default();
        np_bond_conf.mode =
            iface.bond.as_ref().and_then(|b| b.mode.map(BondMode::into));
        bond_mode_np_iface.bond = Some(np_bond_conf);
        ret.push(bond_mode_np_iface);

        // Reattach bond ports
        if let Some(ports) = iface.ports().or_else(|| cur_iface.ports()) {
            for port in ports {
                // Only re-attach bond port which previously exist.
                // For example, when attaching newly create VLAN to bond along
                // with bond mode changes, after we detach all existing bond
                // ports, we cannot attach this not-exist VLAN yet.
                if merged_ifaces
                    .kernel_ifaces
                    .get(port)
                    .map(|merged_iface| merged_iface.current.is_some())
                    == Some(true)
                {
                    let mut port_np_iface = nispor::IfaceConf::default();
                    port_np_iface.name = port.to_string();
                    port_np_iface.state = nispor::IfaceState::Up;
                    port_np_iface.controller = Some(iface.name().to_string());
                    ret.push(port_np_iface);
                }
            }
        }
    }

    if let Some(bond_conf) = iface.bond.as_ref() {
        let mut np_bond_conf = nispor::BondConf::default();
        if cur_iface.is_none() {
            np_bond_conf.mode = bond_conf.mode.map(BondMode::into);
        }
        if let Some(bond_opts) = bond_conf.options.as_ref() {
            np_bond_conf.miimon = bond_opts.miimon;
            np_bond_conf.updelay = bond_opts.updelay;
            np_bond_conf.downdelay = bond_opts.downdelay;
            np_bond_conf.use_carrier = bond_opts.use_carrier;
            np_bond_conf.arp_interval = bond_opts.arp_interval;
            if let Some(arp_ip_target) = bond_opts.arp_ip_target.as_deref() {
                let mut ip_addrs = Vec::new();
                for ip_addr_str in arp_ip_target.split(",") {
                    ip_addrs.push(Ipv4Addr::from_str(ip_addr_str).map_err(
                        |_| {
                            NipartError::new(
                                ErrorKind::InvalidArgument,
                                format!(
                                    "Invalid bond `arp_ip_target` property, \
                                     should be IPv4 addresses separated by \
                                     ',', but got {arp_ip_target}"
                                ),
                            )
                        },
                    )?);
                }
                np_bond_conf.arp_ip_target = Some(ip_addrs);
            }
            np_bond_conf.arp_all_targets = bond_opts
                .arp_all_targets
                .clone()
                .map(BondArpAllTargets::into);
            np_bond_conf.arp_validate =
                bond_opts.arp_validate.clone().map(BondArpValidate::into);
            np_bond_conf.primary = bond_opts.primary.clone();
            np_bond_conf.primary_reselect = bond_opts
                .primary_reselect
                .clone()
                .map(BondPrimaryReselect::into);
            np_bond_conf.fail_over_mac =
                bond_opts.fail_over_mac.map(BondFailOverMac::into);
            np_bond_conf.xmit_hash_policy =
                bond_opts.xmit_hash_policy.map(BondXmitHashPolicy::into);
            np_bond_conf.resend_igmp = bond_opts.resend_igmp;
            np_bond_conf.num_unsol_na = bond_opts.num_unsol_na;
            np_bond_conf.all_ports_active =
                bond_opts.all_ports_active.map(BondAllPortActive::into);
            np_bond_conf.min_links = bond_opts.min_links;
            np_bond_conf.lp_interval = bond_opts.lp_interval;
            np_bond_conf.packets_per_port = bond_opts.packets_per_port;
            np_bond_conf.lacp_rate =
                bond_opts.lacp_rate.map(BondLacpRate::into);
            np_bond_conf.ad_select =
                bond_opts.ad_select.map(BondAdSelect::into);
            np_bond_conf.ad_actor_sys_prio = bond_opts.ad_actor_sys_prio;
            np_bond_conf.ad_user_port_key = bond_opts.ad_user_port_key;
            np_bond_conf.ad_actor_system = bond_opts.ad_actor_system.clone();
            np_bond_conf.tlb_dynamic_lb = bond_opts.tlb_dynamic_lb;
            np_bond_conf.peer_notif_delay = bond_opts.peer_notif_delay;
            np_bond_conf.lacp_active = bond_opts.lacp_active;
            np_bond_conf.arp_missed_max = bond_opts.arp_missed_max;
            np_bond_conf.ns_ip6_target = bond_opts.ns_ip6_target.clone();
        }

        if np_bond_conf != Default::default() {
            np_iface.bond = Some(np_bond_conf);
        }
    }

    ret.push(np_iface);

    Ok(ret)
}

fn is_bond_mode_changed(
    des_iface: &BondInterface,
    cur_iface: &BondInterface,
) -> bool {
    if let Some(des_mode) = des_iface.bond.as_ref().and_then(|b| b.mode)
        && let Some(cur_mode) = cur_iface.bond.as_ref().and_then(|b| b.mode)
        && des_mode != cur_mode
    {
        true
    } else {
        false
    }
}

impl BondInterface {
    pub(crate) fn new_from_nispor(
        base_iface: BaseInterface,
        np_iface: &nispor::Iface,
    ) -> Self {
        if let Some(np_bond_conf) = np_iface.bond.as_ref() {
            Self {
                base: base_iface,
                bond: Some(np_bond_conf.into()),
            }
        } else {
            Self {
                base: base_iface,
                ..Default::default()
            }
        }
    }

    pub(crate) fn append_bond_port_config(
        &mut self,
        port_np_ifaces: Vec<&nispor::Iface>,
    ) {
        let mut port_confs: Vec<BondPortConfig> = Vec::new();
        for port_np_iface in port_np_ifaces {
            port_confs.push(BondPortConfig {
                name: port_np_iface.name.to_string(),
                priority: port_np_iface.bond_port.as_ref().map(|p| p.prio),
                queue_id: port_np_iface.bond_port.as_ref().map(|p| p.queue_id),
            })
        }

        if let Some(bond_conf) = self.bond.as_mut() {
            bond_conf.ports_config = Some(port_confs);
        }
    }

    pub(crate) fn apply_bond_port_configs(&self) -> Vec<nispor::IfaceConf> {
        let mut ret: Vec<nispor::IfaceConf> = Vec::new();
        if let Some(ports_conf) =
            self.bond.as_ref().and_then(|b| b.ports_config.as_ref())
        {
            for port_conf in ports_conf.iter().filter(|p| !p.is_name_only()) {
                let np_bond_port_conf: nispor::BondPortConf = port_conf.into();
                let mut port_np_iface = nispor::IfaceConf::default();
                port_np_iface.name = port_conf.name.to_string();
                port_np_iface.bond_port = Some(np_bond_port_conf);
                ret.push(port_np_iface);
            }
        }
        ret
    }
}

fn np_bond_options_to_nmstate(np_bond: &nispor::BondInfo) -> BondOptions {
    BondOptions {
        ad_actor_sys_prio: np_bond.ad_actor_sys_prio,
        ad_actor_system: np_bond.ad_actor_system.clone(),
        ad_select: np_bond.ad_select.as_ref().and_then(|r| match r {
            nispor::BondAdSelect::Stable => Some(BondAdSelect::Stable),
            nispor::BondAdSelect::Bandwidth => Some(BondAdSelect::Bandwidth),
            nispor::BondAdSelect::Count => Some(BondAdSelect::Count),
            _ => {
                log::warn!("Unsupported bond ad_select option {r:?}");
                None
            }
        }),
        ad_user_port_key: np_bond.ad_user_port_key,
        all_ports_active: np_bond.all_ports_active.as_ref().and_then(
            |r| match r {
                nispor::BondAllPortActive::Dropped => {
                    Some(BondAllPortActive::Dropped)
                }
                nispor::BondAllPortActive::Delivered => {
                    Some(BondAllPortActive::Delivered)
                }
                _ => {
                    log::warn!(
                        "Unsupported bond all ports active options {r:?}"
                    );
                    None
                }
            },
        ),
        arp_all_targets: np_bond.arp_all_targets.as_ref().and_then(
            |r| match r {
                nispor::BondModeArpAllTargets::Any => {
                    Some(BondArpAllTargets::Any)
                }
                nispor::BondModeArpAllTargets::All => {
                    Some(BondArpAllTargets::All)
                }
                _ => {
                    log::warn!("Unsupported bond arp_all_targets option {r:?}");
                    None
                }
            },
        ),
        arp_interval: np_bond.arp_interval,
        arp_ip_target: np_bond.arp_ip_target.clone(),
        arp_validate: np_bond.arp_validate.as_ref().and_then(|r| match r {
            nispor::BondArpValidate::None => Some(BondArpValidate::None),
            nispor::BondArpValidate::Active => Some(BondArpValidate::Active),
            nispor::BondArpValidate::Backup => Some(BondArpValidate::Backup),
            nispor::BondArpValidate::All => Some(BondArpValidate::All),
            nispor::BondArpValidate::FilterActive => {
                Some(BondArpValidate::FilterActive)
            }
            nispor::BondArpValidate::FilterBackup => {
                Some(BondArpValidate::FilterBackup)
            }
            _ => {
                log::warn!("Unsupported bond arp_validate options {r:?}");
                None
            }
        }),
        downdelay: np_bond.downdelay,
        fail_over_mac: np_bond.fail_over_mac.as_ref().and_then(|r| match r {
            nispor::BondFailOverMac::None => Some(BondFailOverMac::None),
            nispor::BondFailOverMac::Active => Some(BondFailOverMac::Active),
            nispor::BondFailOverMac::Follow => Some(BondFailOverMac::Follow),
            _ => {
                log::warn!("Unsupported bond fail_over_mac options {r:?}");
                None
            }
        }),
        lacp_rate: np_bond.lacp_rate.as_ref().and_then(|r| match r {
            nispor::BondLacpRate::Slow => Some(BondLacpRate::Slow),
            nispor::BondLacpRate::Fast => Some(BondLacpRate::Fast),
            _ => {
                log::warn!("Unsupported bond lacp_rate options {r:?}");
                None
            }
        }),
        lp_interval: np_bond.lp_interval,
        miimon: np_bond.miimon,
        min_links: np_bond.min_links,
        num_unsol_na: np_bond.num_grat_arp,
        num_grat_arp: np_bond.num_grat_arp,
        packets_per_port: np_bond.packets_per_port,
        primary: np_bond.primary.clone(),
        primary_reselect: np_bond.primary_reselect.as_ref().and_then(
            |r| match r {
                nispor::BondPrimaryReselect::Always => {
                    Some(BondPrimaryReselect::Always)
                }
                nispor::BondPrimaryReselect::Better => {
                    Some(BondPrimaryReselect::Better)
                }
                nispor::BondPrimaryReselect::Failure => {
                    Some(BondPrimaryReselect::Failure)
                }
                _ => {
                    log::warn!(
                        "Unsupported bond primary_reselect options {r:?}"
                    );
                    None
                }
            },
        ),
        resend_igmp: np_bond.resend_igmp,
        tlb_dynamic_lb: np_bond.tlb_dynamic_lb,
        updelay: np_bond.updelay,
        use_carrier: np_bond.use_carrier,
        xmit_hash_policy: np_bond.xmit_hash_policy.as_ref().and_then(
            |r| match r {
                nispor::BondXmitHashPolicy::Layer2 => {
                    Some(BondXmitHashPolicy::Layer2)
                }
                nispor::BondXmitHashPolicy::Layer34 => {
                    Some(BondXmitHashPolicy::Layer34)
                }
                nispor::BondXmitHashPolicy::Layer23 => {
                    Some(BondXmitHashPolicy::Layer23)
                }
                nispor::BondXmitHashPolicy::Encap23 => {
                    Some(BondXmitHashPolicy::Encap23)
                }
                nispor::BondXmitHashPolicy::Encap34 => {
                    Some(BondXmitHashPolicy::Encap34)
                }
                nispor::BondXmitHashPolicy::VlanSrcMac => {
                    Some(BondXmitHashPolicy::VlanSrcMac)
                }
                _ => {
                    log::warn!(
                        "Unsupported bond xmit_hash_policy options {r:?}"
                    );
                    None
                }
            },
        ),
        arp_missed_max: np_bond.arp_missed_max,
        lacp_active: np_bond.lacp_active,
        ns_ip6_target: np_bond.ns_ip6_target.clone(),
        peer_notif_delay: np_bond.peer_notif_delay,
        // balance_slb is userspace property
        balance_slb: None,
    }
}
