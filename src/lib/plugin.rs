// SPDX-License-Identifier: Apache-2.0

use std::fs::remove_file;

use tokio::net::{UnixListener, UnixStream};

use crate::{
    ErrorKind, NipartConnection, NipartConnectionListener, NipartError,
    NipartEvent,
};

const PLUGIN_SOCKET_PREFIX: &str = "nipart_plugin_";

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct NipartPluginConfig {
    pub log_level: log::Level,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum NipartPluginCapacity {
    Commander,
    Dhcp,
    Kernel,
    Ovs,
    Lldp,
    Monitor,
    State,
    Config,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
struct NipartPluginRunner {
    quit_flag: std::sync::atomic::AtomicBool,
}

pub trait NipartPlugin {
    const PLUGIN_NAME: &'static str;
    fn capacity(&self) -> Vec<NipartPluginCapacity>;

    async fn init(config: NipartPluginConfig) -> Self;

    async fn get_quit_flag(&self) -> bool;
    async fn set_quit_flag(&self, quit: bool);

    async fn run() -> Result<(), NipartError> {
        let mut log_builder = env_logger::Builder::new();
        log_builder.filter(Some("nipart"), config.log_level.into());
        log_builder.filter(Some(Self::PLUGIN_NAME), config.log_level.into());
        log_builder.init();

        let socket_path =
            format!("{PLUGIN_SOCKET_PREFIX}{}", Self::PLUGIN_NAME);
        let listener = NipartConnectionListener::new_abstract(socket_path)?;
        log::debug!(
            "Nipart plugin {} is listening on {}",
            Self::PLUGIN_NAME,
            socket_path
        );
        let runner = Arc::new(NipartPluginRunner::default());
        let plugin = Arc::new(Self::init()

        loop {
            if runner.quit_flag {
                log::info!("Nipart plugin {} got quit signal, quitting");
                break;
            }
            match listener.accept().await {
                Ok(connection) => {
                    // TODO: Limit the maximum connected client as it could
                    //       from suspicious source, not daemon
                    let runner_clone = runner.clone();
                    let socket_path_ref = socket_path.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = handle_plugin_client(
                            runner_clone,
                            socket_path_ref,
                            stream,
                        ) {
                            log::error!(
                                "Nipart plugin {} failed to process \
                                socket connection: {e}",
                                Self::PLUGIN_NAME
                            );
                        }
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

    async fn handle_plugin_client(
        runner: Arc<NipartPluginRunner>,
        plugin: Arc<T>,
        mut connection: NipartConnection,
    ) -> Result<(), NipartError>
    where
        T: NipartPlugin + Sized + std::marker::Send + std::marker::Sync,
    {
        let event: NipartEvent = connection::recv()?;
        if event.data
            == NipartEventData::PluginCommon(NipartPluginCommonEvent::Quit)
        {
            runner
                .quit_flag
                .store(true, std::sync::atomic::Ordering::Relaxed);
            return Ok(());
        }
        handle_event
    }

    async fn handle_event(
        plugin: Arc<T>,
        connection: &mut NipartConnection,
        event: NipartEvent,
    ) -> Result<(), NipartEvent>
    where
        T: NipartPlugin + Sized + std::marker::Send + std::marker::Sync,
    {
    }
}
