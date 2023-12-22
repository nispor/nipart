// SPDX-License-Identifier: Apache-2.0

use nipart::{
    NipartError, NipartEvent, NipartEventAction, NipartEventAddress,
    NipartPluginEvent, NipartUserEvent,
};
use tokio::sync::mpsc::Sender;

use crate::{Plugins, Session, SessionQueue, DEFAULT_TIMEOUT};

pub(crate) async fn handle_refresh_plugin_infos(
    commander_to_switch: &mut Sender<NipartEvent>,
    session_queue: &mut SessionQueue,
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

async fn process_refresh_plugin_infos(
    session: Session,
    plugins: &mut Plugins,
) -> Result<(), NipartError> {
    plugins.clear();
    for reply in session.replies {
        if let NipartPluginEvent::QueryPluginInfoReply(i) = reply.plugin {
            log::debug!(
                "Commander is aware of plugin {} with roles {:?}",
                i.name,
                i.roles
            );
            plugins.push(i);
        }
    }
    Ok(())
}

pub(crate) async fn process_query_plugin_info(
    session: Session,
    plugins: &mut Plugins,
    commander_to_switch: &mut Sender<NipartEvent>,
) -> Result<(), NipartError> {
    // Commander want to refresh its own knowledge of plugins
    if session.request.user == NipartUserEvent::None {
        process_refresh_plugin_infos(session, plugins).await
    } else {
        let mut plugin_infos = Vec::new();
        for reply in &session.replies {
            if let NipartPluginEvent::QueryPluginInfoReply(i) = &reply.plugin {
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
        log::trace!("commander_to_switch sending {reply_event:?}");
        commander_to_switch.send(reply_event).await?;
        Ok(())
    }
}

pub(crate) async fn handle_query_plugin_infos(
    commander_to_switch: &mut Sender<NipartEvent>,
    session_queue: &mut SessionQueue,
    plugin_count: usize,
    ref_uuid: u128,
) -> Result<(), NipartError> {
    log::debug!("Sending QueryPluginInfo to {plugin_count} plugins");
    let mut request = NipartEvent::new(
        NipartEventAction::Request,
        NipartUserEvent::QueryPluginInfo,
        NipartPluginEvent::QueryPluginInfo,
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
