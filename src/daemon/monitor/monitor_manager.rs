// SPDX-License-Identifier: Apache-2.0

use futures_channel::mpsc::UnboundedSender;
use nipart::{InterfaceType, NipartError};

use super::{
    super::daemon::NipartManagerCmd, NipartMonitorCmd, NipartMonitorReply, NipartMonitorWorker,
};
use crate::TaskManager;

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

    /// Start monitoring on specified interface.
    pub(crate) async fn add_iface_to_monitor(
        &mut self,
        iface_name: &str,
    ) -> Result<(), NipartError> {
        self.mgr
            .exec(NipartMonitorCmd::AddIface(iface_name.to_string()))
            .await?;
        Ok(())
    }

    /// Stop monitoring on specified interface.
    pub(crate) async fn del_iface_from_monitor(
        &mut self,
        iface_name: &str,
    ) -> Result<(), NipartError> {
        self.mgr
            .exec(NipartMonitorCmd::DelIface(iface_name.to_string()))
            .await?;
        Ok(())
    }

    /// Start monitoring on specified interface types.
    pub(crate) async fn add_iface_type_to_monitor(
        &mut self,
        iface_type: InterfaceType,
    ) -> Result<(), NipartError> {
        self.mgr
            .exec(NipartMonitorCmd::AddIfaceType(iface_type))
            .await?;
        Ok(())
    }

    /// Stop monitoring on any WIFI NICs
    pub(crate) async fn del_iface_type_from_monitor(
        &mut self,
        iface_type: InterfaceType,
    ) -> Result<(), NipartError> {
        self.mgr
            .exec(NipartMonitorCmd::DelIfaceType(iface_type))
            .await?;
        Ok(())
    }
}
