// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use nipart::{
    ErrorKind, MergedNetworkState, NetworkState, NipartApplyOption,
    NipartError, NipartEvent, NipartEventAction, NipartEventAddress,
    NipartPluginEvent, NipartQueryOption, NipartRole, NipartUserEvent,
};

use crate::{
    Plugins, Task, TaskCallBackFn, TaskKind, WorkFlow, WorkFlowShareData,
    DEFAULT_TIMEOUT,
};

const VERIFY_RETRY_COUNT: u32 = 5;
const VERIFY_RETRY_INTERVAL: u32 = 1000;

impl WorkFlow {
    pub(crate) fn new_query_net_state(
        opt: NipartQueryOption,
        uuid: u128,
        plugins: Arc<Mutex<Plugins>>,
    ) -> (Self, WorkFlowShareData) {
        let plugin_count = get_plugin_count_for_query_apply(plugins);
        let tasks = vec![Task::new(
            uuid,
            TaskKind::QueryNetState(opt),
            plugin_count,
            DEFAULT_TIMEOUT,
        )];
        let share_data = WorkFlowShareData::default();

        let call_backs: Vec<Option<TaskCallBackFn>> =
            vec![Some(query_net_state)];

        (
            WorkFlow::new("query_net_state", uuid, tasks, call_backs),
            share_data,
        )
    }

    pub(crate) fn new_apply_net_state(
        des_state: NetworkState,
        opt: NipartApplyOption,
        uuid: u128,
        plugins: Arc<Mutex<Plugins>>,
    ) -> (Self, WorkFlowShareData) {
        let plugin_count = get_plugin_count_for_query_apply(plugins);
        let mut tasks = vec![
            Task::new(
                uuid,
                TaskKind::QueryRelatedNetState,
                plugin_count,
                DEFAULT_TIMEOUT,
            ),
            Task::new(
                uuid,
                TaskKind::ApplyNetState(opt),
                plugin_count,
                DEFAULT_TIMEOUT,
            ),
        ];
        let mut verify_task = Task::new(
            uuid,
            TaskKind::QueryRelatedNetState,
            plugin_count,
            DEFAULT_TIMEOUT,
        );
        verify_task.set_retry(VERIFY_RETRY_COUNT, VERIFY_RETRY_INTERVAL);

        tasks.push(verify_task);

        let share_data = WorkFlowShareData {
            desired_state: Some(des_state),
            ..Default::default()
        };

        let call_backs: Vec<Option<TaskCallBackFn>> = vec![
            Some(pre_apply_query_related_state),
            Some(apply_net_state),
            Some(post_apply_query_related_state),
        ];

        (
            WorkFlow::new("apply_net_state", uuid, tasks, call_backs),
            share_data,
        )
    }
}

