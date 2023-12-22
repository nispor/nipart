// SPDX-License-Identifier: Apache-2.0

use nipart::{
    ErrorKind, NipartError, NipartEvent, NipartEventAction, NipartEventAddress,
    NipartNetState, NipartPluginEvent, NipartQueryStateOption, NipartRole,
    NipartUserEvent,
};
use tokio::sync::mpsc::Sender;

use crate::{Plugins, Session, SessionQueue, DEFAULT_TIMEOUT};

pub(crate) async fn handle_query_net_state(
    commander_to_switch: &mut Sender<NipartEvent>,
    session_queue: &mut SessionQueue,
    opt: NipartQueryStateOption,
    ref_uuid: u128,
    plugins: &Plugins,
) -> Result<(), NipartError> {
    let mut request = NipartEvent::new(
        NipartEventAction::Request,
        NipartUserEvent::QueryNetState(opt.clone()),
        NipartPluginEvent::QueryNetState(opt),
        NipartEventAddress::Commander,
        NipartEventAddress::Group(NipartRole::Kernel),
    );
    let plugin_count = plugins.get_plugin_count_with_role(NipartRole::Kernel);
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

pub(crate) async fn process_query_net_state_reply(
    _session_queue: &mut SessionQueue,
    session: Session,
    commander_to_switch: &mut Sender<NipartEvent>,
) -> Result<(), NipartError> {
    let mut states = Vec::new();
    // We do not treat timeout on any plugin as error.
    if session.is_expired() {
        log::warn!(
            "Timeout on waiting plugins' reply of QueryNetState \
            expecting {} replies, got {}",
            session.expected_reply_count,
            session.replies.len()
        );
    }
    let mut reply = if session.replies.is_empty() {
        NipartEvent::new(
            NipartEventAction::Request,
            NipartUserEvent::Error(NipartError::new(
                ErrorKind::Timeout,
                "Not plugin replied the query network state call".into(),
            )),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
        )
    } else {
        for reply in session.replies {
            if let NipartPluginEvent::QueryNetStateReply(state, priority) =
                reply.plugin
            {
                states.push((state, priority));
            }
        }
        let state = NipartNetState::merge_states(states);
        NipartEvent::new(
            NipartEventAction::Request,
            NipartUserEvent::QueryNetStateReply(state),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
        )
    };
    reply.ref_uuid = Some(session.request.uuid);
    log::trace!("commander_to_switch sending {reply:?}");
    commander_to_switch.send(reply).await?;
    Ok(())
}
