// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use futures_util::StreamExt;
use zvariant::{ObjectPath, OwnedObjectPath};

use super::{
    bss::WpaSupBss, interface::WpaSupInterface, network::WpaSupNetwork,
};
use crate::{ErrorKind, NipartError};

const WPA_SUP_DBUS_IFACE_ROOT: &str = "fi.w1.wpa_supplicant1";
const WPA_SUP_DBUS_IFACE_IFACE: &str = "fi.w1.wpa_supplicant1.Interface";
const WPA_SUP_DBUS_IFACE_NETWORK: &str = "fi.w1.wpa_supplicant1.Network";
const WPA_SUP_DBUS_IFACE_BSS: &str = "fi.w1.wpa_supplicant1.BSS";
const DBUS_IFACE_PROPS: &str = "org.freedesktop.DBus.Properties";

// These proxy() macros only generate private struct, hence it should be
// sit with its consumer.
#[zbus::proxy(
    interface = "fi.w1.wpa_supplicant1",
    default_service = "fi.w1.wpa_supplicant1",
    default_path = "/fi/w1/wpa_supplicant1"
)]
trait WpaSupplicant {
    #[zbus(property)]
    fn interfaces(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    #[zbus(property)]
    fn capabilities(&self) -> zbus::Result<Vec<String>>;

    fn create_interface(
        &self,
        iface: HashMap<&str, zvariant::Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;

    fn remove_interface(&self, obj_path: OwnedObjectPath) -> zbus::Result<()>;

    fn get_interface(&self, iface_name: &str) -> zbus::Result<OwnedObjectPath>;
}

pub(crate) struct NipartWpaSupDbus<'a> {
    pub(crate) connection: zbus::Connection,
    proxy: WpaSupplicantProxy<'a>,
}

impl NipartWpaSupDbus<'_> {
    pub(crate) async fn new() -> Result<Self, NipartError> {
        let connection = zbus::Connection::system().await.map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to create system DBUS connection: {e}"),
            )
        })?;
        let proxy =
            WpaSupplicantProxy::new(&connection).await.map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Failed to create DBUS proxy to wpa_supplicant: {e}"
                    ),
                )
            })?;
        // Test connection
        proxy.capabilities().await.map_err(|e| {
            if let zbus::Error::MethodError(error_name, ..) = &e
                && error_name.as_str()
                    == "org.freedesktop.DBus.Error.AccessDenied"
            {
                NipartError::new(
                    ErrorKind::PermissionDeny,
                    "Permission deny when connecting wpa_supplicant DBUS \
                     interface"
                        .to_string(),
                )
            } else {
                NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Failed to connect wpa_supplicant DBUS interface: {e}"
                    ),
                )
            }
        })?;

        Ok(Self { connection, proxy })
    }

    pub(crate) async fn get_iface_obj_paths(
        &self,
    ) -> Result<Vec<String>, NipartError> {
        Ok(self
            .proxy
            .interfaces()
            .await
            .map_err(map_zbus_err)?
            .into_iter()
            .map(obj_path_to_string)
            .collect())
    }

    pub(crate) async fn get_iface_obj_path(
        &self,
        iface_name: &str,
    ) -> Result<Option<String>, NipartError> {
        match self
            .proxy
            .get_interface(iface_name)
            .await
            .map(obj_path_to_string)
        {
            Ok(s) => Ok(Some(s)),
            Err(e) => {
                if let zbus::Error::MethodError(error_path, _, _) = &e
                    && error_path.as_str()
                        == "fi.w1.wpa_supplicant1.InterfaceUnknown"
                {
                    Ok(None)
                } else {
                    Err(map_zbus_err(e))
                }
            }
        }
    }

    pub(crate) async fn get_network_obj_paths(
        &self,
        iface_obj_path: &str,
    ) -> Result<Vec<String>, NipartError> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            WPA_SUP_DBUS_IFACE_ROOT,
            iface_obj_path,
            WPA_SUP_DBUS_IFACE_IFACE,
        )
        .await
        .map_err(map_zbus_err)?;
        Ok(proxy
            .get_property::<Vec<OwnedObjectPath>>("Networks")
            .await
            .map_err(map_zbus_err)?
            .into_iter()
            .map(obj_path_to_string)
            .collect())
    }

    pub(crate) async fn get_network(
        &self,
        network_obj_path: &str,
    ) -> Result<WpaSupNetwork, NipartError> {
        log::trace!("NipartWpaSupDbus::get_network(): {network_obj_path}");
        let obj_path = str_to_obj_path(network_obj_path)?;
        let proxy = zbus::Proxy::new(
            &self.connection,
            WPA_SUP_DBUS_IFACE_ROOT,
            obj_path.as_str(),
            WPA_SUP_DBUS_IFACE_NETWORK,
        )
        .await
        .map_err(map_zbus_err)?;
        let value = proxy
            .get_property::<zvariant::OwnedValue>("Properties")
            .await
            .map_err(map_zbus_err)?;

        WpaSupNetwork::from_value(value, obj_path)
    }

    pub(crate) async fn get_networks(
        &self,
        iface_obj_path: &str,
    ) -> Result<Vec<WpaSupNetwork>, NipartError> {
        log::trace!("NipartWpaSupDbus::get_networks(): {iface_obj_path}");
        let mut ret: Vec<WpaSupNetwork> = Vec::new();
        for network_obj_path in
            self.get_network_obj_paths(iface_obj_path).await?
        {
            ret.push(self.get_network(&network_obj_path).await?);
        }
        Ok(ret)
    }

    pub(crate) async fn get_ifaces(
        &self,
    ) -> Result<Vec<WpaSupInterface>, NipartError> {
        log::trace!("NipartWpaSupDbus::get_ifaces()");
        let mut ret: Vec<WpaSupInterface> = Vec::new();
        for iface_obj_path in self.get_iface_obj_paths().await? {
            match self.get_iface(&iface_obj_path).await {
                Ok(iface) => ret.push(iface),
                Err(e) => {
                    // Interface might just been deleted
                    log::trace!(
                        "Ignoring WPA interface {iface_obj_path} for error {e}"
                    );
                }
            }
        }
        Ok(ret)
    }

    pub(crate) async fn get_iface(
        &self,
        iface_obj_path: &str,
    ) -> Result<WpaSupInterface, NipartError> {
        log::trace!("NipartWpaSupDbus::get_iface(): {iface_obj_path}");
        let obj_path = str_to_obj_path(iface_obj_path)?;
        let proxy = zbus::Proxy::new(
            &self.connection,
            WPA_SUP_DBUS_IFACE_ROOT,
            obj_path.as_str(),
            DBUS_IFACE_PROPS,
        )
        .await
        .map_err(map_zbus_err)?;
        let value = proxy
            .call::<&str, &str, HashMap<String, zvariant::OwnedValue>>(
                "GetAll",
                &WPA_SUP_DBUS_IFACE_IFACE,
            )
            .await
            .map_err(map_zbus_err)?;

        WpaSupInterface::from_value(value, obj_path)
    }

    pub(crate) async fn add_iface(
        &self,
        iface_name: &str,
    ) -> Result<String, NipartError> {
        log::trace!("Enabled WPA interface {iface_name}");
        self.proxy
            .create_interface(
                WpaSupInterface::new(iface_name.to_string()).to_value(),
            )
            .await
            .map(obj_path_to_string)
            .map_err(map_zbus_err)
    }

    pub(crate) async fn del_iface(
        &self,
        iface_name: &str,
    ) -> Result<(), NipartError> {
        log::trace!("Deleting WPA interface {iface_name}");
        let iface_obj_path = self
            .proxy
            .get_interface(iface_name)
            .await
            .map_err(map_zbus_err)?;
        self.proxy
            .remove_interface(iface_obj_path)
            .await
            .map_err(map_zbus_err)?;
        log::trace!("Deleted WPA interface {iface_name}");

        Ok(())
    }

    pub(crate) async fn add_network(
        &self,
        iface_obj_path: &str,
        network: &WpaSupNetwork,
    ) -> Result<String, NipartError> {
        log::trace!("Adding WPA network {iface_obj_path} {}", network.ssid);
        let proxy = zbus::Proxy::new(
            &self.connection,
            WPA_SUP_DBUS_IFACE_ROOT,
            iface_obj_path,
            WPA_SUP_DBUS_IFACE_IFACE,
        )
        .await
        .map_err(map_zbus_err)?;
        proxy
            .call::<&str, HashMap<&str, zvariant::Value<'_>>, OwnedObjectPath>(
                "AddNetwork",
                &network.to_value(),
            )
            .await
            .map(obj_path_to_string)
            .map_err(map_zbus_err)
    }

    pub(crate) async fn enable_network(
        &self,
        network_obj_path: &str,
    ) -> Result<(), NipartError> {
        log::trace!("Enable WIFI network {network_obj_path}");
        let proxy = zbus::Proxy::new(
            &self.connection,
            WPA_SUP_DBUS_IFACE_ROOT,
            network_obj_path,
            WPA_SUP_DBUS_IFACE_NETWORK,
        )
        .await
        .map_err(map_zbus_err)?;
        proxy
            .set_property::<bool>("Enabled", true)
            .await
            .map_err(map_zbus_fdo_err)
    }

    pub(crate) async fn del_network(
        &self,
        iface_obj_path: &str,
        network_obj_path: &str,
    ) -> Result<(), NipartError> {
        log::trace!(
            "Deleting WPA network {} {}",
            iface_obj_path,
            network_obj_path
        );
        let network_obj_path = str_to_obj_path(network_obj_path)?;
        let proxy = zbus::Proxy::new(
            &self.connection,
            WPA_SUP_DBUS_IFACE_ROOT,
            iface_obj_path,
            WPA_SUP_DBUS_IFACE_IFACE,
        )
        .await
        .map_err(map_zbus_err)?;
        proxy
            .call::<&str, ObjectPath, ()>("RemoveNetwork", &network_obj_path)
            .await
            .map_err(map_zbus_err)
    }

    pub(crate) async fn get_current_bss(
        &self,
        iface_obj_path: &str,
    ) -> Result<WpaSupBss, NipartError> {
        log::trace!("NipartWpaSupDbus::get_current_bss(): {iface_obj_path}");
        let iface_obj_path = str_to_obj_path(iface_obj_path)?;
        let proxy = zbus::Proxy::new(
            &self.connection,
            WPA_SUP_DBUS_IFACE_ROOT,
            iface_obj_path,
            WPA_SUP_DBUS_IFACE_IFACE,
        )
        .await
        .map_err(map_zbus_err)?;
        let bss_obj_path = proxy
            .get_property::<OwnedObjectPath>("CurrentBSS")
            .await
            .map_err(map_zbus_err)?;

        let proxy = zbus::Proxy::new(
            &self.connection,
            WPA_SUP_DBUS_IFACE_ROOT,
            bss_obj_path.as_str(),
            DBUS_IFACE_PROPS,
        )
        .await
        .map_err(map_zbus_err)?;

        let value = proxy
            .call::<&str, &str, HashMap<String, zvariant::OwnedValue>>(
                "GetAll",
                &WPA_SUP_DBUS_IFACE_BSS,
            )
            .await
            .map_err(map_zbus_err)?;

        WpaSupBss::from_value(value, bss_obj_path)
    }

    pub(crate) async fn get_bsses(
        &self,
        iface_obj_path: &str,
    ) -> Result<Vec<WpaSupBss>, NipartError> {
        log::trace!("NipartWpaSupDbus::get_bsses(): {iface_obj_path}");
        let iface_obj_path = str_to_obj_path(iface_obj_path)?;
        let proxy = zbus::Proxy::new(
            &self.connection,
            WPA_SUP_DBUS_IFACE_ROOT,
            iface_obj_path,
            WPA_SUP_DBUS_IFACE_IFACE,
        )
        .await
        .map_err(map_zbus_err)?;
        let bss_obj_paths = proxy
            .get_property::<Vec<OwnedObjectPath>>("BSSs")
            .await
            .map_err(map_zbus_err)?;

        let mut ret: Vec<WpaSupBss> = Vec::new();
        for bss_obj_path in bss_obj_paths {
            let proxy = zbus::Proxy::new(
                &self.connection,
                WPA_SUP_DBUS_IFACE_ROOT,
                bss_obj_path.as_str(),
                DBUS_IFACE_PROPS,
            )
            .await
            .map_err(map_zbus_err)?;

            let value = proxy
                .call::<&str, &str, HashMap<String, zvariant::OwnedValue>>(
                    "GetAll",
                    &WPA_SUP_DBUS_IFACE_BSS,
                )
                .await
                .map_err(map_zbus_err)?;
            ret.push(WpaSupBss::from_value(value, bss_obj_path)?);
        }
        Ok(ret)
    }

    pub(crate) async fn scan(
        &self,
        iface_obj_path: &str,
    ) -> Result<(), NipartError> {
        log::trace!("Starting WIFI active scan on {iface_obj_path}",);
        let proxy = zbus::Proxy::new(
            &self.connection,
            WPA_SUP_DBUS_IFACE_ROOT,
            iface_obj_path,
            WPA_SUP_DBUS_IFACE_IFACE,
        )
        .await
        .map_err(map_zbus_err)?;

        let mut scan_args = HashMap::new();
        scan_args.insert("Type", zvariant::Value::new("active".to_string()));

        proxy
            .call::<&str, HashMap<&str, zvariant::Value<'_>>, ()>(
                "Scan", &scan_args,
            )
            .await
            .map_err(map_zbus_err)
    }

    pub(crate) async fn is_iface_scanning(
        &self,
        iface_obj_path: &str,
    ) -> Result<bool, NipartError> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            WPA_SUP_DBUS_IFACE_ROOT,
            iface_obj_path,
            WPA_SUP_DBUS_IFACE_IFACE,
        )
        .await
        .map_err(map_zbus_err)?;
        proxy
            .get_property::<bool>("Scanning")
            .await
            .map_err(map_zbus_err)
    }

    pub(crate) async fn wait_scan(
        &self,
        iface_obj_path: &str,
    ) -> Result<(), NipartError> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            WPA_SUP_DBUS_IFACE_ROOT,
            iface_obj_path,
            WPA_SUP_DBUS_IFACE_IFACE,
        )
        .await
        .map_err(map_zbus_err)?;
        let mut stream = proxy
            .receive_signal("ScanDone")
            .await
            .map_err(map_zbus_err)?;
        stream.next().await;
        Ok(())
    }
}

fn obj_path_to_string(obj_path: OwnedObjectPath) -> String {
    obj_path.into_inner().to_string()
}

fn str_to_obj_path(obj_path_str: &str) -> Result<OwnedObjectPath, NipartError> {
    OwnedObjectPath::try_from(obj_path_str).map_err(|e| {
        NipartError::new(
            ErrorKind::Bug,
            format!(
                "Failed to convert string {obj_path_str} to DBUS object path: \
                 {e}"
            ),
        )
    })
}

pub(crate) fn map_zbus_err(e: zbus::Error) -> NipartError {
    NipartError::new(ErrorKind::Bug, format!("DBUS error of wpa_supplicant: {e}"))
}

pub(crate) fn map_zbus_fdo_err(e: zbus::fdo::Error) -> NipartError {
    NipartError::new(ErrorKind::Bug, format!("DBUS error of wpa_supplicant: {e}"))
}
