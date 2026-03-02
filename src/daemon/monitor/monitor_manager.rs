// SPDX-License-Identifier: Apache-2.0

use futures_channel::mpsc::UnboundedSender;
use nipart::{
    Interface, InterfaceTrigger, MergedNetworkState, NetworkState, NipartError,
    NmstateInterface,
};

use super::{
    super::daemon::NipartManagerCmd, NipartMonitorCmd, NipartMonitorReply,
    NipartMonitorWorker,
};
use crate::TaskManager;

// Responsibilities of NipartMonitorManager:
//  * Parse `MergedNetworkState` into a list of interface/SSID to start/stop
//    monitor

#[derive(Debug, Clone)]
pub(crate) struct NipartMonitorManager {
    mgr: TaskManager<NipartMonitorCmd, NipartMonitorReply>,
    msg_to_commander: UnboundedSender<NipartManagerCmd>,
}

impl NipartMonitorManager {
    pub(crate) async fn new(
        msg_to_commander: UnboundedSender<NipartManagerCmd>,
    ) -> Result<Self, NipartError> {
        let mut ret = Self {
            mgr: TaskManager::new::<NipartMonitorWorker>("monitor").await?,
            msg_to_commander,
        };
        ret.mgr
            .exec(NipartMonitorCmd::SetCommanderSender(
                ret.msg_to_commander.clone(),
            ))
            .await?;
        Ok(ret)
    }

    pub(crate) async fn pause(&mut self) -> Result<(), NipartError> {
        self.mgr.exec(NipartMonitorCmd::Pause).await?;
        Ok(())
    }

    pub(crate) async fn resume(&mut self) -> Result<(), NipartError> {
        self.mgr.exec(NipartMonitorCmd::Resume).await?;
        Ok(())
    }

    // Setup monitor for desired state
    // Use `full_saved_state` to determine whether we should enable or disable
    // WIFI SSID monitoring
    pub(crate) async fn setup_monitor(
        &mut self,
        merged_state: &MergedNetworkState,
        full_saved_state: &NetworkState,
    ) -> Result<(), NipartError> {
        if wifi_monitor_is_needed(full_saved_state) {
            self.enable_wifi_monitor().await?;
        } else {
            self.disable_wifi_monitor().await?;
        }

        for iface in merged_state
            .ifaces
            .iter()
            .filter_map(|m| m.for_apply.as_ref())
        {
            if iface.is_absent() {
                self.del_iface_from_monitor(iface.name()).await?;
            } else if iface.base_iface().trigger.as_ref()
                == Some(&InterfaceTrigger::Carrier)
            {
                self.add_iface_to_monitor(iface.name()).await?;
            }
        }
        Ok(())
    }

    /// Start monitoring on specified interface.
    async fn add_iface_to_monitor(
        &mut self,
        iface_name: &str,
    ) -> Result<(), NipartError> {
        self.mgr
            .exec(NipartMonitorCmd::AddIface(iface_name.to_string()))
            .await?;
        Ok(())
    }

    /// Stop monitoring on specified interface.
    async fn del_iface_from_monitor(
        &mut self,
        iface_name: &str,
    ) -> Result<(), NipartError> {
        self.mgr
            .exec(NipartMonitorCmd::DelIface(iface_name.to_string()))
            .await?;
        Ok(())
    }

    /// Enable WIFI SSID monitoring.
    async fn enable_wifi_monitor(&mut self) -> Result<(), NipartError> {
        self.mgr.exec(NipartMonitorCmd::EnableWifiMonitor).await?;
        Ok(())
    }

    /// Disable WIFI SSID monitoring.
    async fn disable_wifi_monitor(&mut self) -> Result<(), NipartError> {
        self.mgr.exec(NipartMonitorCmd::DisableWifiMonitor).await?;
        Ok(())
    }
}

fn wifi_monitor_is_needed(full_saved_state: &NetworkState) -> bool {
    for iface in full_saved_state.ifaces.iter().filter(|i| !i.is_absent()) {
        if let Interface::WifiCfg(wifi_iface) = iface
            && wifi_iface.ssid().is_some()
        {
            return true;
        } else if let Some(trigger) = iface.base_iface().trigger.as_ref()
            && trigger.is_wifi()
        {
            return true;
        }
    }
    false
}
