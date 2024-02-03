// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;
use std::sync::Arc;

use tokio::sync::mpsc::Sender;

use crate::{
    NipartConnection, NipartConnectionListener, NipartError, NipartEvent,
    NipartEventAction, NipartEventAddress, NipartLogLevel, NipartPluginEvent,
    NipartPluginInfo, NipartRole, NipartUserEvent,
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

pub trait NipartPlugin: Sized + Send + Sync + 'static {
    const PLUGIN_NAME: &'static str;
    const LOG_SUFFIX: &'static str;

    fn name(&self) -> &'static str {
        Self::PLUGIN_NAME
    }

    fn roles(&self) -> Vec<NipartRole>;

    fn get_socket_path(&self) -> &str;

    /// Please store the socket_path in your struct, no need to bind the socket
    /// the run() will bind the socket.
    fn init(
        socket_path: &str,
    ) -> impl std::future::Future<Output = Result<Self, NipartError>> + Send;

    /// Please override this function if you need special action for graceful
    /// quit.
    fn quit(&self) -> impl std::future::Future<Output = ()> + Send {
        async {}
    }

    fn get_plugin_info(&self) -> NipartPluginInfo {
        NipartPluginInfo {
            socket_path: self.get_socket_path().to_string(),
            name: self.name().to_string(),
            roles: self.roles(),
        }
    }

    fn run() -> impl std::future::Future<Output = Result<(), NipartError>> + Send
    {
        async {
            let mut log_builder = env_logger::Builder::new();
            log_builder.format_suffix(Self::LOG_SUFFIX);
            let (binary_path, socket_path, log_level) =
                get_conf_from_argv(Self::PLUGIN_NAME);
            let plugin_bin_path =
                std::path::Path::new(&binary_path).file_name();
            log_builder.filter(Some("nipart"), log_level);
            if let Some(p) = plugin_bin_path.and_then(std::ffi::OsStr::to_str) {
                log_builder.filter(Some(p), log_level);
            }
            log_builder.init();

            let listener =
                NipartConnectionListener::new_abstract(&socket_path)?;
            log::debug!(
                "Nipart plugin {} is listening on {}",
                Self::PLUGIN_NAME,
                socket_path
            );
            let runner = Arc::new(NipartPluginRunner::default());
            let plugin = Arc::new(Self::init(socket_path.as_str()).await?);

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

    fn handle_event(
        plugin: &Arc<Self>,
        to_daemon: &Sender<NipartEvent>,
        event: NipartEvent,
    ) -> impl std::future::Future<Output = Result<(), NipartError>> + Send;

    fn handle_connection(
        runner: Arc<NipartPluginRunner>,
        plugin: Arc<Self>,
        mut np_conn: NipartConnection,
    ) -> impl std::future::Future<Output = ()> + Send {
        async move {
            let (to_daemon_tx, mut to_daemon_rx) =
                tokio::sync::mpsc::channel::<NipartEvent>(MPSC_CHANNLE_SIZE);
            loop {
                tokio::select! {
                    Some(event) = to_daemon_rx.recv() => {
                        if let Err(e)  = np_conn.send(&event).await {
                            log::warn!(
                                "Failed to send to daemon {event:?}: {e}");
                        }
                    },
                    result = np_conn.recv::<NipartEvent>() => {
                        match result {
                            Ok(event) => {
                                handle_plugin_event(
                                    &runner,
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

fn get_conf_from_argv(name: &str) -> (String, String, log::LevelFilter) {
    let argv: Vec<String> = std::env::args().collect();
    let binary_path = if let Some(b) = argv.get(0) {
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

async fn handle_query_plugin_info<T>(
    event: &NipartEvent,
    to_daemon: &Sender<NipartEvent>,
    plugin: Arc<T>,
) where
    T: NipartPlugin,
{
    log::debug!("Querying plugin info of {}", T::PLUGIN_NAME);
    let mut reply = NipartEvent::new(
        NipartEventAction::Done,
        NipartUserEvent::None,
        NipartPluginEvent::QueryPluginInfoReply(plugin.get_plugin_info()),
        NipartEventAddress::Unicast(T::PLUGIN_NAME.to_string()),
        event.src.clone(),
        crate::DEFAULT_TIMEOUT,
    );
    reply.uuid = event.uuid;
    log::trace!("Sending reply {reply:?}");
    if let Err(e) = to_daemon.send(reply).await {
        log::warn!("Failed to send event: {e}");
    }
}

async fn handle_change_log_level<T>(
    log_level: NipartLogLevel,
    uuid: u128,
    to_daemon: &Sender<NipartEvent>,
) where
    T: NipartPlugin,
{
    log::debug!("Setting log level of {} to {log_level}", T::PLUGIN_NAME);
    log::set_max_level(log_level.into());
    let mut reply = NipartEvent::new(
        NipartEventAction::Done,
        NipartUserEvent::None,
        NipartPluginEvent::QueryLogLevelReply(log_level),
        NipartEventAddress::Unicast(T::PLUGIN_NAME.to_string()),
        NipartEventAddress::Commander,
        crate::DEFAULT_TIMEOUT,
    );
    reply.uuid = uuid;
    log::trace!("Sending reply {reply:?}");
    if let Err(e) = to_daemon.send(reply).await {
        log::warn!("Failed to send event: {e}");
    }
}

async fn handle_query_log_level<T>(uuid: u128, to_daemon: &Sender<NipartEvent>)
where
    T: NipartPlugin,
{
    log::debug!("Querying log level of {}", T::PLUGIN_NAME);
    let mut reply = NipartEvent::new(
        NipartEventAction::Done,
        NipartUserEvent::None,
        NipartPluginEvent::QueryLogLevelReply(log::max_level().into()),
        NipartEventAddress::Unicast(T::PLUGIN_NAME.to_string()),
        NipartEventAddress::Commander,
        crate::DEFAULT_TIMEOUT,
    );
    reply.uuid = uuid;
    log::trace!("Sending reply {reply:?}");
    if let Err(e) = to_daemon.send(reply).await {
        log::warn!("Failed to send event: {e}");
    }
}

async fn handle_plugin_event<T>(
    runner: &Arc<NipartPluginRunner>,
    plugin: &Arc<T>,
    to_daemon: &Sender<NipartEvent>,
    event: NipartEvent,
) where
    T: NipartPlugin,
{
    match event.plugin {
        NipartPluginEvent::Quit => {
            runner
                .quit_flag
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        NipartPluginEvent::QueryPluginInfo => {
            handle_query_plugin_info::<T>(&event, to_daemon, plugin.clone())
                .await;
        }
        NipartPluginEvent::ChangeLogLevel(l) => {
            handle_change_log_level::<T>(l, event.uuid, to_daemon).await;
        }
        NipartPluginEvent::QueryLogLevel => {
            handle_query_log_level::<T>(event.uuid, to_daemon).await;
        }
        _ => {
            if let Err(e) = T::handle_event(plugin, to_daemon, event).await {
                log::error!("{e}");
            }
        }
    }
}
