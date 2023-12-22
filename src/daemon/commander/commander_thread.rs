// SPDX-License-Identifier: Apache-2.0

use nipart::{
    NipartError, NipartEvent, NipartEventAction, NipartEventAddress,
    NipartPluginEvent, NipartUserEvent,
};
use tokio::sync::mpsc::{Receiver, Sender};

use super::{
    handle_change_log_level, handle_query_log_level, handle_query_net_state,
    handle_query_plugin_infos, handle_refresh_plugin_infos,
    process_query_log_level, process_query_net_state_reply,
    process_query_plugin_info,
};
use crate::{Plugins, Session, SessionQueue, MPSC_CHANNLE_SIZE};

// Check the session queue every 5 seconds
const SESSION_QUEUE_CHECK_INTERVAL: u64 = 5000;

pub(crate) async fn start_commander_thread(
    commander_to_daemon: Sender<NipartEvent>,
) -> Result<
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
            commander_to_daemon,
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
    mut commander_to_daemon: Sender<NipartEvent>,
) {
    let mut plugins = Plugins::new();

    let mut session_queue = SessionQueue::new();

    let mut session_queue_check_interval = tokio::time::interval(
        std::time::Duration::from_millis(SESSION_QUEUE_CHECK_INTERVAL),
    );

    // The first tick just completes instantly
    session_queue_check_interval.tick().await;

    loop {
        if let Err(e) = tokio::select! {
            _ = session_queue_check_interval.tick() => {
                process_session_queue(
                    &mut session_queue,
                    &mut commander_to_switch,
                    &mut plugins,
                ).await
            }
            Some(event) = daemon_to_commander.recv() => {
                log::trace!("daemon_to_commander recv {event:?}");
                handle_daemon_commands(event,
                    &mut session_queue,
                    &mut commander_to_switch).await
            }
            Some(event) = plugin_to_commander.recv() => {
                log::trace!("plugin_to_commander recv {event:?}");
                handle_plugin_reply(
                    &mut plugins,
                    event,
                    &mut session_queue,
                    &mut commander_to_switch).await
            }
            Some(event) = user_to_commander.recv() => {
                log::trace!("user_to_commander recv {event:?}");
                handle_user_request(
                    &plugins,
                    event,
                    &mut session_queue,
                    &mut commander_to_daemon,
                    &mut commander_to_switch).await
            }
        } {
            log::error!("{e}");
        }
    }
}

async fn process_session(
    session_queue: &mut SessionQueue,
    session: Session,
    commander_to_switch: &mut Sender<NipartEvent>,
    plugins: &mut Plugins,
) -> Result<(), NipartError> {
    match session.request.plugin {
        NipartPluginEvent::QueryPluginInfo => {
            process_query_plugin_info(session, plugins, commander_to_switch)
                .await?;
        }
        NipartPluginEvent::QueryLogLevel
        | NipartPluginEvent::ChangeLogLevel(_) => {
            process_query_log_level(session, commander_to_switch).await?;
        }
        NipartPluginEvent::QueryNetState(_) => {
            process_query_net_state_reply(
                session_queue,
                session,
                commander_to_switch,
            )
            .await?;
        }
        _ => {
            log::error!("process_session(): Unexpected session {session:?}");
        }
    }
    Ok(())
}

async fn process_session_queue(
    session_queue: &mut SessionQueue,
    commander_to_switch: &mut Sender<NipartEvent>,
    plugins: &mut Plugins,
) -> Result<(), NipartError> {
    for session in session_queue.get_timeout_or_finished()? {
        if let Err(e) = process_session(
            session_queue,
            session,
            commander_to_switch,
            plugins,
        )
        .await
        {
            log::error!("{e}");
        }
    }
    Ok(())
}

async fn handle_daemon_commands(
    event: NipartEvent,
    session_queue: &mut SessionQueue,
    commander_to_switch: &mut Sender<NipartEvent>,
) -> Result<(), NipartError> {
    match event.plugin {
        NipartPluginEvent::CommanderRefreshPlugins(i) => {
            handle_refresh_plugin_infos(commander_to_switch, session_queue, i)
                .await
        }
        _ => {
            log::error!(
                "commander_thread(): Unexpected daemon_to_commander {event:?}"
            );
            Ok(())
        }
    }
}

async fn handle_plugin_reply(
    plugins: &mut Plugins,
    event: NipartEvent,
    session_queue: &mut SessionQueue,
    commander_to_switch: &mut Sender<NipartEvent>,
) -> Result<(), NipartError> {
    // Notification event from plugin
    if event.ref_uuid.is_none() {
        log::error!("TODO: handle notification event from plugin {event:?}");
        Ok(())
    } else {
        session_queue.push(event)?;
        process_session_queue(session_queue, commander_to_switch, plugins).await
    }
}

async fn handle_user_request(
    plugins: &Plugins,
    event: NipartEvent,
    session_queue: &mut SessionQueue,
    commander_to_daemon: &mut Sender<NipartEvent>,
    commander_to_switch: &mut Sender<NipartEvent>,
) -> Result<(), NipartError> {
    match event.user {
        NipartUserEvent::QueryPluginInfo => {
            handle_query_plugin_infos(
                commander_to_switch,
                session_queue,
                plugins.len(),
                event.uuid,
            )
            .await
        }
        NipartUserEvent::QueryLogLevel => {
            handle_query_log_level(
                commander_to_switch,
                session_queue,
                event.uuid,
                plugins.len(),
            )
            .await
        }
        NipartUserEvent::ChangeLogLevel(l) => {
            handle_change_log_level(
                commander_to_switch,
                session_queue,
                l,
                event.uuid,
                plugins.len(),
            )
            .await
        }
        NipartUserEvent::Quit => {
            handle_quit(commander_to_daemon, commander_to_switch).await
        }
        NipartUserEvent::QueryNetState(opt) => {
            handle_query_net_state(
                commander_to_switch,
                session_queue,
                opt,
                event.uuid,
                plugins,
            )
            .await
        }
        _ => {
            log::error!("Unknown event {event:?}");
            Ok(())
        }
    }
}

async fn handle_quit(
    commander_to_daemon: &mut Sender<NipartEvent>,
    commander_to_switch: &mut Sender<NipartEvent>,
) -> Result<(), NipartError> {
    let plugin_quit_event = NipartEvent::new(
        NipartEventAction::OneShot,
        NipartUserEvent::Quit,
        NipartPluginEvent::Quit,
        NipartEventAddress::Commander,
        NipartEventAddress::AllPlugins,
    );
    log::trace!("handle_quit(): sent to switch {plugin_quit_event:?}");
    if let Err(e) = commander_to_switch.send(plugin_quit_event).await {
        log::error!("{e}");
    }
    // Give switch some time to send out quit event to plugins.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let daemon_quit_event = NipartEvent::new(
        NipartEventAction::OneShot,
        NipartUserEvent::Quit,
        NipartPluginEvent::Quit,
        NipartEventAddress::Commander,
        NipartEventAddress::Daemon,
    );
    log::trace!("handle_quit(): sent to daemon {daemon_quit_event:?}");
    commander_to_daemon.send(daemon_quit_event).await.ok();
    // Wait a little bit for switch/daemon to process this information
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    std::process::exit(0)
}
