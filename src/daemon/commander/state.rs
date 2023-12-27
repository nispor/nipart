// SPDX-License-Identifier: Apache-2.0

use nipart::{
    ErrorKind, MergedNetworkState, NetworkState, NipartApplyOption,
    NipartError, NipartEvent, NipartEventAction, NipartEventAddress,
    NipartPluginEvent, NipartQueryOption, NipartRole, NipartUserEvent,
};
use tokio::sync::mpsc::Sender;

use crate::{Plugins, Session, SessionQueue, DEFAULT_TIMEOUT};

pub(crate) async fn handle_query_net_state(
    commander_to_switch: &mut Sender<NipartEvent>,
    session_queue: &mut SessionQueue,
    opt: NipartQueryOption,
    ref_uuid: u128,
    plugins: &Plugins,
) -> Result<(), NipartError> {
    let mut request = NipartEvent::new(
        NipartEventAction::Request,
        NipartUserEvent::None,
        NipartPluginEvent::QueryNetState(opt),
        NipartEventAddress::Commander,
        NipartEventAddress::Group(NipartRole::QueryAndApply),
    );
    let plugin_count =
        plugins.get_plugin_count_with_role(NipartRole::QueryAndApply);
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
                states.push((*state, priority));
            }
        }
        let state = NetworkState::merge_states(states);
        NipartEvent::new(
            NipartEventAction::Request,
            NipartUserEvent::QueryNetStateReply(Box::new(state)),
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

// Collect current state from plugin which is related to desire state.
pub(crate) async fn handle_apply_net_state(
    commander_to_switch: &mut Sender<NipartEvent>,
    session_queue: &mut SessionQueue,
    desired_state: NetworkState,
    opt: NipartApplyOption,
    ref_uuid: u128,
    plugins: &Plugins,
) -> Result<(), NipartError> {
    let mut request = NipartEvent::new(
        NipartEventAction::Request,
        NipartUserEvent::ApplyNetState(Box::new(desired_state.clone()), opt),
        NipartPluginEvent::QueryRelatedNetState(Box::new(desired_state)),
        NipartEventAddress::Commander,
        NipartEventAddress::Group(NipartRole::QueryAndApply),
    );
    request.uuid = ref_uuid;
    let plugin_count =
        plugins.get_plugin_count_with_role(NipartRole::QueryAndApply);
    session_queue.new_session(
        request.uuid,
        request.clone(),
        plugin_count,
        DEFAULT_TIMEOUT,
    );
    commander_to_switch.send(request.clone()).await?;
    Ok(())
}

// Collect merge related states from plugins and generate `for_apply` state
// to apply
pub(crate) async fn process_query_related_net_state_reply(
    session_queue: &mut SessionQueue,
    session: Session,
    commander_to_switch: &mut Sender<NipartEvent>,
    plugins: &Plugins,
) -> Result<(), NipartError> {
    if session.is_expired() {
        log::warn!(
            "Timeout on waiting plugins' reply of QueryRelatedNetState \
            expecting {} replies, got {}",
            session.expected_reply_count,
            session.replies.len()
        );
    }
    let (des_state, opt) = if let NipartUserEvent::ApplyNetState(state, opt) =
        session.request.user
    {
        (*state, opt)
    } else {
        return Err(NipartError::new(
            ErrorKind::Bug,
            format!(
                "process_query_related_net_state_reply() got \
                unexpected session.user {:?}",
                session.request.user
            ),
        ));
    };

    let mut states = Vec::new();
    for reply in session.replies {
        if let NipartPluginEvent::QueryRelatedNetStateReply(state, priority) =
            reply.plugin
        {
            states.push((*state, priority));
        }
    }
    let cur_state = NetworkState::merge_states(states);

    let merged_state = match MergedNetworkState::new(
        des_state,
        cur_state.clone(),
        false,
        false,
    ) {
        Ok(s) => s,
        Err(e) => {
            let mut reply = NipartEvent::new(
                NipartEventAction::Request,
                NipartUserEvent::Error(e),
                NipartPluginEvent::None,
                NipartEventAddress::Daemon,
                NipartEventAddress::User,
            );
            reply.uuid = session.request.uuid;
            log::trace!("commander_to_switch sending {reply:?}");
            commander_to_switch.send(reply).await?;
            return Ok(());
        }
    };

    let apply_state = merged_state.gen_state_for_apply();

    let plugin_count =
        plugins.get_plugin_count_with_role(NipartRole::QueryAndApply);

    let mut request = NipartEvent::new(
        NipartEventAction::Request,
        NipartUserEvent::None,
        NipartPluginEvent::ApplyNetState(
            Box::new((apply_state, cur_state)),
            opt,
        ),
        NipartEventAddress::Commander,
        NipartEventAddress::Group(NipartRole::QueryAndApply),
    );
    request.uuid = session.request.uuid;

    session_queue.new_session(
        request.uuid,
        request.clone(),
        plugin_count,
        DEFAULT_TIMEOUT,
    );
    log::trace!("commander_to_switch sending {request:?}");
    commander_to_switch.send(request).await?;
    Ok(())
}

pub(crate) async fn process_apply_net_state_reply(
    session: Session,
    commander_to_switch: &mut Sender<NipartEvent>,
) -> Result<(), NipartError> {
    // We do not treat timeout on any plugin as error.
    if session.is_expired() {
        log::warn!(
            "Timeout on waiting plugins' reply of ApplyNetState \
            expecting {} replies, got {}",
            session.expected_reply_count,
            session.replies.len()
        );
    }
    // TODO verify state and rollback
    let mut reply = if let Some(e) = session
        .replies
        .as_slice()
        .iter()
        .find_map(|e| e.clone().into_result().err())
    {
        NipartEvent::new(
            NipartEventAction::Request,
            NipartUserEvent::Error(e),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
        )
    } else {
        NipartEvent::new(
            NipartEventAction::Request,
            NipartUserEvent::ApplyNetStateReply,
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
