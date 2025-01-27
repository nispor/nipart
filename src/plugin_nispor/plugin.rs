// SPDX-License-Identifier: Apache-2.0

use nipart::{
    MergedNetworkState, NipartApplyOption, NipartDhcpLease, NipartError,
    NipartEvent, NipartEventAddress, NipartLogLevel, NipartNativePlugin,
    NipartPluginEvent, NipartRole, NipartUserEvent, NipartUuid,
    DEFAULT_TIMEOUT,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::apply::{nispor_apply, nispor_apply_dhcp_lease};
use crate::show::nispor_retrieve;

const STATE_PRIORITY: u32 = 50;

#[derive(Debug)]
#[non_exhaustive]
pub struct NipartPluginNispor {
    log_level: NipartLogLevel,
    to_daemon: Sender<NipartEvent>,
    from_daemon: Receiver<NipartEvent>,
}

impl NipartNativePlugin for NipartPluginNispor {
    const PLUGIN_NAME: &'static str = "nispor";

    fn roles() -> Vec<NipartRole> {
        vec![NipartRole::QueryAndApply, NipartRole::ApplyDhcpLease]
    }

    fn recver_from_daemon(&mut self) -> &mut Receiver<NipartEvent> {
        &mut self.from_daemon
    }

    fn sender_to_daemon(&self) -> &Sender<NipartEvent> {
        &self.to_daemon
    }

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
            to_daemon,
            from_daemon,
        })
    }

    async fn handle_event(
        &mut self,
        event: NipartEvent,
    ) -> Result<(), NipartError> {
        match event.plugin {
            NipartPluginEvent::QueryNetState(_) => {
                let state = nispor_retrieve(false).await?;
                let mut reply = NipartEvent::new(
                    NipartUserEvent::None,
                    NipartPluginEvent::QueryNetStateReply(
                        Box::new(state),
                        STATE_PRIORITY,
                    ),
                    NipartEventAddress::Unicast(Self::PLUGIN_NAME.to_string()),
                    NipartEventAddress::Commander,
                    DEFAULT_TIMEOUT,
                );
                reply.uuid = event.uuid;
                self.sender_to_daemon().send(reply).await?;
                Ok(())
            }
            // TODO: Currently, we are returning full state, but we should
            // return       only related network state back
            NipartPluginEvent::QueryRelatedNetState(_) => {
                let state = nispor_retrieve(false).await?;
                let mut reply = NipartEvent::new(
                    event.user.clone(),
                    NipartPluginEvent::QueryNetStateReply(
                        Box::new(state),
                        STATE_PRIORITY,
                    ),
                    NipartEventAddress::Unicast(Self::PLUGIN_NAME.to_string()),
                    NipartEventAddress::Commander,
                    DEFAULT_TIMEOUT,
                );
                reply.uuid = event.uuid;
                self.sender_to_daemon().send(reply).await?;
                Ok(())
            }
            NipartPluginEvent::ApplyNetState(merged_state, opt) => {
                // We spawn new thread for apply instead of blocking
                // here
                let to_daemon_clone = self.sender_to_daemon().clone();
                tokio::spawn(async move {
                    handle_apply(
                        *merged_state,
                        opt,
                        to_daemon_clone,
                        event.uuid,
                    )
                    .await
                });
                Ok(())
            }
            NipartPluginEvent::ApplyDhcpLease(lease) => {
                // We spawn new thread for apply instead of blocking
                // here
                let to_daemon_clone = self.sender_to_daemon().clone();
                tokio::spawn(async move {
                    handle_apply_dhcp_lease(*lease, to_daemon_clone, event.uuid)
                        .await
                });
                Ok(())
            }
            _ => {
                log::warn!("Plugin nispor got unknown event {event:?}");
                Ok(())
            }
        }
    }
}

async fn handle_apply(
    merged_state: MergedNetworkState,
    opt: NipartApplyOption,
    to_daemon: Sender<NipartEvent>,
    uuid: NipartUuid,
) {
    let mut reply = match nispor_apply(merged_state, opt).await {
        Ok(()) => NipartEvent::new(
            NipartUserEvent::None,
            NipartPluginEvent::ApplyNetStateReply,
            NipartEventAddress::Unicast(
                NipartPluginNispor::PLUGIN_NAME.to_string(),
            ),
            NipartEventAddress::Commander,
            DEFAULT_TIMEOUT,
        ),
        Err(e) => NipartEvent::new(
            NipartUserEvent::Error(e),
            NipartPluginEvent::ApplyNetStateReply,
            NipartEventAddress::Unicast(
                NipartPluginNispor::PLUGIN_NAME.to_string(),
            ),
            NipartEventAddress::Commander,
            DEFAULT_TIMEOUT,
        ),
    };
    reply.uuid = uuid;
    log::trace!("Sending reply {reply:?}");
    if let Err(e) = to_daemon.send(reply).await {
        log::error!("Failed to reply {e}")
    }
}

async fn handle_apply_dhcp_lease(
    lease: NipartDhcpLease,
    to_daemon: Sender<NipartEvent>,
    uuid: NipartUuid,
) {
    let mut reply = match nispor_apply_dhcp_lease(lease).await {
        Ok(()) => NipartEvent::new(
            NipartUserEvent::None,
            NipartPluginEvent::ApplyDhcpLeaseReply,
            NipartEventAddress::Unicast(
                NipartPluginNispor::PLUGIN_NAME.to_string(),
            ),
            NipartEventAddress::Commander,
            DEFAULT_TIMEOUT,
        ),
        Err(e) => NipartEvent::new(
            NipartUserEvent::Error(e),
            NipartPluginEvent::ApplyDhcpLeaseReply,
            NipartEventAddress::Unicast(
                NipartPluginNispor::PLUGIN_NAME.to_string(),
            ),
            NipartEventAddress::Commander,
            DEFAULT_TIMEOUT,
        ),
    };
    reply.uuid = uuid;
    log::trace!("Sending reply {reply:?}");
    if let Err(e) = to_daemon.send(reply).await {
        log::error!("Failed to reply {e}")
    }
}