fn query_net_state(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Option<NipartEvent>, NipartError> {
    let mut event = if task.replies.is_empty() {
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
        let mut states = Vec::new();
        for reply in task.replies.as_slice() {
            if let NipartPluginEvent::QueryNetStateReply(state, priority) =
                &reply.plugin
            {
                states.push(((**state).clone(), *priority));
            } else {
                log::error!(
                    "BUG: Got unexpected reply, \
                    expecting query_netstate_reply, but got {reply:?}"
                );
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
    event.uuid = task.uuid;
    Ok(Some(event))
}

fn pre_apply_query_related_state(
    task: &Task,
    share_data: &mut WorkFlowShareData,
) -> Result<Option<NipartEvent>, NipartError> {
    let mut states = Vec::new();
    for reply in task.replies.as_slice() {
        if let NipartPluginEvent::QueryRelatedNetStateReply(state, priority) =
            &reply.plugin
        {
            states.push(((**state).clone(), *priority));
        } else {
            log::error!(
                "BUG: Got unexpected reply, \
                expecting query_related_netstate_reply, but got {reply:?}"
            );
        }
    }
    let cur_state = NetworkState::merge_states(states);

    let des_state = if let Some(d) = share_data.desired_state.as_ref() {
        d.clone()
    } else {
        return Err(NipartError::new(
            ErrorKind::Bug,
            format!("Got None for desired_state in share data {share_data:?}",),
        ));
    };

    let merged_state =
        MergedNetworkState::new(des_state, cur_state.clone(), false, false)?;

    share_data.merged_state = Some(merged_state);
    share_data.pre_apply_state = Some(cur_state);

    Ok(None)
}

// Since we have verification process afterwards, here we only log errors
// from plugins
fn apply_net_state(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Option<NipartEvent>, NipartError> {
    for reply in task.replies.as_slice() {
        if let NipartUserEvent::Error(e) = &reply.user {
            log::warn!("{e}");
        }
    }
    Ok(None)
}

// Verify
fn post_apply_query_related_state(
    task: &Task,
    share_data: &mut WorkFlowShareData,
) -> Result<Option<NipartEvent>, NipartError> {
    let mut states: Vec<(NetworkState, u32)> = Vec::new();
    for reply in task.replies.as_slice() {
        if let NipartPluginEvent::QueryRelatedNetStateReply(state, priority) =
            &reply.plugin
        {
            states.push(((**state).clone(), *priority));
        } else {
            log::error!(
                "BUG: Got unexpected reply, \
                expecting query_related_netstate_reply, but got {reply:?}"
            );
        }
    }
    let post_apply_state = NetworkState::merge_states(states);

    let merged_state = if let Some(d) = share_data.merged_state.as_ref() {
        d.clone()
    } else {
        return Err(NipartError::new(
            ErrorKind::Bug,
            format!("Got None for merge_state in share data {share_data:?}",),
        ));
    };

    merged_state.verify(&post_apply_state)?;

    let mut reply = NipartEvent::new(
        NipartEventAction::Request,
        NipartUserEvent::ApplyNetStateReply,
        NipartPluginEvent::None,
        NipartEventAddress::Daemon,
        NipartEventAddress::User,
    );
    reply.uuid = task.uuid;
    Ok(Some(reply))
}

impl Task {
    pub(crate) fn gen_request_query_net_state(
        &self,
        opt: NipartQueryOption,
    ) -> NipartEvent {
        let mut request = NipartEvent::new(
            NipartEventAction::Request,
            NipartUserEvent::None,
            NipartPluginEvent::QueryNetState(opt),
            NipartEventAddress::Commander,
            NipartEventAddress::Group(NipartRole::QueryAndApply),
        );
        request.uuid = self.uuid;
        request
    }

    pub(crate) fn gen_request_query_related(
        &self,
        share_data: &WorkFlowShareData,
    ) -> NipartEvent {
        let desired_state = match share_data.desired_state.as_ref() {
            Some(s) => s.clone(),
            None => {
                log::error!(
                    "BUG: gen_request_apply() got None for \
                    desired_state in share data {share_data:?}"
                );
                NetworkState::default()
            }
        };

        let mut request = NipartEvent::new(
            NipartEventAction::Request,
            NipartUserEvent::None,
            NipartPluginEvent::QueryRelatedNetState(Box::new(desired_state)),
            NipartEventAddress::Commander,
            NipartEventAddress::Group(NipartRole::QueryAndApply),
        );
        request.uuid = self.uuid;
        request
    }

    pub(crate) fn gen_request_apply(
        &self,
        opt: NipartApplyOption,
        share_data: &WorkFlowShareData,
    ) -> NipartEvent {
        let merged_state = match share_data.merged_state.as_ref() {
            Some(s) => s.clone(),
            None => {
                log::error!(
                    "BUG: gen_request_apply() got None for \
                    merge_state in share data {share_data:?}"
                );
                MergedNetworkState::default()
            }
        };
        let mut request = NipartEvent::new(
            NipartEventAction::Request,
            NipartUserEvent::None,
            NipartPluginEvent::ApplyNetState(Box::new(merged_state), opt),
            NipartEventAddress::Commander,
            NipartEventAddress::Group(NipartRole::QueryAndApply),
        );
        request.uuid = self.uuid;
        request
    }
}

fn get_plugin_count_for_query_apply(plugins: Arc<Mutex<Plugins>>) -> usize {
    match plugins.lock() {
        Ok(plugins) => {
            plugins.get_plugin_count_with_role(NipartRole::QueryAndApply)
        }
        Err(e) => {
            log::error!("Failed to lock on Plugins {e}");
            0
        }
    }
}
