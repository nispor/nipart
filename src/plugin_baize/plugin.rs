// SPDX-License-Identifier: Apache-2.0

use nipart::{
    NipartError, NipartEvent, NipartLogLevel, NipartMonitorRule,
    NipartNativePlugin, NipartPluginEvent, NipartRole,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::link::BaizeLinkMonitor;

#[derive(Debug)]
pub struct NipartPluginBaize {
    log_level: NipartLogLevel,
    to_daemon: Sender<NipartEvent>,
    from_daemon: Receiver<NipartEvent>,
    link_monitor: BaizeLinkMonitor,
}

impl NipartNativePlugin for NipartPluginBaize {
    const PLUGIN_NAME: &'static str = "baize";

    fn get_log_level(&self) -> NipartLogLevel {
        self.log_level
    }

    fn set_log_level(&mut self, level: NipartLogLevel) {
        self.log_level = level;
    }

    async fn init(
        log_level: NipartLogLevel,
        to_daemon: Sender<NipartEvent>,
        from_daemon: Receiver<NipartEvent>,
    ) -> Result<Self, NipartError> {
        Ok(Self {
            log_level,
            to_daemon: to_daemon.clone(),
            from_daemon,
            link_monitor: BaizeLinkMonitor::new(to_daemon)?,
        })
    }

    fn recver_from_daemon(&mut self) -> &mut Receiver<NipartEvent> {
        &mut self.from_daemon
    }

    fn sender_to_daemon(&self) -> &Sender<NipartEvent> {
        &self.to_daemon
    }

    fn roles() -> Vec<NipartRole> {
        vec![NipartRole::Monitor]
    }

    async fn handle_event(
        &mut self,
        event: NipartEvent,
    ) -> Result<(), NipartError> {
        match event.plugin {
            NipartPluginEvent::RegisterMonitorRule(rule) => {
                log::trace!("Registering monitor rule {rule:?}");
                self.register_monitor_rule(*rule).await?;
            }
            NipartPluginEvent::RemoveMonitorRule(rule) => {
                log::trace!("Registering monitor rule {rule:?}");
                self.remove_monitor_rule(*rule).await?;
            }
            _ => log::warn!("Plugin baize got unknown event {event}"),
        }
        Ok(())
    }
}

impl NipartPluginBaize {
    async fn register_monitor_rule(
        &mut self,
        rule: NipartMonitorRule,
    ) -> Result<(), NipartError> {
        match rule {
            NipartMonitorRule::Link(rule) => {
                self.link_monitor.add_link_rule(rule).await
            }
            _ => {
                log::error!("TODO: register_monitor_rule() {rule}");
                Ok(())
            }
        }
    }

    async fn remove_monitor_rule(
        &mut self,
        rule: NipartMonitorRule,
    ) -> Result<(), NipartError> {
        match rule {
            NipartMonitorRule::Link(rule) => {
                self.link_monitor.del_link_rule(rule).await?;
                Ok(())
            }
            _ => {
                log::error!("TODO: remove_monitor_rule() {rule}");
                Ok(())
            }
        }
    }
}
