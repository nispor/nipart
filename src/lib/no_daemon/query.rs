// SPDX-License-Identifier: Apache-2.0

use super::{
    base_iface::np_iface_to_base_iface, ovs::NipartOvsDb, route::get_routes,
    wifi::NipartWpaConn,
};
use crate::{
    BondInterface, DummyInterface, ErrorKind, EthernetInterface, Interface,
    InterfaceType, LinuxBridgeInterface, LoopbackInterface, NetworkState,
    NipartError, NipartNoDaemon, NipartstateInterface, NipartstateQueryOption,
    UnknownInterface, VlanInterface, WifiPhyInterface,
};

impl NipartNoDaemon {
    pub async fn query_network_state(
        option: NipartstateQueryOption,
    ) -> Result<NetworkState, NipartError> {
        if option.version != 1 {
            return Err(NipartError::new(
                ErrorKind::InvalidSchemaVersion,
                format!(
                    "Only support version 1, but desired {}",
                    option.version
                ),
            ));
        }
        // TODO: check other property in NipartstateQueryOption

        let mut net_state = NetworkState::default();
        let mut filter = nispor::NetStateFilter::default();
        // Do not query routes in order to prevent BGP routes consuming too much
        // CPU time, we let `get_routes()` do the query by itself.
        filter.route = None;
        let np_state =
            nispor::NetState::retrieve_with_filter_async(&filter).await?;

        let mut has_wifi_nic = false;
        let mut has_ovs_datapath_nic = false;

        for (_, np_iface) in np_state.ifaces.iter() {
            // The `ovs-system` is reserved for OVS kernel datapath
            if np_iface.name == "ovs-system" {
                has_ovs_datapath_nic = true;
                continue;
            }
            // The `ovs-netdev` is reserved for OVS netdev datapath
            if np_iface.name == "ovs-netdev" {
                continue;
            }
            // The vti interface is reserved for Ipsec
            if np_iface.iface_type == nispor::IfaceType::Other("Vti".into()) {
                continue;
            }

            let base_iface = np_iface_to_base_iface(np_iface);
            let iface = match &base_iface.iface_type {
                InterfaceType::Ethernet | InterfaceType::Veth => {
                    Interface::Ethernet(Box::new(
                        EthernetInterface::new_from_nispor(
                            base_iface, np_iface,
                        ),
                    ))
                }
                InterfaceType::Loopback => Interface::Loopback(Box::new(
                    LoopbackInterface::new(base_iface),
                )),
                InterfaceType::WifiPhy => {
                    has_wifi_nic = true;
                    Interface::WifiPhy(Box::new(
                        WifiPhyInterface::new_from_nispor(base_iface, np_iface),
                    ))
                }
                InterfaceType::Dummy => {
                    Interface::Dummy(Box::new(DummyInterface {
                        base: base_iface,
                    }))
                }
                InterfaceType::Vlan => Interface::Vlan(Box::new(
                    VlanInterface::new_from_nispor(base_iface, np_iface),
                )),
                InterfaceType::Bond => {
                    let mut bond_iface =
                        BondInterface::new_from_nispor(base_iface, np_iface);
                    let mut port_np_ifaces = Vec::new();
                    for port_name in bond_iface.ports().unwrap_or_default() {
                        if let Some(p) = np_state.ifaces.get(port_name) {
                            port_np_ifaces.push(p);
                        }
                    }
                    bond_iface.append_bond_port_config(port_np_ifaces);

                    Interface::Bond(Box::new(bond_iface))
                }
                InterfaceType::LinuxBridge => {
                    let mut br_iface = LinuxBridgeInterface::new_from_nispor(
                        base_iface, np_iface,
                    );
                    let mut port_np_ifaces = Vec::new();
                    for port_name in br_iface.ports().unwrap_or_default() {
                        if let Some(p) = np_state.ifaces.get(port_name) {
                            port_np_ifaces.push(p);
                        }
                    }
                    br_iface.append_br_port_config(np_iface, port_np_ifaces);
                    Interface::LinuxBridge(Box::new(br_iface))
                }
                _ => {
                    log::trace!(
                        "Got unsupported interface {} type {:?}",
                        np_iface.name,
                        np_iface.iface_type
                    );
                    Interface::Unknown({
                        Box::new(UnknownInterface::new(base_iface))
                    })
                }
            };
            net_state.ifaces.push(iface);
        }

        if has_wifi_nic {
            NipartWpaConn::fill_wifi_cfg(&mut net_state.ifaces).await?;
        }

        if has_ovs_datapath_nic {
            NipartOvsDb::fill_ovs_cfg(&mut net_state).await?;
        }

        net_state.routes = get_routes(&net_state.ifaces).await;

        net_state
            .routes
            .mark_route_as_ignored_ifaces(&net_state.ifaces);
        Ok(net_state)
    }
}
