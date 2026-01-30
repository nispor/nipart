// SPDX-License-Identifier: Apache-2.0

use super::{
    iface::{apply_iface_link_changes, nmstate_iface_type_to_nispor},
    ip::apply_iface_ip_changes,
    wifi::NipartWpaConn,
};
use crate::{
    ErrorKind, Interface, InterfaceType, MergedInterface, MergedInterfaces,
    NipartError, NipartstateInterface,
};

pub(crate) async fn apply_ifaces(
    merged_ifaces: &MergedInterfaces,
) -> Result<(), NipartError> {
    delete_ifaces_before_apply(merged_ifaces).await?;

    apply_ifaces_link_changes(merged_ifaces).await?;

    apply_ifaces_ip_changes(merged_ifaces).await?;

    Ok(())
}

async fn delete_ifaces_before_apply(
    merged_ifaces: &MergedInterfaces,
) -> Result<(), NipartError> {
    let mut np_ifaces: Vec<nispor::IfaceConf> = Vec::new();
    for merged_iface in merged_ifaces.kernel_ifaces.values().filter(|m| {
        m.merged.is_up() && m.for_apply.is_some() && m.current.is_some()
    }) {
        if let Some(for_apply) = merged_iface.for_apply.as_ref()
            && let Some(current) = merged_iface.current.as_ref()
            && for_apply.need_delete_before_change(current)
        {
            log::debug!(
                "Need to delete interface {}/{} before making changes",
                for_apply.name(),
                for_apply.iface_type()
            );
            let mut np_iface = nispor::IfaceConf::default();
            np_iface.name = for_apply.name().to_string();
            np_iface.iface_type =
                Some(nmstate_iface_type_to_nispor(for_apply.iface_type()));
            np_iface.state = nispor::IfaceState::Absent;
            np_ifaces.push(np_iface)
        }
    }
    if !np_ifaces.is_empty() {
        let mut net_conf = nispor::NetConf::default();
        net_conf.ifaces = Some(np_ifaces);

        log::debug!(
            "Pending nispor changes {}",
            serde_json::to_string(&net_conf).unwrap_or_default()
        );

        if let Err(e) = net_conf.apply_async().await {
            return Err(NipartError::new(
                ErrorKind::Bug,
                format!("Failed to delete interfaces: {e}"),
            ));
        }
    }
    Ok(())
}

/// Apply link level changes (e.g. state up/down, attach to controller)
async fn apply_ifaces_link_changes(
    merged_ifaces: &MergedInterfaces,
) -> Result<(), NipartError> {
    let mut np_ifaces: Vec<nispor::IfaceConf> = Vec::new();

    let mut sorted_changed_mergd_ifaces: Vec<&MergedInterface> = merged_ifaces
        .kernel_ifaces
        .values()
        .chain(merged_ifaces.user_ifaces.values())
        .filter(|i| i.for_apply.is_some())
        .collect();
    sorted_changed_mergd_ifaces.sort_unstable_by_key(|i| {
        i.for_apply
            .as_ref()
            .map(|i| i.base_iface().up_priority)
            .unwrap_or(u32::MAX)
    });

    let mut changed_wifi_ifaces: Vec<&Interface> = Vec::new();

    for merged_iface in sorted_changed_mergd_ifaces.as_slice() {
        let apply_iface = if let Some(i) = merged_iface.for_apply.as_ref() {
            i
        } else {
            continue;
        };

        if apply_iface.iface_type() == &InterfaceType::WifiCfg
            || apply_iface.iface_type() == &InterfaceType::WifiPhy
        {
            changed_wifi_ifaces.push(apply_iface);
        }

        if !apply_iface.iface_type().is_userspace() {
            for np_iface in apply_iface_link_changes(
                apply_iface,
                merged_iface.current.as_ref(),
                merged_ifaces,
            )? {
                np_ifaces.push(np_iface);
            }
        }
    }

    // When port config changed in controller, the `apply_ifaces` above
    // will not have port. And we cannot touch ports when processing controller
    // because port might be virtual interface which is about to created.
    // Hence we handle port config in this separate loop.
    for merged_iface in
        sorted_changed_mergd_ifaces
            .as_slice()
            .iter()
            .filter(|merged_iface| {
                merged_iface.merged.is_controller()
                    && merged_iface.is_desired()
                    && merged_iface.merged.is_up()
            })
    {
        let apply_iface = if let Some(i) = merged_iface.for_apply.as_ref() {
            i
        } else {
            continue;
        };
        match apply_iface {
            Interface::Bond(bond_iface) => {
                np_ifaces
                    .extend(bond_iface.apply_bond_port_configs().into_iter());
            }
            Interface::LinuxBridge(br_iface) => {
                np_ifaces.extend(
                    br_iface
                        .apply_linux_bridge_port_configs(
                            if let Some(Interface::LinuxBridge(cur_br_iface)) =
                                merged_iface.current.as_ref()
                            {
                                Some(cur_br_iface)
                            } else {
                                None
                            },
                        )
                        .into_iter(),
                );
            }
            Interface::OvsBridge(_) => {
                // Place holder
            }
            _ => (),
        }
    }

    if !changed_wifi_ifaces.is_empty() {
        NipartWpaConn::apply(changed_wifi_ifaces.as_slice(), merged_ifaces)
            .await?;
    }

    if !np_ifaces.is_empty() {
        let mut net_conf = nispor::NetConf::default();
        net_conf.ifaces = Some(np_ifaces);

        log::trace!(
            "Pending nispor changes {}",
            serde_json::to_string(&net_conf).unwrap_or_default()
        );
        if let Err(e) = net_conf.apply_async().await {
            return Err(NipartError::new(
                ErrorKind::Bug,
                format!("Failed to change link layer: {e}"),
            ));
        }
    }

    Ok(())
}

async fn apply_ifaces_ip_changes(
    merged_ifaces: &MergedInterfaces,
) -> Result<(), NipartError> {
    let mut np_ifaces: Vec<nispor::IfaceConf> = Vec::new();

    for merged_iface in merged_ifaces
        .kernel_ifaces
        .values()
        .filter(|i| i.for_apply.is_some())
    {
        // It is safe to unwrap here as it is checked by filter()
        let apply_iface = merged_iface.for_apply.as_ref().unwrap();

        if let Some(np_iface) = apply_iface_ip_changes(
            apply_iface.base_iface(),
            merged_iface.current.as_ref().map(|c| c.base_iface()),
        )? {
            np_ifaces.push(np_iface);
        }
    }
    if !np_ifaces.is_empty() {
        let mut net_conf = nispor::NetConf::default();
        net_conf.ifaces = Some(np_ifaces);

        log::debug!(
            "Pending nispor changes {}",
            serde_json::to_string(&net_conf).unwrap_or_default()
        );

        if let Err(e) = net_conf.apply_async().await {
            return Err(NipartError::new(
                ErrorKind::Bug,
                format!("Failed to change IP: {e}"),
            ));
        }
    }

    Ok(())
}
