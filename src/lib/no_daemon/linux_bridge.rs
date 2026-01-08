// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original file
// (rust/src/lib/nispor/linux_bridge.rs) are:
//  * Gris Ge <fge@redhat.com>
//  * Fernando Fernandez Mancera <ffmancera@riseup.net>
//  * Íñigo Huguet <ihuguet@redhat.com>

use super::linux_bridge_vlan::{gen_bridge_vlan_conf, parse_bridge_vlan_conf};
use crate::{
    BaseInterface, ErrorKind, LinuxBridgeConfig, LinuxBridgeInterface,
    LinuxBridgeMulticastRouterType, LinuxBridgeOptions, LinuxBridgePortConfig,
    LinuxBridgeStpOptions, NipartError,
};

impl From<nispor::BridgeMulticastRouterType>
    for LinuxBridgeMulticastRouterType
{
    fn from(v: nispor::BridgeMulticastRouterType) -> Self {
        match v {
            nispor::BridgeMulticastRouterType::Disabled => {
                LinuxBridgeMulticastRouterType::Disabled
            }
            nispor::BridgeMulticastRouterType::TempQuery => {
                LinuxBridgeMulticastRouterType::Auto
            }
            nispor::BridgeMulticastRouterType::Perm => {
                LinuxBridgeMulticastRouterType::Enabled
            }
            _ => {
                log::debug!("Unsupported linux bridge multicast router {v:?}");
                LinuxBridgeMulticastRouterType::Unknown
            }
        }
    }
}

impl From<LinuxBridgeMulticastRouterType>
    for nispor::BridgeMulticastRouterType
{
    fn from(v: LinuxBridgeMulticastRouterType) -> Self {
        match v {
            LinuxBridgeMulticastRouterType::Disabled => Self::Disabled,
            LinuxBridgeMulticastRouterType::Auto => Self::TempQuery,
            LinuxBridgeMulticastRouterType::Enabled => Self::Perm,
            _ => {
                log::debug!(
                    "Unsupported linux bridge multicast router type {v:?}, \
                     treating as auto"
                );
                Self::TempQuery
            }
        }
    }
}

impl From<&nispor::BridgeInfo> for LinuxBridgeConfig {
    fn from(np_bridge: &nispor::BridgeInfo) -> Self {
        Self {
            options: Some(LinuxBridgeOptions {
                gc_timer: np_bridge.gc_timer,
                group_addr: np_bridge
                    .group_addr
                    .as_ref()
                    .map(|s| s.to_uppercase()),
                group_forward_mask: np_bridge.group_fwd_mask,
                group_fwd_mask: np_bridge.group_fwd_mask,
                hash_max: np_bridge.multicast_hash_max,
                hello_timer: np_bridge.hello_timer,
                mac_ageing_time: np_bridge.ageing_time.map(devide_by_user_hz),
                multicast_last_member_count: np_bridge
                    .multicast_last_member_count,
                multicast_last_member_interval: np_bridge
                    .multicast_last_member_interval,
                multicast_membership_interval: np_bridge
                    .multicast_membership_interval,
                multicast_querier: np_bridge.multicast_querier,
                multicast_querier_interval: np_bridge
                    .multicast_querier_interval,
                multicast_query_interval: np_bridge.multicast_query_interval,
                multicast_query_response_interval: np_bridge
                    .multicast_query_response_interval,
                multicast_query_use_ifaddr: np_bridge
                    .multicast_query_use_ifaddr,
                multicast_router: np_bridge
                    .multicast_router
                    .as_ref()
                    .map(|r| (*r).into()),
                multicast_snooping: np_bridge.multicast_snooping,
                multicast_startup_query_count: np_bridge
                    .multicast_startup_query_count,
                multicast_startup_query_interval: np_bridge
                    .multicast_startup_query_interval,
                stp: Some(get_stp_options(np_bridge)),
                vlan_protocol: np_bridge
                    .vlan_protocol
                    .as_ref()
                    .map(|p| (*p).into()),
                vlan_default_pvid: np_bridge.default_pvid,
            }),
            vlan: if np_bridge.vlan_filtering == Some(true) {
                np_bridge.vlans.as_deref().and_then(|vlans| {
                    parse_bridge_vlan_conf(vlans, np_bridge.default_pvid)
                })
            } else {
                None
            },
            ports: Some(
                np_bridge
                    .ports
                    .as_slice()
                    .iter()
                    .map(|iface_name| LinuxBridgePortConfig {
                        name: iface_name.to_string(),
                        ..Default::default()
                    })
                    .collect(),
            ),
        }
    }
}

