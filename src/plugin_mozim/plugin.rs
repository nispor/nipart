// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use nipart::{
    NipartDhcpConfig, NipartError, NipartEvent, NipartEventAddress,
    NipartNativePlugin, NipartPluginEvent, NipartRole, NipartUserEvent,
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
                    self.apply_dhcp_conf(&conf, event.uuid)?;
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

    fn apply_dhcp_conf(
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
                let mut worker = MozimWorkerV4::new(conf, event_uuid)?;
                //TODO: only invoke run after got link up event;
                worker.run(self.to_daemon())?;
                self.v4_workers.insert(conf.iface.clone(), worker);
            }
            NipartDhcpConfig::V6(_) => {
                log::error!("Plugin mozim not supporting IPv6 yet");
            }
        }
        Ok(())
    }
}
