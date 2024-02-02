// SPDX-License-Identifier: Apache-2.0

use futures::{stream::FuturesUnordered, StreamExt};
use std::collections::HashMap;

use nipart::{
    ErrorKind, NipartConnection, NipartError, NipartEvent, NipartEventAction,
    NipartEventAddress, NipartPluginEvent, NipartPluginInfo, NipartRole,
    NipartUserEvent,
};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::time::DelayQueue;

const QUERY_PLUGIN_RETRY: usize = 5;
const QUERY_PLUGIN_RETRY_INTERAL: u64 = 500; // milliseconds

pub(crate) async fn start_event_switch_thread(
    plugins: &[(String, String)],
    user_to_switch: Receiver<NipartEvent>,
    switch_to_user: Sender<NipartEvent>,
    commander_to_switch: Receiver<NipartEvent>,
    plugin_to_commander: Sender<NipartEvent>,
    user_to_commander: Sender<NipartEvent>,
    commander_to_daemon: Sender<NipartEvent>,
) {
    let mut plugin_infos: Vec<(NipartConnection, NipartPluginInfo)> =
        Vec::new();

    for (plugin_name, plugin_socket) in plugins {
        // TODO: The plugin might not ready yet on slow system, we need retry
        // here
        match connect_plugin(plugin_name, plugin_socket).await {
            Ok(i) => plugin_infos.push(i),
            Err(e) => {
                log::error!(
                    "Failed to reach plugin \
                    {plugin_name} via {plugin_socket}: {e}"
                );
            }
        }
    }

    tokio::spawn(async move {
        run_event_switch(
            user_to_switch,
            switch_to_user,
            commander_to_switch,
            plugin_to_commander,
            user_to_commander,
            commander_to_daemon,
            plugin_infos,
        )
        .await;
    });
    log::debug!("switch started");
}

async fn connect_plugin(
    plugin_name: &str,
    plugin_socket: &str,
) -> Result<(NipartConnection, NipartPluginInfo), NipartError> {
    let mut cur_count = 0usize;
    while cur_count < QUERY_PLUGIN_RETRY {
        let result = get_plugin_info(plugin_name, plugin_socket).await;
        match result {
            Ok(i) => return Ok(i),
            Err(e) => {
                if cur_count == QUERY_PLUGIN_RETRY - 1 {
                    return Err(e);
                }
                std::thread::sleep(std::time::Duration::from_millis(
                    QUERY_PLUGIN_RETRY_INTERAL,
                ));
                cur_count += 1;
                continue;
            }
        }
    }
    Err(NipartError::new(
        ErrorKind::Bug,
        "BUG: connect_plugin() unreachable".to_string(),
    ))
}

