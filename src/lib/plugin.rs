// SPDX-License-Identifier: Apache-2.0

use std::fs::remove_file;
use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::net::{UnixListener, UnixStream};

use crate::{
    ErrorKind, NipartConnection, NipartConnectionListener, NipartError,
    NipartEvent, NipartEventAction, NipartEventAddress, NipartEventData,
};

const DEFAULT_PLUGIN_SOCKET_PREFIX: &str = "nipart_plugin_";
// const DEFAULT_LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;
const DEFAULT_LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NipartPluginInfo {
    pub socket_path: String,
    pub name: String,
    pub roles: Vec<NipartRole>,
}

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
#[non_exhaustive]
pub enum NipartRole {
    Daemon,
    Commander,
    Dhcp,
    Kernel,
    Ovs,
    Lldp,
    Monitor,
    State,
    Config,
}

#[derive(Debug, Default)]
#[non_exhaustive]
struct NipartPluginRunner {
    quit_flag: std::sync::atomic::AtomicBool,
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
                return Ok(());
            }
            match listener.accept().await {
                Ok(np_conn) => {
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
                Err(e) => {
                    log::error!(
                        "Nipart plugin {} failed to accept connection {}",
                        Self::PLUGIN_NAME,
                        e
                    );
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

                match event.data {
                    NipartEventData::PluginQuit => {
                        runner
                            .quit_flag
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    NipartEventData::QueryPluginInfo => {
                        handle_query_plugin_info(
                            &event,
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
    let mut reply = NipartEvent::new(
        NipartEventAction::Done,
        NipartEventData::QueryPluginInfoReply(plugin.get_plugin_info()),
        NipartEventAddress::Unicast(T::PLUGIN_NAME.to_string()),
        event.src.clone(),
    );
    reply.ref_uuid = Some(event.uuid);
    if let Err(e) = np_conn.send(&reply).await {
        log::warn!("Failed to send event to daemon {event:?}: {e}");
    }
}
