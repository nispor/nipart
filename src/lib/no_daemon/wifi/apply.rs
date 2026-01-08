// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use super::{
    NipartWpaConn, bss::WpaSupBss, dbus::NipartWpaSupDbus, network::WpaSupNetwork,
    scan::bss_active_scan,
};
use crate::{
    ErrorKind, Interface, InterfaceType, MergedInterfaces, NipartError,
    NipartstateInterface, WifiConfig,
};

impl NipartWpaConn {
    pub(crate) async fn apply(
        ifaces: &[&Interface],
        merged_ifaces: &MergedInterfaces,
    ) -> Result<(), NipartError> {
        let dbus = NipartWpaSupDbus::new().await?;

        let mut ssids_to_delete: HashSet<&str> = HashSet::new();
        let mut iface_names_to_delete: HashSet<&str> = HashSet::new();
        let mut wifi_cfg_to_add: Vec<(&str, &WifiConfig)> = Vec::new();
        for iface in ifaces {
            let wifi_cfg = match iface {
                Interface::WifiCfg(iface) => iface.wifi.as_ref(),
                Interface::WifiPhy(iface) => iface.wifi.as_ref(),
                _ => {
                    continue;
                }
            };
            if iface.is_absent() || iface.is_down() {
                if iface.iface_type() == &InterfaceType::WifiPhy {
                    iface_names_to_delete.insert(iface.name());
                } else {
                    let ssid = if let Some(s) =
                        wifi_cfg.as_ref().map(|w| w.ssid.as_str())
                    {
                        s
                    } else {
                        iface.name()
                    };
                    ssids_to_delete.insert(ssid);
                }
            } else if iface.is_up() {
                let Some(wifi_cfg) = wifi_cfg else {
                    continue;
                };
                log::trace!("Applying {wifi_cfg}");
                if iface.iface_type() == &InterfaceType::WifiPhy {
                    wifi_cfg_to_add.push((iface.name(), wifi_cfg));
                } else if let Some(iface_name) = wifi_cfg.base_iface.as_ref() {
                    wifi_cfg_to_add.push((iface_name, wifi_cfg));
                } else {
                    // Bind to any WIFI NICs
                    for merged_iface in
                        merged_ifaces.kernel_ifaces.values().filter(|i| {
                            i.for_apply.as_ref().map(|i| {
                                i.iface_type() == &InterfaceType::WifiPhy
                                    && i.is_up()
                            }) == Some(true)
                        })
                    {
                        wifi_cfg_to_add
                            .push((merged_iface.merged.name(), wifi_cfg));
                    }
                }
            } else {
                return Err(NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "NipartWpaConn::apply(): Got invalid interface state: \
                         {iface}"
                    ),
                ));
            }
        }

        del_interfaces(&dbus, &iface_names_to_delete).await?;
        del_networks(&dbus, &ssids_to_delete).await?;
        add_networks(&dbus, &wifi_cfg_to_add).await?;

        Ok(())
    }
}

async fn add_networks(
    dbus: &NipartWpaSupDbus<'_>,
    wifi_cfg_to_add: &[(&str, &WifiConfig)],
) -> Result<(), NipartError> {
    let ifaces_to_scan: Vec<&str> = wifi_cfg_to_add
        .iter()
        .map(|(iface_name, _)| *iface_name)
        .collect();

    if ifaces_to_scan.is_empty() {
        return Ok(());
    }
    log::trace!("Adding WIFI network {:?}", wifi_cfg_to_add);

    let existing_bsses = bss_active_scan(dbus, &ifaces_to_scan).await?;

    for (iface_name, wifi_cfg) in wifi_cfg_to_add {
        add_wifi_cfg(
            iface_name,
            wifi_cfg,
            dbus,
            existing_bsses
                .get(&(iface_name.to_string(), wifi_cfg.ssid.to_string())),
        )
        .await?;
    }
    Ok(())
}

async fn del_interfaces(
    dbus: &NipartWpaSupDbus<'_>,
    iface_names: &HashSet<&str>,
) -> Result<(), NipartError> {
    let ifaces = dbus.get_ifaces().await?;
    let existing_ifaces: Vec<&str> =
        ifaces.iter().map(|i| i.iface_name.as_str()).collect();

    for iface_name in iface_names {
        if existing_ifaces.contains(iface_name) {
            dbus.del_iface(iface_name).await?;
        }
    }
    Ok(())
}

async fn del_networks(
    dbus: &NipartWpaSupDbus<'_>,
    ssids: &HashSet<&str>,
) -> Result<(), NipartError> {
    let wpa_ifaces = dbus.get_ifaces().await?;
    for wpa_iface in wpa_ifaces {
        let networks = dbus.get_networks(wpa_iface.obj_path.as_str()).await?;
        for network in networks {
            if ssids.contains(network.ssid.as_str()) {
                dbus.del_network(
                    wpa_iface.obj_path.as_str(),
                    network.obj_path.as_str(),
                )
                .await?;
            }
        }
    }
    Ok(())
}

async fn add_wifi_cfg(
    iface_name: &str,
    wifi_cfg: &WifiConfig,
    dbus: &NipartWpaSupDbus<'_>,
    bss: Option<&WpaSupBss>,
) -> Result<(), NipartError> {
    let ssid = wifi_cfg.ssid.as_str();
    let iface_obj_path = match dbus.get_iface_obj_path(iface_name).await? {
        None => dbus.add_iface(iface_name).await?,
        Some(iface_obj_path) => {
            let networks = dbus.get_networks(&iface_obj_path).await?;
            for network in networks {
                if network.ssid == ssid {
                    log::debug!(
                        "Deactivating existing WIFI network {ssid} on \
                         interface {}: {}",
                        iface_name,
                        network.obj_path.as_str(),
                    );
                    dbus.del_network(
                        iface_obj_path.as_str(),
                        network.obj_path.as_str(),
                    )
                    .await?;
                }
            }
            iface_obj_path
        }
    };

    let mut wpa_network = WpaSupNetwork {
        ssid: ssid.to_string(),
        psk: wifi_cfg.password.clone(),
        bssid: wifi_cfg.bssid.clone(),
        ..Default::default()
    };
    if let Some(bss) = bss {
        if bss.is_wpa3_psk() {
            wpa_network.change_to_wpa3_psk();
        }
    }
    log::debug!("Adding WIFI network {ssid} to interface {}", iface_name);
    let network_obj_path = dbus
        .add_network(iface_obj_path.as_str(), &wpa_network)
        .await?;
    dbus.enable_network(network_obj_path.as_str()).await?;

    Ok(())
}
