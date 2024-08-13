// SPDX-License-Identifier: Apache-2.0

use nipart::{
    ErrorKind, MergedNetworkState, NetworkState, NipartApplyOption,
    NipartError, NipartEvent, NipartEventAddress, NipartPluginEvent,
    NipartQueryOption, NipartRole, NipartUserEvent,
};

use super::{Task, TaskCallBackFn, TaskKind, WorkFlow, WorkFlowShareData};
use crate::PluginRoles;

const VERIFY_RETRY_COUNT: u32 = 5;
const VERIFY_RETRY_INTERVAL: u32 = 1000;

impl WorkFlow {
    pub(crate) fn new_query_net_state(
        opt: NipartQueryOption,
        uuid: u128,
        plugins: &PluginRoles,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        // Also include DHCP plugin
        let plugin_count = plugins.get_plugin_count(NipartRole::QueryAndApply)
            + plugins.get_plugin_count(NipartRole::Dhcp);
        let tasks = vec![Task::new(
            uuid,
            TaskKind::QueryNetState(opt),
            plugin_count,
            timeout,
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
        plugins: &PluginRoles,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        let plugin_count = plugins.get_plugin_count(NipartRole::QueryAndApply)
            + plugins.get_plugin_count(NipartRole::Dhcp);

        let mut tasks = vec![
            Task::new(
                uuid,
                TaskKind::QueryRelatedNetState,
                plugin_count,
                timeout,
            ),
            Task::new(
                uuid,
                TaskKind::ApplyNetState(opt),
                plugin_count,
                timeout,
            ),
        ];
        let mut verify_task = Task::new(
            uuid,
            TaskKind::QueryRelatedNetState,
            plugin_count,
            timeout,
        );
        verify_task.set_retry(VERIFY_RETRY_COUNT, VERIFY_RETRY_INTERVAL);

        tasks.push(verify_task);
        tasks.push(Task::new(uuid, TaskKind::Commit, 1, timeout));

        let share_data = WorkFlowShareData {
            desired_state: Some(des_state),
            ..Default::default()
        };

        let call_backs: Vec<Option<TaskCallBackFn>> = vec![
            Some(pre_apply_query_related_state),
            Some(apply_net_state),
            Some(post_apply_query_related_state),
            Some(post_commit_net_state),
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
) -> Result<Vec<NipartEvent>, NipartError> {
    let mut event = if task.replies.is_empty() {
        NipartEvent::new(
            NipartUserEvent::Error(NipartError::new(
                ErrorKind::Timeout,
                "Not plugin replied the query network state call".into(),
            )),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
            task.timeout,
        )
    } else {
        let state = get_state_from_replies(task.replies.as_slice());
        NipartEvent::new(
            NipartUserEvent::QueryNetStateReply(Box::new(state)),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
            task.timeout,
        )
    };
    event.uuid = task.uuid;
    Ok(vec![event])
}

fn pre_apply_query_related_state(
    task: &Task,
    share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    let cur_state = get_state_from_replies(task.replies.as_slice());

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

    Ok(Vec::new())
}

// Since we have verification process afterwards, here we only log errors
// from plugins
fn apply_net_state(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    for reply in task.replies.as_slice() {
        if let NipartUserEvent::Error(e) = &reply.user {
            log::warn!("{e}");
        }
    }
    Ok(Vec::new())
}

// Verify
fn post_apply_query_related_state(
    task: &Task,
    share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    let post_apply_state = get_state_from_replies(task.replies.as_slice());

    let merged_state = if let Some(d) = share_data.merged_state.as_ref() {
        d.clone()
    } else {
        return Err(NipartError::new(
            ErrorKind::Bug,
            format!("Got None for merge_state in share data {share_data:?}",),
        ));
    };

    merged_state.verify(&post_apply_state)?;
    Ok(Vec::new())
}

fn post_commit_net_state(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    let mut reply = NipartEvent::new(
        NipartUserEvent::ApplyNetStateReply,
        NipartPluginEvent::None,
        NipartEventAddress::Daemon,
        NipartEventAddress::User,
        task.timeout,
    );
    reply.uuid = task.uuid;
    Ok(vec![reply])
}

impl Task {
    pub(crate) fn gen_request_query_net_state(
        &self,
        opt: NipartQueryOption,
    ) -> Vec<NipartEvent> {
        let mut ret = Vec::new();
        ret.push(NipartEvent::new(
            NipartUserEvent::None,
            NipartPluginEvent::QueryNetState(opt),
            NipartEventAddress::Commander,
            NipartEventAddress::Group(NipartRole::QueryAndApply),
            self.timeout,
        ));
        ret.push(NipartEvent::new(
            NipartUserEvent::None,
            NipartPluginEvent::QueryDhcpConfig(Box::default()),
            NipartEventAddress::Commander,
            NipartEventAddress::Dhcp,
            self.timeout,
        ));
        for request in &mut ret {
            request.uuid = self.uuid;
        }
        ret
    }

    pub(crate) fn gen_request_query_related(
        &self,
        share_data: &WorkFlowShareData,
    ) -> Vec<NipartEvent> {
        let mut ret = Vec::new();
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

        ret.push(NipartEvent::new(
            NipartUserEvent::None,
            NipartPluginEvent::QueryRelatedNetState(Box::new(desired_state)),
            NipartEventAddress::Commander,
            NipartEventAddress::Group(NipartRole::QueryAndApply),
            self.timeout,
        ));
        // TODO: Only query DHCP config for related  interfaces
        ret.push(NipartEvent::new(
            NipartUserEvent::None,
            NipartPluginEvent::QueryDhcpConfig(Box::default()),
            NipartEventAddress::Commander,
            NipartEventAddress::Dhcp,
            self.timeout,
        ));
        for request in &mut ret {
            request.uuid = self.uuid;
        }
        ret
    }

    pub(crate) fn gen_request_apply(
        &self,
        opt: NipartApplyOption,
        share_data: &WorkFlowShareData,
    ) -> Vec<NipartEvent> {
        let mut ret = Vec::new();
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
        let dhcp_changes = merged_state.get_dhcp_changes();
        ret.push(NipartEvent::new(
            NipartUserEvent::None,
            NipartPluginEvent::ApplyNetState(Box::new(merged_state), opt),
            NipartEventAddress::Commander,
            NipartEventAddress::Group(NipartRole::QueryAndApply),
            self.timeout,
        ));
        ret.push(NipartEvent::new(
            NipartUserEvent::None,
            NipartPluginEvent::ApplyDhcpConfig(Box::new(dhcp_changes)),
            NipartEventAddress::Commander,
            NipartEventAddress::Dhcp,
            self.timeout,
        ));
        for request in &mut ret {
            request.uuid = self.uuid;
        }
        ret
    }
}

fn get_state_from_replies(replies: &[NipartEvent]) -> NetworkState {
    let mut states = Vec::new();
    for reply in replies {
        if let NipartPluginEvent::QueryNetStateReply(state, priority) =
            &reply.plugin
        {
            states.push(((**state).clone(), *priority));
        } else if let NipartPluginEvent::QueryDhcpConfigReply(_) = &reply.plugin
        {
            // Will process later after multiple NetState been merged into
            // one.
        } else {
            log::error!(
                "BUG: Got unexpected reply, \
                expecting query_netstate_reply, but got {reply:?}"
            );
        }
    }
    let mut state = NetworkState::merge_states(states);

    for reply in replies {
        if let NipartPluginEvent::QueryDhcpConfigReply(dhcp_confs) =
            &reply.plugin
        {
            state.fill_dhcp_config(dhcp_confs);
        }
    }
    state
}
