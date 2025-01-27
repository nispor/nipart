// SPDX-License-Identifier: Apache-2.0

use std::future::Future;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    NipartError, NipartEvent, NipartEventAddress, NipartLogEntry,
    NipartLogLevel, NipartPluginEvent, NipartPluginInfo, NipartPostStartData,
    NipartRole, NipartUserEvent, NipartUuid,
};

pub trait NipartNativePlugin: Sized + Send + Sync + 'static {
    const PLUGIN_NAME: &'static str;

    fn sender_to_daemon(&self) -> &Sender<NipartEvent>;

    fn recver_from_daemon(&mut self) -> &mut Receiver<NipartEvent>;

    fn roles() -> Vec<NipartRole>;

    fn get_log_level(&self) -> NipartLogLevel;

    fn set_log_level(&mut self, level: NipartLogLevel);

    fn log_trace(
        &self,
        uuid: NipartUuid,
        msg: String,
    ) -> impl Future<Output = ()> + Send {
        self.log(NipartLogLevel::Trace, uuid, msg)
    }

    fn log_debug(
        &self,
        uuid: NipartUuid,
        msg: String,
    ) -> impl Future<Output = ()> + Send {
        self.log(NipartLogLevel::Debug, uuid, msg)
    }

    fn log_info(
        &self,
        uuid: NipartUuid,
        msg: String,
    ) -> impl Future<Output = ()> + Send {
        self.log(NipartLogLevel::Info, uuid, msg)
    }

    fn log_warn(
        &self,
        uuid: NipartUuid,
        msg: String,
    ) -> impl Future<Output = ()> + Send {
        self.log(NipartLogLevel::Warn, uuid, msg)
    }

    fn log_error(
        &self,
        uuid: NipartUuid,
        msg: String,
    ) -> impl Future<Output = ()> + Send {
        self.log(NipartLogLevel::Error, uuid, msg)
    }

    // TODO: Use macro to create a wrapper like log_debug!()
    fn log(
        &self,
        level: NipartLogLevel,
        uuid: NipartUuid,
        msg: String,
    ) -> impl Future<Output = ()> + Send {
        async move {
            if level > self.get_log_level() {
                return;
            }

            let event = NipartLogEntry::new(level, msg).to_event(
                uuid,
                NipartEventAddress::Unicast(Self::PLUGIN_NAME.to_string()),
            );
            if let Err(e) = self.sender_to_daemon().send(event).await {
                log::warn!("Failed to send log: {e}");
            }
        }
    }

    fn plugin_info() -> NipartPluginInfo {
        NipartPluginInfo {
            name: Self::PLUGIN_NAME.to_string(),
            roles: Self::roles(),
        }
    }

    fn init(
        log_level: NipartLogLevel,
        to_daemon: Sender<NipartEvent>,
        from_daemon: Receiver<NipartEvent>,
    ) -> impl Future<Output = Result<Self, NipartError>> + Send;

    fn handle_query_plugin_info(
        uuid: NipartUuid,
        src: &NipartEventAddress,
    ) -> NipartEvent {
        crate::plugin_common::handle_query_plugin_info(
            uuid,
            src,
            Self::plugin_info(),
            Self::PLUGIN_NAME,
        )
    }

    fn handle_change_log_level(
        &mut self,
        log_level: NipartLogLevel,
        uuid: NipartUuid,
    ) -> NipartEvent {
        self.set_log_level(log_level);

        NipartEvent::new_with_uuid(
            uuid,
            NipartUserEvent::None,
            NipartPluginEvent::QueryLogLevelReply(log_level),
            NipartEventAddress::Unicast(Self::PLUGIN_NAME.to_string()),
            NipartEventAddress::Commander,
            crate::DEFAULT_TIMEOUT,
        )
    }

    fn handle_query_log_level(uuid: NipartUuid) -> NipartEvent {
        crate::plugin_common::handle_query_log_level(uuid, Self::PLUGIN_NAME)
    }

    fn handle_plugin_event_post_start(
        &mut self,
        _uuid: NipartUuid,
        _post_start_data: NipartPostStartData,
    ) -> impl std::future::Future<Output = Result<(), NipartError>> + Send {
        async { Ok(()) }
    }

    fn handle_plugin_event(
        &mut self,
        event: NipartEvent,
    ) -> impl std::future::Future<Output = ()> + Send {
        async {
            self.log(
                NipartLogLevel::Debug,
                event.uuid,
                format!("Got event {event}"),
            )
            .await;
            self.log(
                NipartLogLevel::Trace,
                event.uuid,
                format!("Got event {event:?}"),
            )
            .await;

            match event.plugin {
                NipartPluginEvent::QueryPluginInfo => {
                    let reply =
                        Self::handle_query_plugin_info(event.uuid, &event.src);
                    self.log(
                        NipartLogLevel::Debug,
                        event.uuid,
                        format!("Sending {reply}",),
                    )
                    .await;
                    self.log(
                        NipartLogLevel::Trace,
                        event.uuid,
                        format!("Sending {reply:?}",),
                    )
                    .await;
                    if let Err(e) = self.sender_to_daemon().send(reply).await {
                        self.log(
                            NipartLogLevel::Error,
                            event.uuid,
                            format!("{e}",),
                        )
                        .await;
                    }
                }
                NipartPluginEvent::ChangeLogLevel(l) => {
                    let reply = self.handle_change_log_level(l, event.uuid);
                    self.log(
                        NipartLogLevel::Debug,
                        event.uuid,
                        format!("Sending {reply}",),
                    )
                    .await;
                    self.log(
                        NipartLogLevel::Trace,
                        event.uuid,
                        format!("Sending {reply:?}",),
                    )
                    .await;
                    if let Err(e) = self.sender_to_daemon().send(reply).await {
                        self.log(
                            NipartLogLevel::Error,
                            event.uuid,
                            format!("{e}",),
                        )
                        .await;
                    }
                }
                NipartPluginEvent::QueryLogLevel => {
                    let reply = Self::handle_query_log_level(event.uuid);
                    self.log(
                        NipartLogLevel::Debug,
                        event.uuid,
                        format!("Sending {reply}",),
                    )
                    .await;
                    self.log(
                        NipartLogLevel::Trace,
                        event.uuid,
                        format!("Sending {reply:?}",),
                    )
                    .await;
                    if let Err(e) = self.sender_to_daemon().send(reply).await {
                        self.log(
                            NipartLogLevel::Error,
                            event.uuid,
                            format!("{e}",),
                        )
                        .await;
                    }
                }
                NipartPluginEvent::PostStart(data) => {
                    if let Err(e) = self
                        .handle_plugin_event_post_start(event.uuid, *data)
                        .await
                    {
                        self.log(
                            NipartLogLevel::Error,
                            event.uuid,
                            format!("{e}",),
                        )
                        .await;
                    }
                }
                _ => {
                    let uuid = event.uuid;
                    if let Err(e) = self.handle_event(event).await {
                        self.log(NipartLogLevel::Error, uuid, format!("{e}"))
                            .await;
                    }
                }
            }
        }
    }

    fn handle_event(
        &mut self,
        event: NipartEvent,
    ) -> impl Future<Output = Result<(), NipartError>> + Send;

    fn run(&mut self) -> impl std::future::Future<Output = ()> + Send {
        async move {
            loop {
                match self.recver_from_daemon().recv().await {
                    Some(event) if event.plugin == NipartPluginEvent::Quit => {
                        break;
                    }
                    Some(event) => self.handle_plugin_event(event).await,
                    None => {
                        self.log(
                            NipartLogLevel::Debug,
                            0.into(),
                            "MPSC channel remote end closed".to_string(),
                        )
                        .await;

                        break;
                    }
                }
            }
        }
    }
}