pub(crate) fn apply_bridge_conf(
    mut np_iface: nispor::IfaceConf,
    iface: &LinuxBridgeInterface,
    cur_iface: Option<&LinuxBridgeInterface>,
) -> Result<Vec<nispor::IfaceConf>, NipartError> {
    let mut np_br = nispor::BridgeConf::default();
    if let Some(br_opts) =
        iface.bridge.as_ref().and_then(|b| b.options.as_ref())
    {
        if let Some(v) = br_opts.group_addr.as_ref() {
            np_br.group_address = Some(parse_eth_mac(v.as_str())?);
        }
        if let Some(v) = br_opts.group_forward_mask.as_ref() {
            np_br.group_fwd_mask = Some(*v);
        }
        if let Some(v) = br_opts.group_fwd_mask.as_ref() {
            np_br.group_fwd_mask = Some(*v);
        }
        if let Some(v) = br_opts.hash_max.as_ref() {
            np_br.mcast_hash_max = Some(*v);
        }
        if let Some(v) = br_opts.mac_ageing_time.as_ref() {
            np_br.ageing_time = Some(*v * get_user_hz());
        }
        if let Some(v) = br_opts.multicast_last_member_count.as_ref() {
            np_br.mcast_last_member_count = Some(*v);
        }
        if let Some(v) = br_opts.multicast_last_member_interval.as_ref() {
            np_br.mcast_last_member_interval = Some(*v);
        }
        if let Some(v) = br_opts.multicast_membership_interval.as_ref() {
            np_br.mcast_membership_interval = Some(*v);
        }
        if let Some(v) = br_opts.multicast_querier.as_ref() {
            np_br.mcast_querier = Some(*v);
        }
        if let Some(v) = br_opts.multicast_querier_interval.as_ref() {
            np_br.mcast_querier_interval = Some(*v);
        }
        if let Some(v) = br_opts.multicast_query_interval.as_ref() {
            np_br.mcast_query_interval = Some(*v);
        }
        if let Some(v) = br_opts.multicast_query_response_interval.as_ref() {
            np_br.mcast_query_response_interval = Some(*v);
        }
        if let Some(v) = br_opts.multicast_query_use_ifaddr.as_ref() {
            np_br.mcast_query_use_ifaddr = Some(*v);
        }
        if let Some(v) = br_opts.multicast_router.as_ref() {
            np_br.mcast_router = Some(v.clone().into());
        }
        if let Some(v) = br_opts.multicast_snooping.as_ref() {
            np_br.mcast_snooping = Some(*v);
        }
        if let Some(v) = br_opts.multicast_startup_query_count.as_ref() {
            np_br.mcast_startup_query_count = Some(*v);
        }
        if let Some(v) = br_opts.multicast_startup_query_interval.as_ref() {
            np_br.mcast_startup_query_interval = Some(*v);
        }
        if let Some(v) = br_opts.vlan_protocol.as_ref() {
            np_br.vlan_protocol = Some((*v).into());
        }
        if let Some(v) = br_opts.vlan_default_pvid.as_ref() {
            np_br.vlan_default_pvid = Some(*v);
        }
        if let Some(stp) = br_opts.stp.as_ref() {
            np_br.stp_state = stp.enabled.map(|enabled| {
                if enabled {
                    nispor::BridgeStpState::KernelStp
                } else {
                    nispor::BridgeStpState::Disabled
                }
            });

            np_br.forward_delay =
                stp.forward_delay.map(|v| v as u32 * get_user_hz());

            np_br.hello_time = stp.hello_time.map(|v| v as u32 * get_user_hz());
            np_br.max_age = stp.max_age.map(|v| v as u32 * get_user_hz());
            np_br.priority = stp.priority;
        }
    }
    if let Some(vlan_conf) = iface
        .bridge
        .as_ref()
        .and_then(|br_conf| br_conf.vlan.as_ref())
    {
        np_br.vlans = Some(gen_bridge_vlan_conf(
            vlan_conf,
            cur_iface
                .and_then(|c| c.bridge.as_ref())
                .and_then(|cur_br_conf| cur_br_conf.vlan.as_ref()),
        ));
    }
    np_br.vlan_filtering = Some(iface.vlan_filtering_is_enabled(cur_iface));

    if let Some(br_conf) = iface.bridge.as_ref()
        && (br_conf.options.is_some() || br_conf.vlan.is_some())
    {
        np_iface.bridge = Some(np_br);
    }
    Ok(vec![np_iface])
}

