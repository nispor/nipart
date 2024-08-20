// SPDX-License-Identifier: Apache-2.0

use futures::{stream::FuturesUnordered, StreamExt};

use nipart::{NipartError, NipartEvent, NipartEventAddress};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::time::DelayQueue;

use crate::{PluginRoles, Plugins};

pub(crate) async fn start_event_switch_thread(
    plugins: Plugins,
    api_to_switch: Receiver<NipartEvent>,
    switch_to_api: Sender<NipartEvent>,
    commander_to_switch: Receiver<NipartEvent>,
    switch_to_commander: Sender<NipartEvent>,
) -> Result<PluginRoles, NipartError> {
    let plugin_roles = plugins.roles.clone();
    tokio::spawn(async move {
        run_event_switch(
            plugins,
            api_to_switch,
            switch_to_api,
            commander_to_switch,
            switch_to_commander,
        )
        .await;
    });
    log::debug!("switch started");
    Ok(plugin_roles)
}

async fn run_event_switch(
    mut plugins: Plugins,
    mut api_to_switch: Receiver<NipartEvent>,
    switch_to_api: Sender<NipartEvent>,
    mut commander_to_switch: Receiver<NipartEvent>,
    switch_to_commander: Sender<NipartEvent>,
) {
    let mut postponed_events: DelayQueue<NipartEvent> = DelayQueue::new();
    loop {
        let mut plugin_futures = FuturesUnordered::new();
        for plugin_conn in plugins.connections.values_mut() {
            plugin_futures.push(plugin_conn.recv());
        }

        let event = tokio::select! {
            Some(Ok(event)) = plugin_futures.next() => {
                log::trace!("run_event_switch(): from plugin {event:?}");
                log::debug!("run_event_switch(): from plugin {event}");
                event
            },
            Some(event) = api_to_switch.recv() => {
                log::trace!("run_event_switch(): from daemon {event:?}");
                log::debug!("run_event_switch(): from daemon {event}");
                event
            }
            Some(event) = commander_to_switch.recv() => {
                log::trace!("run_event_switch(): from commander {event:?}");
                log::debug!("run_event_switch(): from commander {event}");
                event
            }
            Some(event) = postponed_events.next() => {
                let mut event = event.into_inner();
                log::trace!("postponed event ready to process {event:?}");
                log::trace!("postponed event ready to process {event}");
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
                "Discarding event which holds the same src and dst: {event:?}"
            );
            continue;
        }
        match &event.dst {
            NipartEventAddress::User | NipartEventAddress::Daemon => {
                if let Err(e) = switch_to_api.send(event.clone()).await {
                    log::warn!("Failed to send event: {event}, {e}");
                    continue;
                }
            }
            NipartEventAddress::Commander => {
                if let Err(e) = switch_to_commander.send(event.clone()).await {
                    log::warn!("Failed to send event: {event}, {e}");
                    continue;
                }
            }
            NipartEventAddress::Unicast(plugin_name) => {
                if let Some(plugin_conn) =
                    plugins.connections.get_mut(plugin_name.as_str())
                {
                    if let Err(e) = plugin_conn.send(&event.clone()).await {
                        log::warn!(
                            "Failed to send event {event} to \
                             plugin {plugin_name}: {e}",
                        );
                    }
                }
            }
            NipartEventAddress::Group(role) => {
                if let Some(plugin_names) = plugins.roles.get(*role) {
                    for plugin_name in plugin_names.iter() {
                        if let Some(plugin_conn) = plugins
                            .connections
                            .get_mut(&plugin_name.to_string())
                        {
                            if let Err(e) = plugin_conn.send(&event).await {
                                log::warn!(
                                    "Failed to send event {event} to \
                                     plugin {plugin_name}: {e}",
                                );
                            }
                        }
                    }
                } else {
                    log::warn!("No plugin is holding role: {role}");
                }
            }
            NipartEventAddress::AllPlugins => {
                for (plugin_name, plugin_conn) in plugins.connections.iter_mut()
                {
                    log::trace!(
                        "run_event_switch(): to plugin \
                        {plugin_name}, {event:?}"
                    );
                    if let Err(e) = plugin_conn.send(&event).await {
                        log::warn!(
                            "Failed to send event {event} to \
                             plugin {plugin_name}: {e}",
                        );
                    }
                }
            }
            NipartEventAddress::Dhcp => {
                match plugins.get_dhcp_connection_mut() {
                    Ok(plugin_conn) => {
                        if let Err(e) = plugin_conn.send(&event).await {
                            log::warn!(
                                "Failed to send event {event} to \
                                DHCP plugin: {e}",
                            );
                        }
                    }
                    Err(e) => {
                        log::error!("{e}");
                    }
                }
            }
            NipartEventAddress::Track => {
                match plugins.get_track_connection_mut() {
                    Ok(plugin_conn) => {
                        if let Err(e) = plugin_conn.send(&event).await {
                            log::warn!(
                                "Failed to send event {event} to \
                                track plugin: {e}",
                            );
                        }
                    }
                    Err(e) => {
                        log::error!("{e}");
                    }
                }
            }
            NipartEventAddress::Locker => {
                match plugins.get_locker_connection_mut() {
                    Ok(plugin_conn) => {
                        if let Err(e) = plugin_conn.send(&event).await {
                            log::warn!(
                                "Failed to send event {event} to \
                                locker plugin: {e}",
                            );
                        }
                    }
                    Err(e) => {
                        log::error!("{e}");
                    }
                }
            }
            _ => {
                log::error!("BUG: Unknown dst of event {event:?}");
            }
        }
    }
}
