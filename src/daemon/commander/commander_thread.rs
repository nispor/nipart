// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use nipart::{
    NipartError, NipartEvent, NipartEventAction, NipartEventAddress,
    NipartLogLevel, NipartPluginEvent, NipartRole, NipartUserEvent,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::MPSC_CHANNLE_SIZE;

use super::session_queue::{CommanderSession, CommanderSessionQueue};

// Check the session queue every 5 seconds
const SESSION_QUEUE_CHECK_INTERVAL: u64 = 5000;
const DEFAULT_TIMEOUT: u64 = 1000;

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
    commander_to_daemon: Sender<NipartEvent>,
) {
    let mut plugin_infos: HashMap<String, Vec<NipartRole>> = HashMap::new();

    let mut session_queue = CommanderSessionQueue::new();

    let mut session_queue_check_interval = tokio::time::interval(
        std::time::Duration::from_millis(SESSION_QUEUE_CHECK_INTERVAL),
    );

    // The first tick just completes instantly
    session_queue_check_interval.tick().await;

    loop {
        tokio::select! {
            _ = session_queue_check_interval.tick()  => {
                if let Err(e) = process_session_queue(
                    &mut session_queue,
                    &mut commander_to_switch,
                    &mut plugin_infos).await {
                    log::error!("{e}");
                }
            }
            Some(event) = daemon_to_commander.recv() => {
                log::trace!("daemon_to_commander {event:?}");
                match event.plugin {
                    NipartPluginEvent::CommanderRefreshPlugins(i) => {
                        plugin_infos.clear();
                        if let Err(e) = handle_refresh_plugin_infos(
                            &mut commander_to_switch,
                            &mut session_queue,
                            i).await {
                            log::error!("{e}");
                        }
                    }
                    _ => {
                        log::error!(
                            "commander_thread(): Unexpected \
                            daemon_to_commander {event:?}");
                    }
                }
            }
            // Here we might get notification event from plugin on
            // changes like DHCP lease change) or reply from previous
            // requests.
            Some(event) = plugin_to_commander.recv() => {
                // Notification event from plugin
                if event.ref_uuid.is_none() {
                    log::error!("TODO: handle notification event \
                                from plugin {event:?}");
                } else {
                    if let Err(e) = session_queue.push(event) {
                        log::error!("{e}");
                        continue;
                    }
                    if let Err(e) = process_session_queue(
                        &mut session_queue,
                        &mut commander_to_switch,
                        &mut plugin_infos).await {
                        log::error!("{e}");
                    }
                }
            }
            Some(event) = user_to_commander.recv() => {
                match event.user {
                    NipartUserEvent::QueryPluginInfo => {
                        if let Err(e) = handle_plugin_info_query(
                            &mut commander_to_switch,
                            &mut session_queue,
                            &event,
                            plugin_infos.len()).await {
                            log::error!("{e}");
                        }
                    }
                    NipartUserEvent::QueryLogLevel => {
                        if let Err(e) = handle_query_log_level(
                            &mut commander_to_switch,
                            &mut session_queue,
                            event.uuid,
                            plugin_infos.len()
                            ).await {
                            log::error!("{e}");
                        }
                    }
                    NipartUserEvent::ChangeLogLevel(l) => {
                        if let Err(e) = handle_change_log_level(
                            &mut commander_to_switch,
                            &mut session_queue,
                            l,
                            event.uuid,
                            plugin_infos.len()).await {
                            log::error!("{e}");
                        }
                    }
                    NipartUserEvent::Quit => {
                        if let Err(e) = commander_to_daemon.send(event).await {
                            log::error!("{e}");
                        }
                        break;
                    }
                    _ => {
                        log::error!("Unknown event {event:?}");
                    }
                }
            }
        }
    }
}

async fn handle_plugin_info_query(
    commander_to_switch: &mut Sender<NipartEvent>,
    session_queue: &mut CommanderSessionQueue,
    event: &NipartEvent,
    plugin_count: usize,
) -> Result<(), NipartError> {
    log::debug!("Sending QueryPluginInfo to {plugin_count} plugins");
    let mut request = NipartEvent::new(
        NipartEventAction::Request,
        event.user.clone(),
        NipartPluginEvent::QueryPluginInfo,
        NipartEventAddress::Commander,
        NipartEventAddress::AllPlugins,
    );
    request.uuid = event.uuid;
    session_queue.new_session(
        request.uuid,
        request.clone(),
        plugin_count,
        DEFAULT_TIMEOUT,
    );
    commander_to_switch.send(request.clone()).await?;
    Ok(())
}