impl LinuxBridgeInterface {
    pub(crate) fn new_from_nispor(
        base_iface: BaseInterface,
        np_iface: &nispor::Iface,
    ) -> Self {
        if let Some(np_bridge_conf) = np_iface.bridge.as_ref() {
            Self {
                base: base_iface,
                bridge: Some(np_bridge_conf.into()),
            }
        } else {
            Self {
                base: base_iface,
                ..Default::default()
            }
        }
    }

    pub(crate) fn append_br_port_config(
        &mut self,
        np_iface: &nispor::Iface,
        port_np_ifaces: Vec<&nispor::Iface>,
    ) {
        let mut port_confs: Vec<LinuxBridgePortConfig> = Vec::new();
        for port_np_iface in port_np_ifaces {
            let mut port_conf = LinuxBridgePortConfig {
                name: port_np_iface.name.to_string(),
                stp_hairpin_mode: port_np_iface
                    .bridge_port
                    .as_ref()
                    .map(|i| i.hairpin_mode),
                stp_path_cost: port_np_iface
                    .bridge_port
                    .as_ref()
                    .map(|i| i.stp_path_cost),
                stp_priority: port_np_iface
                    .bridge_port
                    .as_ref()
                    .map(|i| i.stp_priority),
                ..Default::default()
            };

            if np_iface.bridge.as_ref().and_then(|b| b.vlan_filtering)
                == Some(true)
                && let Some(np_port_info) = port_np_iface.bridge_port.as_ref()
            {
                port_conf.vlan = np_port_info.vlans.as_ref().and_then(|v| {
                    parse_bridge_vlan_conf(
                        v.as_slice(),
                        self.bridge
                            .as_ref()
                            .and_then(|br_conf| br_conf.options.as_ref())
                            .and_then(|br_opts| br_opts.vlan_default_pvid),
                    )
                });
            }
            port_confs.push(port_conf);
        }

        if let Some(br_conf) = self.bridge.as_mut() {
            br_conf.ports = Some(port_confs);
        }
    }

    pub(crate) fn apply_linux_bridge_port_configs(
        &self,
        cur_br_iface: Option<&Self>,
    ) -> Vec<nispor::IfaceConf> {
        let mut ret: Vec<nispor::IfaceConf> = Vec::new();
        if let Some(ports_conf) =
            self.bridge.as_ref().and_then(|b| b.ports.as_ref())
        {
            for port_conf in ports_conf.iter().filter(|p| !p.is_name_only()) {
                let np_br_port_conf = gen_bridge_port_conf(
                    port_conf,
                    cur_br_iface.and_then(|cur_br_iface| {
                        cur_br_iface.get_port_conf(port_conf.name.as_str())
                    }),
                );
                let mut port_np_iface = nispor::IfaceConf::default();
                port_np_iface.name = port_conf.name.to_string();
                port_np_iface.bridge_port = Some(np_br_port_conf);
                ret.push(port_np_iface);
            }
        }
        ret
    }
}

