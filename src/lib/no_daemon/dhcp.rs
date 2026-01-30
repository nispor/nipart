// SPDX-License-Identifier: Apache-2.0

use futures_util::{StreamExt, stream::FuturesUnordered};
use mozim::{DhcpV4Client, DhcpV4Config, DhcpV4Lease, DhcpV4State};

use super::{ip::apply_iface_ip_changes, route::apply_routes};
use crate::{
    ErrorKind, InterfaceIpAddr, InterfaceIpv4, InterfaceType, MergedInterfaces,
    MergedRoutes, NipartError, NipartNoDaemon, NipartstateInterface,
    RouteEntry, Routes,
};

const DEFAULT_ROUTE_TABLE_ID: u32 = 254;

impl NipartNoDaemon {
    pub(crate) async fn run_dhcp_once(
        merged_ifaces: &MergedInterfaces,
    ) -> Result<(), NipartError> {
        let mut get_lease_futures = FuturesUnordered::new();

        for iface in merged_ifaces
            .kernel_ifaces
            .values()
            .filter_map(|i| i.for_apply.as_ref())
            .filter(|i| {
                i.base_iface().ipv4.as_ref().and_then(|ip| ip.dhcp)
                    == Some(true)
            })
        {
            let get_lease_future = get_lease(iface.name(), iface.iface_type());
            get_lease_futures.push(get_lease_future);
        }

        while let Some(result) = get_lease_futures.next().await {
            // Should fail the whole apply action for any errors of DHCP.
            let (iface_name, lease) = result?;
            apply_lease(merged_ifaces, iface_name, lease).await?;
        }
        Ok(())
    }
}

async fn get_lease<'a>(
    iface_name: &'a str,
    iface_type: &InterfaceType,
) -> Result<(&'a str, DhcpV4Lease), NipartError> {
    let dhcp_config = DhcpV4Config::new(iface_name);
    log::debug!(
        "Waiting link carrier up for interface {}/{} before start DHCP",
        iface_name,
        iface_type
    );
    NipartNoDaemon::wait_link_carrier_up(iface_name).await?;
    log::debug!(
        "Interface {}/{} link carrier is up, starting DHCP process",
        iface_name,
        iface_type
    );
    let mut dhcp_client =
        DhcpV4Client::init(dhcp_config, None).await.map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!(
                    "Failed to start DHCPv4 client on iface {}/{}: {e}",
                    iface_name, iface_type,
                ),
            )
        })?;
    loop {
        let state = dhcp_client.run().await.map_err(|e| {
            NipartError::new(
                ErrorKind::InvalidArgument,
                format!("DHCPv4 failed: {e}"),
            )
        })?;
        if let DhcpV4State::Done(lease) = state {
            log::info!(
                "DHCPv4 on interface acquired lease: {}/{}",
                lease.yiaddr,
                lease.prefix_length()
            );
            return Ok((iface_name, *lease));
        } else {
            log::info!(
                "DHCPv4 on interface {iface_name}/{iface_type} reach {state} \
                 state",
            );
        }
    }
}

async fn apply_lease(
    merged_ifaces: &MergedInterfaces,
    iface_name: &str,
    lease: DhcpV4Lease,
) -> Result<(), NipartError> {
    let Some(merged_iface) = merged_ifaces.kernel_ifaces.get(iface_name) else {
        return Err(NipartError::new(
            ErrorKind::Bug,
            format!(
                "apply_lease(): Failed to find merged interface for interface \
                 {iface_name}"
            ),
        ));
    };
    log::debug!(
        "Applying DHCPv4 lease {}/{} to interface {}/{}",
        lease.yiaddr,
        lease.prefix_length(),
        iface_name,
        merged_iface.merged.iface_type(),
    );

    let mut ip_addr =
        InterfaceIpAddr::new(lease.yiaddr.into(), lease.prefix_length());
    ip_addr.preferred_life_time = Some(format!("{}sec", lease.lease_time_sec));
    ip_addr.valid_life_time = Some(format!("{}sec", lease.lease_time_sec));

    let mut ipv4_conf = InterfaceIpv4::new();
    ipv4_conf.enabled = Some(true);
    ipv4_conf.dhcp = Some(true);
    ipv4_conf.addresses = Some(vec![ip_addr]);

    let mut apply_base_iface =
        merged_iface.merged.base_iface().clone_name_type_only();

    apply_base_iface.ipv4 = Some(ipv4_conf);
    if let Some(mtu) = lease.mtu {
        apply_base_iface.mtu = Some(mtu.into());
    }

    if let Some(np_iface) = apply_iface_ip_changes(
        &apply_base_iface,
        merged_iface.current.as_ref().map(|c| c.base_iface()),
    )? {
        let mut net_conf = nispor::NetConf::default();
        net_conf.ifaces = Some(vec![np_iface]);
        if let Err(e) = net_conf.apply_async().await {
            return Err(NipartError::new(
                ErrorKind::Bug,
                format!("Failed to apply DHCP IP address: {e}"),
            ));
        }
    }

    let mut conf_routes: Vec<RouteEntry> = Vec::new();
    // TODO: Handle multiple addresses of router
    if let Some(gateways) = lease.gateways.as_ref() {
        for (index, gateway) in gateways.iter().enumerate() {
            let route = RouteEntry {
                destination: Some("0.0.0.0/0".to_string()),
                next_hop_iface: Some(iface_name.to_string()),
                next_hop_addr: Some(gateway.to_string()),
                table_id: Some(DEFAULT_ROUTE_TABLE_ID),
                // TODO: Be consistent on metric?
                // TODO: Priority ethernet over wifi/VPN/etc ?
                metric: merged_iface
                    .current
                    .as_ref()
                    .and_then(|c| c.base_iface().iface_index)
                    .map(|iface_index| {
                        100i64 * iface_index as i64 + index as i64
                    }),
                ..Default::default()
            };
            conf_routes.push(route);
        }
    }

    let des_routes = Routes {
        config: Some(conf_routes),
        ..Default::default()
    };

    let merged_routes =
        MergedRoutes::new(des_routes, Default::default(), merged_ifaces)?;

    apply_routes(&merged_routes).await?;

    Ok(())
}
