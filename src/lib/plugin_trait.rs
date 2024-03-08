// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::str::FromStr;
use std::sync::Arc;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    NipartConnection, NipartConnectionListener, NipartError, NipartEvent,
    NipartEventAddress, NipartLogLevel, NipartPluginEvent, NipartPluginInfo,
    NipartRole, NipartUserEvent,
};

const DEFAULT_PLUGIN_SOCKET_PREFIX: &str = "nipart_plugin_";
const DEFAULT_LOG_LEVEL: log::LevelFilter = log::LevelFilter::Trace;
const DEFAULT_QUIT_FLAG_CHECK_INTERVAL: u64 = 1000;
const MPSC_CHANNLE_SIZE: usize = 64;

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct NipartPluginRunner {
    pub quit_flag: std::sync::atomic::AtomicBool,
}

fn get_conf_from_argv(name: &str) -> (String, String, log::LevelFilter) {
    let argv: Vec<String> = std::env::args().collect();
    let binary_path = if let Some(b) = argv.first() {
        b.to_string()
    } else {
        String::new()
    };
    let socket_path = if let Some(p) = argv.get(1) {
        p.to_string()
    } else {
        format!("{DEFAULT_PLUGIN_SOCKET_PREFIX}{name}")
    };
    let log_level = if let Some(l) =
        argv.get(2).and_then(|l| log::LevelFilter::from_str(l).ok())
    {
        l
    } else {
        DEFAULT_LOG_LEVEL
    };
    (binary_path, socket_path, log_level)
}

fn _handle_query_plugin_info(
    uuid: u128,
    src: &NipartEventAddress,
    plugin_info: NipartPluginInfo,
    plugin_name: &str,
) -> NipartEvent {
    log::debug!("Querying plugin info of {}", plugin_name);
    let mut reply = NipartEvent::new(
        NipartUserEvent::None,
        NipartPluginEvent::QueryPluginInfoReply(plugin_info),
        NipartEventAddress::Unicast(plugin_name.to_string()),
        src.clone(),
        crate::DEFAULT_TIMEOUT,
    );
    reply.uuid = uuid;
    reply
}

fn _handle_change_log_level(
    log_level: NipartLogLevel,
    uuid: u128,
    plugin_name: &str,
) -> NipartEvent {
    log::debug!("Setting log level of {} to {log_level}", plugin_name);
    log::set_max_level(log_level.into());
    let mut reply = NipartEvent::new(
        NipartUserEvent::None,
        NipartPluginEvent::QueryLogLevelReply(log_level),
        NipartEventAddress::Unicast(plugin_name.to_string()),
        NipartEventAddress::Commander,
        crate::DEFAULT_TIMEOUT,
    );
    reply.uuid = uuid;
    reply
}

fn _handle_query_log_level(uuid: u128, plugin_name: &str) -> NipartEvent {
    log::debug!("Querying log level of {}", plugin_name);
    let mut reply = NipartEvent::new(
        NipartUserEvent::None,
        NipartPluginEvent::QueryLogLevelReply(log::max_level().into()),
        NipartEventAddress::Unicast(plugin_name.to_string()),
        NipartEventAddress::Commander,
        crate::DEFAULT_TIMEOUT,
    );
    reply.uuid = uuid;
    reply
}

