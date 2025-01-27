// SPDX-License-Identifier: Apache-2.0

mod api_listener;
mod commander;
mod plugin;
mod switch;

pub(crate) use self::plugin::{PluginRoles, Plugins};

use nipart::{NipartError, DEFAULT_TIMEOUT};

use self::api_listener::start_api_listener_thread;
use self::commander::start_commander_thread;
use self::switch::start_event_switch_thread;

pub(crate) const DEFAULT_LOG_LEVEL: log::LevelFilter = log::LevelFilter::Debug;
pub(crate) const MPSC_CHANNLE_SIZE: usize = 64;

#[tokio::main(flavor = "multi_thread", worker_threads = 50)]
async fn main() -> Result<(), NipartError> {
    init_logger();

    // TODO: Find a way to refresh plugins in switch
    let plugins = Plugins::start().await?;

    let (api_to_switch_tx, api_to_switch_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (switch_to_api_tx, switch_to_api_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (commander_to_switch_tx, commander_to_switch_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (switch_to_commander_tx, switch_to_commander_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);

    let api_thread =
        start_api_listener_thread(switch_to_api_rx, api_to_switch_tx).await?;

    let plugin_roles = start_event_switch_thread(
        plugins,
        api_to_switch_rx,
        switch_to_api_tx,
        commander_to_switch_rx,
        switch_to_commander_tx,
    )
    .await?;

    start_commander_thread(
        commander_to_switch_tx,
        switch_to_commander_rx,
        plugin_roles,
    )
    .await?;

    api_thread.await.ok();
    Ok(())
}

fn init_logger() {
    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(Some("nipart"), DEFAULT_LOG_LEVEL);
    log_builder.init();
}
