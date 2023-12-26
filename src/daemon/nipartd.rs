// SPDX-License-Identifier: Apache-2.0

mod api_listener;
mod commander;
mod plugin;
mod session_queue;
mod switch;

pub(crate) use self::plugin::Plugins;
pub(crate) use self::session_queue::{Session, SessionQueue};

use nipart::{
    NipartError, NipartEvent, NipartEventAction, NipartEventAddress,
    NipartPluginEvent, NipartUserEvent,
};

use self::api_listener::start_api_listener_thread;
use self::commander::start_commander_thread;
use self::plugin::load_plugins;
use self::switch::start_event_switch_thread;

const DEFAULT_LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;
pub(crate) const MPSC_CHANNLE_SIZE: usize = 64;
pub(crate) const DEFAULT_TIMEOUT: u64 = 1000; // 1 seconds

#[tokio::main(flavor = "multi_thread", worker_threads = 50)]
async fn main() -> Result<(), NipartError> {
    init_logger();

    // TODO: Find a way to refresh plugins in switch
    let plugins = load_plugins();
    let plugin_count = plugins.len();

    let (commander_to_daemon_tx, mut commander_to_daemon_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);

    let (user_to_switch, switch_to_user) = start_api_listener_thread().await?;
    let (
        commander_to_switch,
        plugin_to_commander,
        user_to_commander,
        daemon_to_commander,
    ) = start_commander_thread(commander_to_daemon_tx).await?;

    start_event_switch_thread(
        &plugins,
        user_to_switch,
        switch_to_user,
        commander_to_switch,
        plugin_to_commander,
        user_to_commander,
    )
    .await;

    // Inform commander that daemon ready, please refresh your knowledge of
    // plugins
    let event = NipartEvent::new(
        NipartEventAction::OneShot,
        NipartUserEvent::None,
        NipartPluginEvent::CommanderRefreshPlugins(plugin_count),
        NipartEventAddress::Daemon,
        NipartEventAddress::User,
    );

    daemon_to_commander.send(event).await?;

    loop {
        match commander_to_daemon_rx.recv().await {
            Some(event) => {
                if event.plugin == NipartPluginEvent::Quit {
                    log::info!("Stopping daemon as requested");
                    return Ok(());
                } else {
                    log::error!("Unexpected event received from {event:?}");
                }
            }
            None => {
                log::error!("Stopping daemon because commander stopped");
                return Ok(());
            }
        }
    }
}

fn init_logger() {
    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(Some("nipart"), DEFAULT_LOG_LEVEL);
    log_builder.init();
}