async fn run_event_switch(
    mut user_to_switch: Receiver<NipartEvent>,
    switch_to_user: Sender<NipartEvent>,
    mut commander_to_switch: Receiver<NipartEvent>,
    plugin_to_commander: Sender<NipartEvent>,
    user_to_commander: Sender<NipartEvent>,
    commander_to_daemon: Sender<NipartEvent>,
    plugins: Vec<(NipartConnection, NipartPluginInfo)>,
) {
    let mut np_conn_map: HashMap<String, NipartConnection> = HashMap::new();
    let mut role_map: HashMap<NipartRole, Vec<String>> = HashMap::new();
    let mut plugin_info_map: HashMap<String, NipartPluginInfo> = HashMap::new();
    let mut postponed_events: DelayQueue<NipartEvent> = DelayQueue::new();
    for (np_conn, plugin_info) in plugins {
        np_conn_map.insert(plugin_info.name.to_string(), np_conn);
        for role in plugin_info.roles.as_slice() {
            role_map
                .entry(*role)
                .or_default()
                .push(plugin_info.name.to_string());
        }
        plugin_info_map.insert(plugin_info.name.to_string(), plugin_info);
    }

    loop {
        let mut plugin_futures = FuturesUnordered::new();
        for np_conn in np_conn_map.values_mut() {
            plugin_futures.push(np_conn.recv());
        }

        let event = tokio::select! {
            Some(Ok(event)) = plugin_futures.next() => {
                log::trace!("run_event_switch(): from plugin {event:?}");
                event
            },
            Some(event) = user_to_switch.recv() => {
                log::trace!("run_event_switch(): from daemon {event:?}");
                event
            }
            Some(event) = commander_to_switch.recv() => {
                log::trace!("run_event_switch(): from commander {event:?}");
                event
            }
            Some(event) = postponed_events.next() => {
                let mut event = event.into_inner();
                log::trace!("postponed event ready to process {event:?}");
                event.postpone_millis = 0;
                event
            }
        };
        drop(plugin_futures);

        if event.postpone_millis > 0 {
            let t = event.postpone_millis;
            postponed_events
                .insert(event, std::time::Duration::from_millis(t.into()));
            continue;
        }

        // Discard dead-loop
        if event.src == event.dst {
            log::warn!(
                "Discarding event which holds the same \
                src and dst: {event:?}"
            );
            continue;
        }
        match &event.dst {
            NipartEventAddress::User => {
                if let Err(e) = switch_to_user.send(event.clone()).await {
                    log::warn!("Failed to send event: {event:?}, {e}");
                    continue;
                }
            }
            NipartEventAddress::Daemon => {
                if let Err(e) = commander_to_daemon.send(event.clone()).await {
                    log::warn!("Failed to send event: {event:?}, {e}");
                    continue;
                }
            }
            NipartEventAddress::Commander => {
                if event.src == NipartEventAddress::User {
                    if let Err(e) = user_to_commander.send(event.clone()).await
                    {
                        log::warn!("Failed to send event: {event:?}, {e}");
                        continue;
                    }
                } else if let Err(e) =
                    plugin_to_commander.send(event.clone()).await
                {
                    log::warn!("Failed to send event: {event:?}, {e}");
                    continue;
                }
            }
            NipartEventAddress::Unicast(plugin_name) => {
                if let Some(np_conn) = np_conn_map.get_mut(plugin_name.as_str())
                {
                    if let Err(e) = np_conn.send(&event.clone()).await {
                        log::warn!(
                            "Failed to send event {event:?} to \
                             plugin {plugin_name}: {e}",
                        );
                    }
                }
            }
            NipartEventAddress::Group(role) => {
                if let Some(plugin_names) = role_map.get(&role) {
                    for plugin_name in plugin_names {
                        if let Some(np_conn) =
                            np_conn_map.get_mut(plugin_name.as_str())
                        {
                            if let Err(e) = np_conn.send(&event).await {
                                log::warn!(
                                    "Failed to send event {event:?} to \
                                     plugin {plugin_name}: {e}",
                                );
                            }
                        }
                    }
                }
            }
            NipartEventAddress::AllPlugins => {
                for (role, plugin_names) in role_map.iter() {
                    if role == &NipartRole::Commander {
                        continue;
                    }
                    for plugin_name in plugin_names {
                        if let Some(np_conn) =
                            np_conn_map.get_mut(plugin_name.as_str())
                        {
                            log::trace!(
                                "run_event_switch(): to plugin \
                                {plugin_name}, {event:?}"
                            );
                            if let Err(e) = np_conn.send(&event.clone()).await {
                                log::warn!(
                                    "Failed to send event {event:?} to \
                                     plugin {plugin_name}: {e}",
                                );
                            }
                        }
                    }
                }
            }
            _ => {
                log::error!("BUG: Unknown dst of event {event:?}");
            }
        }
    }
}

async fn get_plugin_info(
    plugin_name: &str,
    plugin_socket: &str,
) -> Result<(NipartConnection, NipartPluginInfo), NipartError> {
    let event = NipartEvent::new(
        NipartEventAction::Request,
        NipartUserEvent::None,
        NipartPluginEvent::QueryPluginInfo,
        NipartEventAddress::Daemon,
        NipartEventAddress::Unicast(plugin_name.to_string()),
    );
    let mut np_conn = NipartConnection::new_abstract(plugin_socket)?;
    np_conn.send(&event).await?;
    let reply: NipartEvent = np_conn.recv().await?;
    if let NipartPluginEvent::QueryPluginInfoReply(i) = reply.plugin {
        log::debug!("Got plugin info {i:?}");
        Ok((np_conn, i))
    } else {
        Err(NipartError::new(
            ErrorKind::Bug,
            format!("invalid reply {event:?}"),
        ))
    }
}
