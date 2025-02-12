// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use nipart::{
    Interface, InterfaceType, Interfaces, NetworkState, NipartError,
    NipartInterface, UnknownInterface,
};

use crate::{
    base_iface::np_iface_to_base_iface, error::np_error_to_nipart,
    ethernet::np_ethernet_to_nipart,
};

pub(crate) async fn nispor_retrieve(
    running_config_only: bool,
) -> Result<NetworkState, NipartError> {
    let mut net_state = NetworkState::default();
    let mut filter = nispor::NetStateFilter::default();
    // Do not query routes in order to prevent BGP routes consuming too much CPU
    // time, we let `get_routes()` do the query by itself.
    filter.route = None;
    let np_state = nispor::NetState::retrieve_with_filter_async(&filter)
        .await
        .map_err(np_error_to_nipart)?;

    for (_, np_iface) in np_state.ifaces.iter() {
        // The `ovs-system` is reserved for OVS kernel datapath
        if np_iface.name == "ovs-system" {
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

        let base_iface = np_iface_to_base_iface(np_iface, running_config_only);
        let iface = match &base_iface.iface_type {
            InterfaceType::Ethernet => Interface::Ethernet(Box::new(
                np_ethernet_to_nipart(np_iface, base_iface),
            )),
            _ => {
                log::debug!(
                    "Got unsupported interface {} type {:?}",
                    np_iface.name,
                    np_iface.iface_type
                );
                Interface::Unknown({
                    let mut iface = UnknownInterface::default();
                    iface.base = base_iface;
                    Box::new(iface)
                })
            }
        };
        net_state.ifaces.push(iface);
    }
    set_controller_type(&mut net_state.ifaces);

    Ok(net_state)
}

fn set_controller_type(ifaces: &mut Interfaces) {
    let mut ctrl_to_type: HashMap<String, InterfaceType> = HashMap::new();
    for iface in ifaces.to_vec() {
        if iface.is_controller() {
            ctrl_to_type
                .insert(iface.name().to_string(), iface.iface_type().clone());
        }
    }
    for iface in ifaces.kernel_ifaces.values_mut() {
        if let Some(ctrl) = iface.base_iface().controller.as_ref() {
            if let Some(ctrl_type) = ctrl_to_type.get(ctrl) {
                iface.base_iface_mut().controller_type =
                    Some(ctrl_type.clone());
            }
        }
    }
}
