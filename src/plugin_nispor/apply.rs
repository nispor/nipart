// SPDX-License-Identifier: Apache-2.0

use nipart::{
    ErrorKind, InterfaceType, MergedInterface, MergedInterfaces,
    MergedNetworkState, NipartDhcpLease, NipartError, NipartInterface,
};

pub(crate) async fn nispor_apply(
    merged_state: MergedNetworkState,
) -> Result<(), NipartError> {
    delete_ifaces(&merged_state.ifaces).await?;
    Ok(())
}

fn nipart_iface_type_to_np(
    nms_iface_type: &InterfaceType,
) -> nispor::IfaceType {
    match nms_iface_type {
        InterfaceType::LinuxBridge => nispor::IfaceType::Bridge,
        InterfaceType::Bond => nispor::IfaceType::Bond,
        InterfaceType::Ethernet => nispor::IfaceType::Ethernet,
        InterfaceType::Veth => nispor::IfaceType::Veth,
        InterfaceType::Vlan => nispor::IfaceType::Vlan,
        _ => nispor::IfaceType::Unknown,
    }
}

fn nipart_iface_to_np(
    merged_iface: &MergedInterface,
) -> Result<nispor::IfaceConf, NipartError> {
    let mut np_iface = nispor::IfaceConf::default();

    let for_apply = match merged_iface.for_apply.as_ref() {
        Some(i) => i,
        None => {
            return Err(NipartError::new(
                ErrorKind::Bug,
                format!(
                    "nipart_iface_to_np() got MergedInterface with \
                    for_apply set to None: {merged_iface:?}"
                ),
            ));
        }
    };

    let np_iface_type = nipart_iface_type_to_np(for_apply.iface_type());

    np_iface.name = for_apply.name().to_string();
    np_iface.iface_type = Some(np_iface_type);
    if for_apply.is_absent() {
        np_iface.state = nispor::IfaceState::Absent;
        return Ok(np_iface);
    }

    np_iface.state = nispor::IfaceState::Up;

    Ok(np_iface)
}

async fn delete_ifaces(
    merged_ifaces: &MergedInterfaces,
) -> Result<(), NipartError> {
    let mut np_ifaces: Vec<nispor::IfaceConf> = Vec::new();
    for iface in merged_ifaces
        .kernel_ifaces
        .values()
        .filter(|i| i.merged.is_absent())
    {
        log::debug!("Deleting interface {}", iface.merged.name());
        np_ifaces.push(nipart_iface_to_np(iface)?);
    }

    let mut net_conf = nispor::NetConf::default();
    net_conf.ifaces = Some(np_ifaces);

    if let Err(e) = net_conf.apply_async().await {
        Err(NipartError::new(
            ErrorKind::PluginFailure,
            format!("Unknown error from nipsor plugin: {}, {}", e.kind, e.msg),
        ))
    } else {
        Ok(())
    }
}

pub(crate) async fn nispor_apply_dhcp_lease(
    lease: NipartDhcpLease,
) -> Result<(), NipartError> {
    match lease {
        NipartDhcpLease::V4(lease) => {
            let mut net_conf = nispor::NetConf::default();
            let mut np_iface = nispor::IfaceConf::default();
            np_iface.name = lease.iface.to_string();
            let mut ip_conf = nispor::IpConf::default();
            let mut ip_addr = nispor::IpAddrConf::default();
            ip_addr.address = lease.ip.to_string();
            ip_addr.prefix_len = lease.prefix_length;
            ip_addr.valid_lft = format!("{}sec", lease.lease_time);
            ip_addr.preferred_lft = format!("{}sec", lease.lease_time);
            // BUG: We should preserve existing IP address
            ip_conf.addresses.push(ip_addr);
            np_iface.ipv4 = Some(ip_conf);
            np_iface.state = nispor::IfaceState::Up;
            net_conf.ifaces = Some(vec![np_iface]);

            log::debug!("Plugin nispor apply {net_conf:?}");

            if let Err(e) = net_conf.apply_async().await {
                Err(NipartError::new(
                    ErrorKind::PluginFailure,
                    format!(
                        "Unknown error nispor apply_async: {}, {}",
                        e.kind, e.msg
                    ),
                ))
            } else {
                Ok(())
            }
        }
        NipartDhcpLease::V6(_) => {
            todo!()
        }
    }
}