async fn handle_query_log_level(
    commander_to_switch: &mut Sender<NipartEvent>,
    session_queue: &mut CommanderSessionQueue,
    ref_uuid: u128,
    plugin_count: usize,
) -> Result<(), NipartError> {
    log::debug!("Sending PluginQueryLogLevel to {plugin_count} plugins");
    let mut request = NipartEvent::new(
        NipartEventAction::Request,
        NipartUserEvent::QueryLogLevel,
        NipartPluginEvent::QueryLogLevel,
        NipartEventAddress::Commander,
        NipartEventAddress::AllPlugins,
    );
    request.uuid = ref_uuid;
    session_queue.new_session(
        request.uuid,
        request.clone(),
        plugin_count,
        DEFAULT_TIMEOUT,
    );
    commander_to_switch.send(request.clone()).await?;
    Ok(())
}

async fn handle_change_log_level(
    commander_to_switch: &mut Sender<NipartEvent>,
    session_queue: &mut CommanderSessionQueue,
    log_level: NipartLogLevel,
    ref_uuid: u128,
    plugin_count: usize,
) -> Result<(), NipartError> {
    log::set_max_level(log_level.into());

    log::debug!("Sending PluginChangeLogLevel to {plugin_count} plugins");
    let mut request = NipartEvent::new(
        NipartEventAction::Request,
        NipartUserEvent::ChangeLogLevel(log_level),
        NipartPluginEvent::ChangeLogLevel(log_level),
        NipartEventAddress::Commander,
        NipartEventAddress::AllPlugins,
    );
    request.uuid = ref_uuid;
    session_queue.new_session(
        request.uuid,
        request.clone(),
        plugin_count,
        DEFAULT_TIMEOUT,
    );
    commander_to_switch.send(request.clone()).await?;
    Ok(())
}

async fn handle_refresh_plugin_infos(
    commander_to_switch: &mut Sender<NipartEvent>,
    session_queue: &mut CommanderSessionQueue,
    plugin_count: usize,
) -> Result<(), NipartError> {
    let request = NipartEvent::new(
        NipartEventAction::Request,
        NipartUserEvent::None,
        NipartPluginEvent::QueryPluginInfo,
        NipartEventAddress::Commander,
        NipartEventAddress::AllPlugins,
    );
    session_queue.new_session(
        request.uuid,
        request.clone(),
        plugin_count,
        DEFAULT_TIMEOUT,
    );
    log::trace!("commander_to_switch {request:?}");
    commander_to_switch.send(request.clone()).await?;
    Ok(())
}

fn process_session(
    session: CommanderSession,
    plugin_infos: &mut HashMap<String, Vec<NipartRole>>,
) -> Result<Option<NipartEvent>, NipartError> {
    match session.request.plugin {
        NipartPluginEvent::QueryPluginInfo => {
            // Commander want to refresh its own knowledge of plugins
            if session.request.user == NipartUserEvent::None {
                plugin_infos.clear();
                for reply in session.replies {
                    if let NipartPluginEvent::QueryPluginInfoReply(i) =
                        reply.plugin
                    {
                        log::debug!(
                            "Commander is aware of plugin {} with roles {:?}",
                            i.name,
                            i.roles
                        );
                        plugin_infos.insert(i.name, i.roles);
                    }
                }
                Ok(None)
            } else {
                let mut plugin_infos = Vec::new();
                for reply in &session.replies {
                    if let NipartPluginEvent::QueryPluginInfoReply(i) =
                        &reply.plugin
                    {
                        plugin_infos.push(i.clone());
                    }
                }
                let mut reply_event = NipartEvent::new(
                    NipartEventAction::Done,
                    NipartUserEvent::QueryPluginInfoReply(plugin_infos),
                    NipartPluginEvent::None,
                    NipartEventAddress::Daemon,
                    NipartEventAddress::User,
                );
                reply_event.ref_uuid = Some(session.request.uuid);
                Ok(Some(reply_event))
            }
        }
        NipartPluginEvent::QueryLogLevel
        | NipartPluginEvent::ChangeLogLevel(_) => {
            let mut log_levels = HashMap::new();
            for reply in &session.replies {
                if let NipartPluginEvent::QueryLogLevelReply(l) = &reply.plugin
                {
                    log_levels.insert(reply.src.to_string(), *l);
                }
            }
            log_levels.insert("daemon".to_string(), log::max_level().into());
            let mut reply_event = NipartEvent::new(
                NipartEventAction::Done,
                NipartUserEvent::QueryLogLevelReply(log_levels),
                NipartPluginEvent::None,
                NipartEventAddress::Daemon,
                NipartEventAddress::User,
            );
            reply_event.ref_uuid = Some(session.request.uuid);
            Ok(Some(reply_event))
        }
        _ => Ok(None),
    }
}

async fn process_session_queue(
    session_queue: &mut CommanderSessionQueue,
    commander_to_switch: &mut Sender<NipartEvent>,
    plugin_infos: &mut HashMap<String, Vec<NipartRole>>,
) -> Result<(), NipartError> {
    for session in session_queue.get_timeout_or_finished()? {
        if let Some(event) = process_session(session, plugin_infos)? {
            if let Err(_e) = commander_to_switch.send(event.clone()).await {
                log::error!("Failed to send event to switch {event:?}");
                break;
            }
        }
    }
    Ok(())
}
