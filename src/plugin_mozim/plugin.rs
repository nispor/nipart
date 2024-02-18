// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use nipart::{
    NipartError, NipartEvent, NipartEventAddress, NipartNativePlugin,
    NipartPlugin, NipartPluginEvent, NipartRole, NipartUserEvent,
};
use tokio::sync::mpsc::Sender;

#[derive(Debug, Default)]
struct NipartPluginMozimShareData {}

impl NipartPluginMozimShareData {
    fn _clear(&mut self) {}
}

#[derive(Debug)]
pub struct NipartPluginMozim {
    _data: Mutex<NipartPluginMozimShareData>,
}

impl NipartPlugin for NipartPluginMozim {
    const PLUGIN_NAME: &'static str = "mozim";
    const LOG_SUFFIX: &'static str = " (plugin mozim)\n";

    fn roles() -> Vec<NipartRole> {
        vec![NipartRole::Dhcp]
    }

    async fn init() -> Result<Self, NipartError> {
        Ok(Self {
            _data: Mutex::new(NipartPluginMozimShareData::default()),
        })
    }

    async fn handle_event(
        _plugin: &Arc<Self>,
        to_daemon: &Sender<NipartEvent>,
        event: NipartEvent,
    ) -> Result<(), NipartError> {
        match event.plugin {
            NipartPluginEvent::QueryDhcpConfig(ifaces) => {
                log::trace!("Querying DHCP config for {ifaces:?}");
                let mut reply = NipartEvent::new(
                    NipartUserEvent::None,
                    NipartPluginEvent::QueryDhcpConfigReply(Box::new(
                        Vec::new(),
                    )),
                    NipartEventAddress::Dhcp,
                    NipartEventAddress::Commander,
                    event.timeout,
                );
                reply.uuid = event.uuid;
                to_daemon.send(reply).await?;
            }
            NipartPluginEvent::ApplyDhcpConfig(confs) => {
                log::trace!("Apply DHCP config for {confs:?}");
                let mut reply = NipartEvent::new(
                    NipartUserEvent::None,
                    NipartPluginEvent::ApplyDhcpConfigReply,
                    NipartEventAddress::Dhcp,
                    NipartEventAddress::Commander,
                    event.timeout,
                );
                reply.uuid = event.uuid;
                to_daemon.send(reply).await?;
            }
            _ => log::warn!("Plugin mozim got unknown event {event}"),
        }
        Ok(())
    }
}

impl NipartNativePlugin for NipartPluginMozim {}