pub trait NipartExternalPlugin: Sized + Send + Sync + 'static {
    const PLUGIN_NAME: &'static str;
    const LOG_SUFFIX: &'static str;

    fn roles() -> Vec<NipartRole>;

    fn init() -> impl Future<Output = Result<Self, NipartError>> + Send;

    /// Please override this function if you need special action for graceful
    /// quit.
    fn quit(&self) -> impl Future<Output = ()> + Send {
        async {}
    }

    fn plugin_info() -> NipartPluginInfo {
        NipartPluginInfo {
            name: Self::PLUGIN_NAME.to_string(),
            roles: Self::roles(),
        }
    }

    fn handle_query_plugin_info(
        uuid: u128,
        src: &NipartEventAddress,
    ) -> NipartEvent {
        _handle_query_plugin_info(
            uuid,
            src,
            Self::plugin_info(),
            Self::PLUGIN_NAME,
        )
    }

    fn handle_change_log_level(
        log_level: NipartLogLevel,
        uuid: u128,
    ) -> NipartEvent {
        _handle_change_log_level(log_level, uuid, Self::PLUGIN_NAME)
    }

    fn handle_query_log_level(uuid: u128) -> NipartEvent {
        _handle_query_log_level(uuid, Self::PLUGIN_NAME)
    }

    fn handle_plugin_event(
        plugin: &Arc<Self>,
        to_daemon: &Sender<NipartEvent>,
        event: NipartEvent,
    ) -> impl std::future::Future<Output = ()> + Send {
        async {
            log::debug!("Plugin {} got event {event}", Self::PLUGIN_NAME);
            log::trace!("Plugin {} got event {event:?}", Self::PLUGIN_NAME);
            match event.plugin {
                NipartPluginEvent::Quit => {
                    let mut reply = NipartEvent::new(
                        NipartUserEvent::None,
                        NipartPluginEvent::Quit,
                        NipartEventAddress::Unicast(
                            Self::PLUGIN_NAME.to_string(),
                        ),
                        NipartEventAddress::Commander,
                        crate::DEFAULT_TIMEOUT,
                    );
                    reply.uuid = event.uuid;
                    log::debug!("Sending {event}");
                    to_daemon.send(reply).await.ok();
                }
                NipartPluginEvent::QueryPluginInfo => {
                    let event =
                        Self::handle_query_plugin_info(event.uuid, &event.src);
                    log::debug!("Sending {event}");
                    log::trace!("Sending {event:?}");
                    if let Err(e) = to_daemon.send(event).await {
                        log::error!("{e}");
                    }
                }
                NipartPluginEvent::ChangeLogLevel(l) => {
                    let event = Self::handle_change_log_level(l, event.uuid);
                    log::debug!("Sending {event}");
                    log::trace!("Sending {event:?}");
                    if let Err(e) = to_daemon.send(event).await {
                        log::error!("{e}");
                    }
                }
                NipartPluginEvent::QueryLogLevel => {
                    let event = Self::handle_query_log_level(event.uuid);
                    log::debug!("Sending {event}");
                    log::trace!("Sending {event:?}");
                    if let Err(e) = to_daemon.send(event).await {
                        log::error!("{e}");
                    }
                }
                _ => {
                    if let Err(e) =
                        Self::handle_event(plugin, to_daemon, event).await
                    {
                        log::error!("{e}");
                    }
                }
            }
        }
    }

    fn handle_event(
        plugin: &Arc<Self>,
        to_daemon: &Sender<NipartEvent>,
        event: NipartEvent,
    ) -> impl Future<Output = Result<(), NipartError>> + Send;

    fn init_logger(binary_path: &str, log_level: log::LevelFilter) {
        let mut log_builder = env_logger::Builder::new();
        log_builder.format_suffix(Self::LOG_SUFFIX);
        let plugin_bin_path = std::path::Path::new(&binary_path).file_name();
        log_builder.filter(Some("nipart"), log_level);
        if let Some(p) = plugin_bin_path.and_then(std::ffi::OsStr::to_str) {
            log_builder.filter(Some(p), log_level);
        }
        log_builder.init();
    }

    fn run() -> impl Future<Output = Result<(), NipartError>> + Send {
        async {
            let (binary_path, socket_path, log_level) =
                get_conf_from_argv(Self::PLUGIN_NAME);
            Self::init_logger(&binary_path, log_level);

            let listener =
                NipartConnectionListener::new_abstract(&socket_path)?;
            log::debug!(
                "Nipart plugin {} is listening on {}",
                Self::PLUGIN_NAME,
                socket_path
            );
            let runner = Arc::new(NipartPluginRunner::default());
            let plugin = Arc::new(Self::init().await?);

            loop {
                if runner.quit_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    log::info!(
                        "Nipart plugin {} got quit signal, quitting",
                        Self::PLUGIN_NAME
                    );
                    plugin.quit().await;
                    return Ok(());
                }
                match tokio::time::timeout(
                    std::time::Duration::from_millis(
                        DEFAULT_QUIT_FLAG_CHECK_INTERVAL,
                    ),
                    listener.accept(),
                )
                .await
                {
                    Ok(Ok(np_conn)) => {
                        // TODO: Limit the maximum connected client as it could
                        //       from suspicious source, not daemon
                        let runner_clone = runner.clone();
                        let plugin_clone = plugin.clone();

                        tokio::spawn(async move {
                            Self::handle_connection(
                                runner_clone,
                                plugin_clone,
                                np_conn,
                            )
                            .await
                        });
                    }
                    Ok(Err(e)) => {
                        log::error!(
                            "Nipart plugin {} failed to accept connection {}",
                            Self::PLUGIN_NAME,
                            e
                        );
                    }
                    _ => {
                        // timeout, we need to check the quit_flag again
                    }
                }
            }
        }
    }

    fn handle_connection(
        runner: Arc<NipartPluginRunner>,
        plugin: Arc<Self>,
        mut np_conn: NipartConnection,
    ) -> impl Future<Output = ()> + Send {
        async move {
            let (to_daemon_tx, mut to_daemon_rx) =
                tokio::sync::mpsc::channel::<NipartEvent>(MPSC_CHANNLE_SIZE);
            loop {
                tokio::select! {
                    Some(event) = to_daemon_rx.recv() => {
                        log::debug!("Sending {event}");
                        if let Err(e)  = np_conn.send(&event).await {
                            log::warn!(
                                "Failed to send to daemon {event:?}: {e}");
                        }
                        if event.plugin == NipartPluginEvent::Quit {
                            runner
                                .quit_flag
                                .store(true,
                                       std::sync::atomic::Ordering::Relaxed);
                        }
                    },
                    result = np_conn.recv::<NipartEvent>() => {
                        match result {
                            Ok(event) => {
                                Self::handle_plugin_event(
                                    &plugin,
                                    &to_daemon_tx,
                                    event,
                                ).await;
                            },
                            Err(e) => {
                                // Connection might be disconnected
                                log::debug!("{e}");
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
}

pub trait NipartNativePlugin: Sized + Send + Sync + 'static {
    const PLUGIN_NAME: &'static str;

    fn to_daemon(&self) -> &Sender<NipartEvent>;

    fn from_daemon(&mut self) -> &mut Receiver<NipartEvent>;

    fn roles() -> Vec<NipartRole>;

    fn plugin_info() -> NipartPluginInfo {
        NipartPluginInfo {
            name: Self::PLUGIN_NAME.to_string(),
            roles: Self::roles(),
        }
    }

    fn init(
        to_daemon: Sender<NipartEvent>,
        from_daemon: Receiver<NipartEvent>,
    ) -> impl Future<Output = Result<Self, NipartError>> + Send;

    fn handle_query_plugin_info(
        uuid: u128,
        src: &NipartEventAddress,
    ) -> NipartEvent {
        _handle_query_plugin_info(
            uuid,
            src,
            Self::plugin_info(),
            Self::PLUGIN_NAME,
        )
    }

    fn handle_change_log_level(
        log_level: NipartLogLevel,
        uuid: u128,
    ) -> NipartEvent {
        _handle_change_log_level(log_level, uuid, Self::PLUGIN_NAME)
    }

    fn handle_query_log_level(uuid: u128) -> NipartEvent {
        _handle_query_log_level(uuid, Self::PLUGIN_NAME)
    }

    fn handle_plugin_event(
        &mut self,
        event: NipartEvent,
    ) -> impl std::future::Future<Output = ()> + Send {
        async {
            log::debug!("Plugin {} got event {event}", Self::PLUGIN_NAME);
            log::trace!("Plugin {} got event {event:?}", Self::PLUGIN_NAME);
            let to_daemon = self.to_daemon();

            match event.plugin {
                NipartPluginEvent::QueryPluginInfo => {
                    let event =
                        Self::handle_query_plugin_info(event.uuid, &event.src);
                    log::debug!("Sending {event}");
                    log::trace!("Sending {event:?}");
                    if let Err(e) = to_daemon.send(event).await {
                        log::error!("{e}");
                    }
                }
                NipartPluginEvent::ChangeLogLevel(l) => {
                    let event = Self::handle_change_log_level(l, event.uuid);
                    log::debug!("Sending {event}");
                    log::trace!("Sending {event:?}");
                    if let Err(e) = to_daemon.send(event).await {
                        log::error!("{e}");
                    }
                }
                NipartPluginEvent::QueryLogLevel => {
                    let event = Self::handle_query_log_level(event.uuid);
                    log::debug!("Sending {event}");
                    log::trace!("Sending {event:?}");
                    if let Err(e) = to_daemon.send(event).await {
                        log::error!("{e}");
                    }
                }
                _ => {
                    if let Err(e) = self.handle_event(event).await {
                        log::error!("{e}");
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
                match self.from_daemon().recv().await {
                    Some(event) if event.plugin == NipartPluginEvent::Quit => {
                        break;
                    }
                    Some(event) => self.handle_plugin_event(event).await,
                    None => {
                        log::debug!("MPSC channel remote end closed");
                        break;
                    }
                }
            }
        }
    }
}
