// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use nipart::{
    NipartError, NipartEvent, NipartEventAction, NipartEventAddress,
    NipartLogLevel, NipartPluginEvent, NipartUserEvent,
};
use tokio::sync::mpsc::Sender;

use crate::{Session, SessionQueue, DEFAULT_TIMEOUT};

pub(crate) async fn handle_query_log_level(
    commander_to_switch: &mut Sender<NipartEvent>,
    session_queue: &mut SessionQueue,
    ref_uuid: u128,
    plugin_count: usize,
) -> Result<(), NipartError> {
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
    log::trace!("commander_to_switch sending {request:?}");
    commander_to_switch.send(request.clone()).await?;
    Ok(())
}

pub(crate) async fn handle_change_log_level(
    commander_to_switch: &mut Sender<NipartEvent>,
    session_queue: &mut SessionQueue,
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

pub(crate) async fn process_query_log_level(
    session: Session,
    commander_to_switch: &mut Sender<NipartEvent>,
) -> Result<(), NipartError> {
    let mut log_levels = HashMap::new();
    for reply in &session.replies {
        if let NipartPluginEvent::QueryLogLevelReply(l) = &reply.plugin {
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
    log::trace!("commander_to_switch sending {reply_event:?}");
    commander_to_switch.send(reply_event).await?;

    Ok(())
}
