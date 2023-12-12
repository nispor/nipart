// SPDX-License-Identifier: Apache-2.0

mod api_listener;
mod plugin;
mod switch;

use nipart::{
    ErrorKind, NipartConnection, NipartConnectionListener, NipartError,
    NipartEvent, NipartEventAddress, NipartRole,
};
use tokio::net::UnixListener;
use tokio::sync::mpsc::{Receiver, Sender};

use self::api_listener::start_api_listener;
use self::plugin::load_plugins;
use self::switch::start_event_switch;

pub(crate) const MPSC_CHANNLE_SIZE: usize = 64;

#[tokio::main(flavor = "multi_thread", worker_threads = 50)]
async fn main() -> Result<(), NipartError> {
    init_logger();

    // We don't plan to unload plugin during runtime when plugin is slow or bad.
    // To support that, we need a mutex protected Vec which is complex.
    // We assume the plugin can be trusted.
    let plugins = load_plugins();

    let (mut from_api, to_api) = start_api_listener().await?;
    start_event_switch(plugins, from_api, to_api).await;
    Ok(())
}

fn init_logger() {
    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(Some("nipart"), log::LevelFilter::Debug);
    log_builder.init();
}
