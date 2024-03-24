// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use nipart::{
    NipartDhcpConfig, NipartError, NipartEvent, NipartEventAddress,
    NipartLinkMonitorKind, NipartLinkMonitorRule, NipartMonitorEvent,
    NipartMonitorRule, NipartNativePlugin, NipartPluginEvent, NipartRole,
    NipartUserEvent,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::worker::MozimWorkerV4;

// TODO: Use /var/lib folder to store DHCP lease files

#[derive(Debug)]
pub struct NipartPluginMozim {
    to_daemon: Sender<NipartEvent>,
    from_daemon: Receiver<NipartEvent>,
    v4_workers: HashMap<String, MozimWorkerV4>,
}

impl NipartNativePlugin for NipartPluginMozim {
    const PLUGIN_NAME: &'static str = "mozim";

    async fn init(
        to_daemon: Sender<NipartEvent>,
        from_daemon: Receiver<NipartEvent>,
    ) -> Result<Self, NipartError> {
        Ok(Self {
            to_daemon,
            from_daemon,
            v4_workers: HashMap::new(),
        })
    }

    fn from_daemon(&mut self) -> &mut Receiver<NipartEvent> {
        &mut self.from_daemon
    }

    fn to_daemon(&self) -> &Sender<NipartEvent> {
        &self.to_daemon
    }

    fn roles() -> Vec<NipartRole> {
        vec![NipartRole::Dhcp]
    }

    async fn handle_event(
        &mut self,
        event: NipartEvent,
    ) -> Result<(), NipartError> {
        match event.plugin {
            NipartPluginEvent::QueryDhcpConfig(ifaces) => {
                log::trace!("Querying DHCP config for {ifaces:?}");
                let mut reply = NipartEvent::new(
                    NipartUserEvent::None,
                    NipartPluginEvent::QueryDhcpConfigReply(Box::new(
                        self.query_dhcp_conf(),
                    )),
                    NipartEventAddress::Dhcp,
                    NipartEventAddress::Commander,
                    event.timeout,
                );
                reply.uuid = event.uuid;
                self.to_daemon().send(reply).await?;
            }
            NipartPluginEvent::ApplyDhcpConfig(confs) => {
                log::trace!("Apply DHCP config for {confs:?}");
                for conf in (*confs).as_slice() {
                    self.apply_dhcp_conf(&conf, event.uuid).await?;
                }
                let mut reply = NipartEvent::new(
                    NipartUserEvent::None,
                    NipartPluginEvent::ApplyDhcpConfigReply,
                    NipartEventAddress::Dhcp,
                    NipartEventAddress::Commander,
                    event.timeout,
                );
                reply.uuid = event.uuid;
                self.to_daemon().send(reply).await?;
            }
            NipartPluginEvent::GotMonitorEvent(monitor_event) => {
                self.got_monitor_event(*monitor_event).await?;
            }
            _ => log::warn!("Plugin mozim got unknown event {event}"),
        }
        Ok(())
    }
}

impl NipartPluginMozim {
    fn query_dhcp_conf(&self) -> Vec<NipartDhcpConfig> {
        self.v4_workers
            .values()
            .map(|worker| NipartDhcpConfig::V4(worker.get_config()))
            .collect()
    }

    async fn apply_dhcp_conf(
        &mut self,
        dhcp_conf: &NipartDhcpConfig,
        event_uuid: u128,
    ) -> Result<(), NipartError> {
        match dhcp_conf {
            NipartDhcpConfig::V4(conf) => {
                log::debug!("{event_uuid} applying DHCP {conf:?}");
                // Stop current DHCP process
                self.v4_workers.remove(conf.iface.as_str());
                // Create new one
                let worker = MozimWorkerV4::new(
                    conf,
                    event_uuid,
                    self.to_daemon.clone(),
                )
                .await?;

                if conf.enabled {
                    self.register_link_up_event(
                        conf.iface.as_str(),
                        event_uuid,
                    )
                    .await?;
                }
                self.v4_workers.insert(conf.iface.clone(), worker);
            }
            NipartDhcpConfig::V6(_) => {
                log::error!("Plugin mozim not supporting IPv6 yet");
            }
        }
        Ok(())
    }

    async fn register_link_up_event(
        &self,
        iface: &str,
        event_uuid: u128,
    ) -> Result<(), NipartError> {
        let mut reply = NipartEvent::new(
            NipartUserEvent::None,
            NipartPluginEvent::RegisterMonitorRule(Box::new(
                NipartMonitorRule::Link(NipartLinkMonitorRule::new(
                    NipartLinkMonitorKind::Up,
                    NipartEventAddress::Dhcp,
                    event_uuid,
                    iface.to_string(),
                )),
            )),
            NipartEventAddress::Dhcp,
            NipartEventAddress::Group(NipartRole::Monitor),
            nipart::DEFAULT_TIMEOUT,
        );
        reply.uuid = event_uuid;
        log::debug!("Registering link up monitor {reply}");
        self.to_daemon().send(reply).await?;
        Ok(())
    }

    async fn got_monitor_event(
        &mut self,
        monitor_event: NipartMonitorEvent,
    ) -> Result<(), NipartError> {
        match monitor_event {
            NipartMonitorEvent::LinkUp(iface) => {
                self.start_dhcp_if_enabled(iface.as_str())
            }
            NipartMonitorEvent::LinkDown(iface) => {
                self.stop_dhcp_if_enabled(iface.as_str()).await;
                Ok(())
            }
            _ => {
                log::warn!("Got unexpected monitor event {monitor_event:?}");
                Ok(())
            }
        }
    }

    fn start_dhcp_if_enabled(
        &mut self,
        iface: &str,
    ) -> Result<(), NipartError> {
        if let Some(worker) = self.v4_workers.get_mut(iface) {
            if worker.config.enabled {
                worker.start()
            } else {
                log::debug!(
                    "DHCP is disabled for interface {iface}, \
                    ignore request of start DHCP"
                );
                Ok(())
            }
        } else {
            log::debug!(
                "No DHCP worker registered to mozim for interface {iface}"
            );
            Ok(())
        }
    }

    async fn stop_dhcp_if_enabled(&mut self, iface: &str) {
        if let Some(worker) = self.v4_workers.get_mut(iface) {
            worker.stop().await;
        }
    }
}
