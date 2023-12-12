// SPDX-License-Identifier: Apache-2.0

use futures::{stream::FuturesUnordered, StreamExt};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

use nipart::{
    ErrorKind, NipartConnection, NipartError, NipartEvent, NipartEventAction,
    NipartEventAddress, NipartEventData, NipartPluginInfo, NipartRole,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::MPSC_CHANNLE_SIZE;

const QUERY_PLUGIN_RETRY: usize = 5;
const QUERY_PLUGIN_RETRY_INTERAL: u64 = 500; // milliseconds

pub(crate) async fn start_event_switch(
    plugins: Vec<(String, String)>,
    from_api: Receiver<NipartEvent>,
    to_api: Sender<NipartEvent>,
) {
    let mut plugin_infos: Vec<(NipartConnection, NipartPluginInfo)> =
        Vec::new();

    for (plugin_name, plugin_socket) in plugins {
        match connect_plugin(&plugin_name, &plugin_socket).await {
            Ok(i) => plugin_infos.push(i),
            Err(e) => {
                log::error!(
                    "Failed to reach plugin \
                    {plugin_name} via {plugin_socket}: {e}"
                );
            }
        }
    }

    run_event_switch(from_api, to_api, plugin_infos).await;
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

async fn plugin_conn_thread(
    mut from_switch_rx: Receiver<NipartEvent>,
    to_switch_tx: Sender<NipartEvent>,
) {
}

async fn run_event_switch(
    mut from_api: Receiver<NipartEvent>,
    to_api: Sender<NipartEvent>,
    mut plugins: Vec<(NipartConnection, NipartPluginInfo)>,
) {
    let mut np_conn_map: HashMap<String, NipartConnection> = HashMap::new();
    let mut role_map: HashMap<NipartRole, Vec<String>> = HashMap::new();
    let mut plugin_info_map: HashMap<String, NipartPluginInfo> = HashMap::new();
    for (np_conn, plugin_info) in plugins {
        np_conn_map.insert(plugin_info.name.to_string(), np_conn);
        for role in plugin_info.roles.as_slice() {
            role_map
                .entry(*role)
                .or_insert(Vec::new())
                .push(plugin_info.name.to_string());
        }
        plugin_info_map.insert(plugin_info.name.to_string(), plugin_info);
    }

    // Should only have single commander
    let commander_name = match role_map.get(&NipartRole::Commander) {
        Some(plugins) => {
            if plugins.len() > 1 {
                log::error!("Only support single commander plugin");
                return;
            } else if plugins.len() == 1 {
                log::info!("Commander {} on duty", plugins[0]);
                plugins[0].as_str()
            } else {
                log::error!("No commander plugin found");
                return;
            }
        }
        None => {
            log::error!("No commander plugin found");
            return;
        }
    };

    // Tell commander for all the plugins we got
    let event = NipartEvent::new(
        NipartEventAction::OneShot,
        NipartEventData::UpdateAllPluginInfo(
            plugin_info_map.values().cloned().collect(),
        ),
        NipartEventAddress::Daemon,
        NipartEventAddress::Group(NipartRole::Commander),
    );
    if let Some(commanders) = role_map.get(&NipartRole::Commander) {
        for commander_name in commanders {
            if let Some(np_conn) = np_conn_map.get_mut(commander_name.as_str())
            {
                if let Err(e) = np_conn.send(&event).await {
                    log::warn!(
                        "Failed to update plugin info in \
                        commander {commander_name} {e}"
                    );
                    continue;
                }
            }
        }
    }

    loop {
        let mut plugin_futures = FuturesUnordered::new();
        for np_conn in np_conn_map.values_mut() {
            plugin_futures.push(np_conn.recv());
        }

        let event = tokio::select! {
            Some(Ok(event)) = plugin_futures.next() => {
                event
            },
            Some(mut event) = from_api.recv() => {
                event.src = NipartEventAddress::User;
                // All API event should be process by commander, hence
                // redirect API event to commander plugin(s)
                event.dst = NipartEventAddress::Group(NipartRole::Commander);
                event
            }
        };
        drop(plugin_futures);

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
                if let Err(e) = to_api.send(event.clone()).await {
                    log::warn!("Failed to send event to user: {event:?}, {e}");
                    continue;
                }
            }
            NipartEventAddress::Daemon => {
                log::error!("BUG: Got a event dst to Daemon: {event:?}");
            }
            NipartEventAddress::Commander => {
                if let Some(np_conn) = np_conn_map.get_mut(commander_name) {
                    if let Err(e) = np_conn.send(&event.clone()).await {
                        log::warn!(
                            "Failed to send event {event:?} to \
                             commander {commander_name}: {e}",
                        );
                    }
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
            NipartEventAddress::AllPluginNoCommander => {
                for (role, plugin_names) in role_map.iter() {
                    if *role != NipartRole::Commander {
                        for plugin_name in plugin_names {
                            if let Some(np_conn) =
                                np_conn_map.get_mut(plugin_name.as_str())
                            {
                                if let Err(e) =
                                    np_conn.send(&event.clone()).await
                                {
                                    log::warn!(
                                        "Failed to send event {event:?} to \
                                         plugin {plugin_name}: {e}",
                                    );
                                }
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
        NipartEventData::QueryPluginInfo,
        NipartEventAddress::Daemon,
        NipartEventAddress::Unicast(plugin_name.to_string()),
    );
    let mut np_conn = NipartConnection::new_abstract(plugin_socket)?;
    np_conn.send(&event).await?;
    let reply: NipartEvent = np_conn.recv().await?;
    if let NipartEventData::QueryPluginInfoReply(i) = reply.data {
        Ok((np_conn, i))
    } else {
        Err(NipartError::new(
            ErrorKind::Bug,
            format!("invalid reply {event:?}"),
        ))
    }
}
