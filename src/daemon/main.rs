// SPDX-License-Identifier: Apache-2.0

mod api;
mod apply;
mod commander;
mod conf;
mod daemon;
mod dhcp;
mod event;
mod lock;
mod logger;
mod monitor;
mod plugin;
mod query;
mod task;
mod udev;

pub(crate) use self::{
    logger::{log_debug, log_error, log_info, log_trace, log_warn},
    task::{TaskManager, TaskWorker},
};

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<(), nipart::NipartError> {
    enable_logging();

    // According to https://github.com/tokio-rs/tokio/discussions/7091
    // We should not use the main thread for heavy lifting.
    let handle = tokio::spawn(async move {
        match self::daemon::NipartDaemon::new().await {
            Ok(mut daemon) => daemon.run().await,
            Err(e) => log::error!("Failed to start daemon {e}"),
        };
    });

    handle.await.map_err(|e| {
        nipart::NipartError::new(nipart::ErrorKind::Bug, format!("{e}"))
    })
}

fn enable_logging() {
    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(Some("nm"), log::LevelFilter::Trace);
    log_builder.filter(Some("nipart"), log::LevelFilter::Trace);
    log_builder.init();
}
