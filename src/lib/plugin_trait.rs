// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;
use std::sync::Arc;

use crate::{
    NipartConnection, NipartConnectionListener, NipartError, NipartEvent,
    NipartEventAction, NipartEventAddress, NipartLogLevel, NipartPluginEvent,
    NipartPluginInfo, NipartRole, NipartUserEvent,
};

const DEFAULT_PLUGIN_SOCKET_PREFIX: &str = "nipart_plugin_";
const DEFAULT_LOG_LEVEL: log::LevelFilter = log::LevelFilter::Trace;
const DEFAULT_QUIT_FLAG_CHECK_INTERVAL: u64 = 1000;

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct NipartPluginRunner {
    pub quit_flag: std::sync::atomic::AtomicBool,
}

pub trait NipartPlugin: Sized + Send + Sync + 'static {
    const PLUGIN_NAME: &'static str;

    fn name(&self) -> &'static str {
        Self::PLUGIN_NAME
    }

    fn roles(&self) -> Vec<NipartRole>;

    fn get_socket_path(&self) -> &str;

    /// Please store the socket_path in your struct, no need to bind the socket
    /// the run() will bind the socket.
    async fn init(socket_path: &str) -> Result<Self, NipartError>;

    /// Please override this function if you need special action for graceful
    /// quit.
    async fn quit(&self) {}

    fn get_plugin_info(&self) -> NipartPluginInfo {
        NipartPluginInfo {
            socket_path: self.get_socket_path().to_string(),
            name: self.name().to_string(),
            roles: self.roles(),
        }
    }

    async fn run() -> Result<(), NipartError> {
        let mut log_builder = env_logger::Builder::new();
        let (socket_path, log_level) = get_conf_from_argv(Self::PLUGIN_NAME);
        log_builder.filter(Some("nipart"), log_level);
        log_builder.filter(Some(Self::PLUGIN_NAME), log_level);
        log_builder.init();

        let listener = NipartConnectionListener::new_abstract(&socket_path)?;
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

    fn handle_event(
        plugin: Arc<Self>,
        connection: &mut NipartConnection,
        event: NipartEvent,
    ) -> impl std::future::Future<Output = Result<Vec<NipartEvent>, NipartError>>
           + Send;

    fn handle_connection(
        runner: Arc<NipartPluginRunner>,
        plugin: Arc<Self>,
        mut np_conn: NipartConnection,
    ) -> impl std::future::Future<Output = ()> + Send {
        async move {
            loop {
                let event: NipartEvent = match np_conn.recv().await {
                    Ok(e) => e,
                    Err(e) => {
                        log::error!(
                            "Nipart plugin {} failed to receive \
                            socket connection: {e}",
                            Self::PLUGIN_NAME
                        );
                        return;
                    }
                };

                match event.plugin {
                    NipartPluginEvent::Quit => {
                        runner
                            .quit_flag
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    NipartPluginEvent::QueryPluginInfo => {
                        handle_query_plugin_info(
                            &event,
                            &mut np_conn,
                            plugin.clone(),
                        )
                        .await
                    }
                    NipartPluginEvent::ChangeLogLevel(l) => {
                        handle_change_log_level(
                            l,
                            event.uuid,
                            &mut np_conn,
                            plugin.clone(),
                        )
                        .await
                    }
                    NipartPluginEvent::QueryLogLevel => {
                        handle_query_log_level(
                            event.uuid,
                            &mut np_conn,
                            plugin.clone(),
                        )
                        .await
                    }
                    _ => match Self::handle_event(
                        plugin.clone(),
                        &mut np_conn,
                        event,
                    )
                    .await
                    {
                        Ok(events) => {
                            for event in events {
                                if let Err(e) = np_conn.send(&event).await {
                                    log::warn!(
                                        "Failed to send event to \
                                        daemon {event:?}: {e}"
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            log::error!(
                                "Nipart plugin {} failed to process \
                                socket connection: {e}",
                                Self::PLUGIN_NAME
                            )
                        }
                    },
                }
            }
        }
    }
}

fn get_conf_from_argv(name: &str) -> (String, log::LevelFilter) {
    let argv: Vec<String> = std::env::args().collect();
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
    (socket_path, log_level)
}

async fn handle_query_plugin_info<T>(
    event: &NipartEvent,
    np_conn: &mut NipartConnection,
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
    );
    reply.ref_uuid = Some(event.uuid);
    if let Err(e) = np_conn.send(&reply).await {
        log::warn!("Failed to send event {reply:?}: {e}");
    }
}

async fn handle_change_log_level<T>(
    log_level: NipartLogLevel,
    ref_uuid: u128,
    np_conn: &mut NipartConnection,
    _plugin: Arc<T>,
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
    );
    reply.ref_uuid = Some(ref_uuid);
    if let Err(e) = np_conn.send(&reply).await {
        log::warn!("Failed to send event {reply:?}: {e}");
    }
}

async fn handle_query_log_level<T>(
    ref_uuid: u128,
    np_conn: &mut NipartConnection,
    _plugin: Arc<T>,
) where
    T: NipartPlugin,
{
    log::debug!("Querying log level of {}", T::PLUGIN_NAME);
    let mut reply = NipartEvent::new(
        NipartEventAction::Done,
        NipartUserEvent::None,
        NipartPluginEvent::QueryLogLevelReply(log::max_level().into()),
        NipartEventAddress::Unicast(T::PLUGIN_NAME.to_string()),
        NipartEventAddress::Commander,
    );
    reply.ref_uuid = Some(ref_uuid);
    if let Err(e) = np_conn.send(&reply).await {
        log::warn!("Failed to send event {reply:?}: {e}");
    }
}
