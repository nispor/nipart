// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use nipart::{
    ErrorKind, NipartError, NipartEvent, NipartPluginEvent, NipartUserEvent,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{Plugins, WorkFlow, WorkFlowQueue, MPSC_CHANNLE_SIZE};

// Check the session queue every 5 seconds
const WORKFLOW_QUEUE_CHECK_INTERVAL: u64 = 5000;

pub(crate) async fn start_commander_thread() -> Result<
    (
        Receiver<NipartEvent>,
        Sender<NipartEvent>,
        Sender<NipartEvent>,
        Sender<NipartEvent>,
    ),
    NipartError,
> {
    let (commander_to_switch_tx, commander_to_switch_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (plugin_to_commander_tx, plugin_to_commander_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (user_to_commander_tx, user_to_commander_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (daemon_to_commander_tx, daemon_to_commander_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);

    tokio::spawn(async move {
        commander_thread(
            commander_to_switch_tx,
            plugin_to_commander_rx,
            user_to_commander_rx,
            daemon_to_commander_rx,
        )
        .await;
    });
    log::debug!("Commander started");

    Ok((
        commander_to_switch_rx,
        plugin_to_commander_tx,
        user_to_commander_tx,
        daemon_to_commander_tx,
    ))
}

async fn commander_thread(
    mut commander_to_switch: Sender<NipartEvent>,
    mut plugin_to_commander: Receiver<NipartEvent>,
    mut user_to_commander: Receiver<NipartEvent>,
    mut daemon_to_commander: Receiver<NipartEvent>,
) {
    let plugins = Arc::new(Mutex::new(Plugins::new()));

    let mut workflow_queue = WorkFlowQueue::new();

    let mut workflow_queue_check_interval = tokio::time::interval(
        std::time::Duration::from_millis(WORKFLOW_QUEUE_CHECK_INTERVAL),
    );

    // The first tick just completes instantly
    workflow_queue_check_interval.tick().await;

    loop {
        if let Err(e) = tokio::select! {
            _ = workflow_queue_check_interval.tick() => {
                process_workflow_queue(
                    &mut workflow_queue, &mut commander_to_switch).await
            }
            Some(event) = daemon_to_commander.recv() => {
                log::debug!("daemon_to_commander {event}");
                log::trace!("daemon_to_commander {event:?}");
                handle_daemon_commands(event,
                    &mut workflow_queue,
                    &mut commander_to_switch,
                    plugins.clone()).await
            }
            Some(event) = plugin_to_commander.recv() => {
                log::debug!("plugin_to_commander {event}");
                log::trace!("plugin_to_commander {event:?}");
                workflow_queue.add_reply(event);
                process_workflow_queue(
                    &mut workflow_queue, &mut commander_to_switch).await
            }
            Some(event) = user_to_commander.recv() => {
                log::debug!("user_to_commander {event}");
                log::trace!("user_to_commander {event:?}");
                handle_user_request(
                    plugins.clone(),
                    event,
                    &mut workflow_queue,
                    &mut commander_to_switch).await
            }
        } {
            log::error!("{e}");
        }
    }
}

async fn process_workflow_queue(
    workflow_queue: &mut WorkFlowQueue,
    commander_to_switch: &mut Sender<NipartEvent>,
) -> Result<(), NipartError> {
    for event in workflow_queue.process()? {
        log::debug!("Sent to switch {event}");
        log::trace!("Sent to switch {event:?}");
        if let Err(e) = commander_to_switch.send(event).await {
            log::error!("{e}");
        }
    }
    Ok(())
}

async fn handle_daemon_commands(
    event: NipartEvent,
    workflow_queue: &mut WorkFlowQueue,
    commander_to_switch: &mut Sender<NipartEvent>,
    plugins: Arc<Mutex<Plugins>>,
) -> Result<(), NipartError> {
    match event.plugin {
        NipartPluginEvent::CommanderRefreshPlugins(i) => {
            let (workflow, share_data) =
                WorkFlow::new_refresh_plugins(plugins, event.uuid, i);
            workflow_queue.add_workflow(workflow, share_data);
            process_workflow_queue(workflow_queue, commander_to_switch).await
        }
        _ => {
            log::error!(
                "commander_thread(): Unexpected daemon_to_commander {event:?}"
            );
            Ok(())
        }
    }
}

async fn handle_user_request(
    plugins: Arc<Mutex<Plugins>>,
    event: NipartEvent,
    workflow_queue: &mut WorkFlowQueue,
    commander_to_switch: &mut Sender<NipartEvent>,
) -> Result<(), NipartError> {
    let all_plugins_count = match plugins.lock() {
        Ok(p) => p.len(),
        Err(e) => {
            return Err(NipartError::new(
                ErrorKind::Bug,
                format!("Failed to lock on Plugins {e}"),
            ));
        }
    };
    let (workflow, share_data) = match event.user {
        NipartUserEvent::QueryPluginInfo => {
            WorkFlow::new_query_plugin_info(event.uuid, all_plugins_count)
        }
        NipartUserEvent::QueryLogLevel => {
            WorkFlow::new_query_log_level(event.uuid, all_plugins_count)
        }
        NipartUserEvent::ChangeLogLevel(l) => {
            WorkFlow::new_change_log_level(l, event.uuid, all_plugins_count)
        }
        NipartUserEvent::Quit => {
            WorkFlow::new_quit(event.uuid, all_plugins_count)
        }
        NipartUserEvent::QueryNetState(opt) => {
            WorkFlow::new_query_net_state(opt, event.uuid, plugins)
        }
        NipartUserEvent::ApplyNetState(des, opt) => {
            WorkFlow::new_apply_net_state(*des, opt, event.uuid, plugins)
        }
        _ => {
            log::error!("Unknown event {event:?}");
            return Ok(());
        }
    };
    workflow_queue.add_workflow(workflow, share_data);

    process_workflow_queue(workflow_queue, commander_to_switch).await
}