fn gen_bridge_port_conf(
    port_conf: &LinuxBridgePortConfig,
    cur_port_conf: Option<&LinuxBridgePortConfig>,
) -> nispor::BridgePortConf {
    let mut ret = nispor::BridgePortConf::default();
    ret.hairpin_mode = port_conf.stp_hairpin_mode;
    ret.stp_priority = port_conf.stp_priority;
    ret.stp_path_cost = port_conf.stp_path_cost;
    ret.vlans = port_conf.vlan.as_ref().map(|vlan| {
        gen_bridge_vlan_conf(
            vlan,
            cur_port_conf.as_ref().and_then(|p| p.vlan.as_ref()),
        )
    });
    ret
}

const DEFAULT_USER_HZ: u32 = 100;

// The kernel is multiplying these bridge properties by USER_HZ, we should
// divide into seconds:
//   * forward_delay
//   * ageing_time
//   * hello_time
//   * max_age
//
// When kernel CONFIG_HZ is not multiple times of USER_HZ, we will noticed 1
// different between desired value and active value. For example, Archlinux has
// CONFIG_HZ=300 which causing ageing_time requested 600 got 599 in return.
// To fix the verification error, we add 1 for this trivial difference.
fn devide_by_user_hz(v: u32) -> u32 {
    (v + 1) / get_user_hz()
}

fn get_user_hz() -> u32 {
    if let Ok(Some(user_hz)) =
        nix::unistd::sysconf(nix::unistd::SysconfVar::CLK_TCK)
        && user_hz > 0
    {
        user_hz as u32
    } else {
        DEFAULT_USER_HZ
    }
}

fn get_stp_options(np_bridge: &nispor::BridgeInfo) -> LinuxBridgeStpOptions {
    LinuxBridgeStpOptions {
        enabled: Some(
            [
                Some(nispor::BridgeStpState::KernelStp),
                Some(nispor::BridgeStpState::UserStp),
            ]
            .contains(&np_bridge.stp_state),
        ),
        forward_delay: np_bridge.forward_delay.map(devide_by_user_hz).map(
            |v| {
                u8::try_from(v)
                    .unwrap_or(LinuxBridgeStpOptions::FORWARD_DELAY_MAX)
            },
        ),
        max_age: np_bridge.max_age.map(devide_by_user_hz).map(|v| {
            u8::try_from(v).unwrap_or(LinuxBridgeStpOptions::MAX_AGE_MAX)
        }),

        hello_time: np_bridge.hello_time.map(devide_by_user_hz).map(|v| {
            u8::try_from(v).unwrap_or(LinuxBridgeStpOptions::HELLO_TIME_MAX)
        }),
        priority: np_bridge.priority,
    }
}

fn parse_eth_mac(mac_str: &str) -> Result<[u8; 6], NipartError> {
    let mut mac_vec: Vec<u8> = Vec::new();
    for byte in mac_str.split(':') {
        mac_vec.push(u8::from_str_radix(byte, 16).map_err(|_| {
            NipartError::new(
                ErrorKind::InvalidArgument,
                format!(
                    "Invalid MAC address {mac_str}, expecting format like: \
                     02:69:4c:41:42:cd"
                ),
            )
        })?);
    }
    mac_vec.try_into().map_err(|_| {
        NipartError::new(
            ErrorKind::InvalidArgument,
            format!(
                "Invalid MAC address {mac_str}, expecting format like: \
                 02:69:4c:41:42:cd"
            ),
        )
    })
}
