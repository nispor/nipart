// SPDX-License-Identifier: Apache-2.0

use nipart::{
    ErrorKind, MergedNetworkState, NetworkCommit, NetworkState,
    NipartApplyOption, NipartError, NipartEvent, NipartEventAddress,
    NipartLockEntry, NipartLockOption, NipartPluginEvent, NipartQueryOption,
    NipartRole, NipartStateKind, NipartUserEvent, NipartUuid,
};

use super::{Task, TaskKind, WorkFlow, WorkFlowShareData};
use crate::PluginRoles;

const VERIFY_RETRY_COUNT: u32 = 5;
const VERIFY_RETRY_INTERVAL: u32 = 1000;

/// Generate tasks for apply netstate, tasks not will send reply to user upon
/// finish.
pub(crate) fn gen_apply_net_state_tasks(
    opt: &NipartApplyOption,
    uuid: NipartUuid,
    plugins: &PluginRoles,
    timeout: u32,
) -> Vec<Task> {
    let plugin_count = plugins.get_plugin_count(NipartRole::QueryAndApply)
        + plugins.get_plugin_count(NipartRole::Dhcp);

    let mut tasks = vec![
        Task::new(
            uuid,
            TaskKind::QueryRelatedNetState,
            plugin_count,
            timeout,
            Some(pre_apply_query_related_state),
        ),
        Task::new(uuid, TaskKind::Lock, 1, timeout, None),
        Task::new(
            uuid,
            TaskKind::ApplyNetState(opt.clone()),
            plugin_count,
            timeout,
            Some(apply_net_state),
        ),
        Task::new(uuid, TaskKind::Unlock, 1, timeout, None),
    ];
    // Query post apply full network state instead of related network
    // state, so we can store this full network state as
    // commit after verification.
    let task = if opt.no_verify {
        Task::new(
            uuid,
            TaskKind::QueryNetState(Default::default()),
            plugin_count,
            timeout,
            Some(store_post_apply_state),
        )
    } else {
        new_verify_task(uuid, plugin_count, timeout)
    };

    tasks.push(task);

    if !opt.memory_only {
        tasks.push(Task::new(
            uuid,
            TaskKind::CreateCommit,
            plugins.get_plugin_count(NipartRole::Commit),
            timeout,
            Some(log_reply_error),
        ));
    }

    tasks
}

impl WorkFlow {
    pub(crate) fn new_query_net_state(
        opt: NipartQueryOption,
        uuid: NipartUuid,
        plugins: &PluginRoles,
        timeout: u32,
    ) -> Result<(Self, WorkFlowShareData), NipartError> {
        // Also include DHCP plugin
        let plugin_count = plugins.get_plugin_count(NipartRole::QueryAndApply)
            + plugins.get_plugin_count(NipartRole::Dhcp);

        match opt.kind {
            NipartStateKind::RunningNetworkState => {
                let tasks = vec![Task::new(
                    uuid,
                    TaskKind::QueryNetState(opt),
                    plugin_count,
                    timeout,
                    Some(query_net_state),
                )];
                let share_data = WorkFlowShareData::default();

                Ok((WorkFlow::new("query_net_state", uuid, tasks), share_data))
            }
            NipartStateKind::SavedNetworkState => {
                Ok(WorkFlow::new_query_net_state_in_commits(
                    plugins.get_plugin_count(NipartRole::Commit),
                    uuid,
                    timeout,
                ))
            }
            NipartStateKind::PostLastCommitNetworkState => {
                Ok(WorkFlow::new_query_post_commit_net_state(
                    plugins.get_plugin_count(NipartRole::Commit),
                    uuid,
                    timeout,
                ))
            }
            _ => Err(NipartError::new(
                ErrorKind::Bug,
                format!(
                    "BUG: Commander new_query_net_state got \
                        unexpected NipartStateKind {}",
                    opt.kind
                ),
            )),
        }
    }

    pub(crate) fn new_apply_net_state(
        des_state: NetworkState,
        opt: NipartApplyOption,
        uuid: NipartUuid,
        plugins: &PluginRoles,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        let mut tasks = gen_apply_net_state_tasks(&opt, uuid, plugins, timeout);

        tasks.push(Task::new(
            uuid,
            TaskKind::Callback,
            0,
            timeout,
            Some(reply_net_state_apply),
        ));

        let share_data = WorkFlowShareData {
            desired_state: Some(des_state),
            apply_option: Some(opt),
            ..Default::default()
        };
        (WorkFlow::new("apply_net_state", uuid, tasks), share_data)
    }
}

fn query_net_state(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    let event = if task.replies.is_empty() {
        NipartEvent::new_with_uuid(
            task.uuid,
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
        NipartEvent::new_with_uuid(
            task.uuid,
            NipartUserEvent::QueryNetStateReply(Box::new(state)),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
            task.timeout,
        )
    };
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

fn store_post_apply_state(
    task: &Task,
    share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    let post_apply_state = get_state_from_replies(task.replies.as_slice());

    let desired_state = if let Some(d) = share_data.desired_state.as_ref() {
        d.clone()
    } else {
        return Err(NipartError::new(
            ErrorKind::Bug,
            format!(
                "store_post_apply_state(): Got None for desired_state in \
                share data {share_data:?}",
            ),
        ));
    };
    let pre_apply_state = if let Some(s) = share_data.pre_apply_state.as_ref() {
        s
    } else {
        return Err(NipartError::new(
            ErrorKind::Bug,
            format!(
                "store_post_apply_state(): Got None for pre_apply_state in \
                share data {share_data:?}",
            ),
        ));
    };

    share_data.post_apply_state = Some(post_apply_state);
    share_data.commit =
        Some(NetworkCommit::new(desired_state, pre_apply_state));

    Ok(Vec::new())
}

fn verify_state(
    task: &Task,
    share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    let post_apply_state = get_state_from_replies(task.replies.as_slice());

    let merged_state = if let Some(d) = share_data.merged_state.as_ref() {
        d.clone()
    } else {
        return Err(NipartError::new(
            ErrorKind::Bug,
            format!(
                "verify_state(): Got None for merged_state in \
                share data {share_data:?}",
            ),
        ));
    };
    let desired_state = if let Some(d) = share_data.desired_state.as_ref() {
        d.clone()
    } else {
        return Err(NipartError::new(
            ErrorKind::Bug,
            format!(
                "verify_state(): Got None for desired_state in \
                share data {share_data:?}",
            ),
        ));
    };

    let pre_apply_state = if let Some(s) = share_data.pre_apply_state.as_ref() {
        s
    } else {
        return Err(NipartError::new(
            ErrorKind::Bug,
            format!(
                "verify_state(): Got None for pre_apply_state in \
                share data {share_data:?}",
            ),
        ));
    };

    merged_state.verify(&post_apply_state)?;

    share_data.post_apply_state = Some(post_apply_state);
    share_data.commit =
        Some(NetworkCommit::new(desired_state, pre_apply_state));
    Ok(Vec::new())
}

fn log_reply_error(
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

fn reply_net_state_apply(
    task: &Task,
    share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    if Some(true) == share_data.apply_option.as_ref().map(|opt| opt.memory_only)
    {
        Ok(vec![NipartEvent::new_with_uuid(
            task.uuid,
            NipartUserEvent::ApplyNetStateReply(Box::new(None)),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
            task.timeout,
        )])
    } else {
        Ok(vec![NipartEvent::new_with_uuid(
            task.uuid,
            NipartUserEvent::ApplyNetStateReply(Box::new(
                share_data.commit.clone(),
            )),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
            task.timeout,
        )])
    }
}

impl Task {
    pub(crate) fn gen_request_query_net_state(
        &self,
        opt: NipartQueryOption,
    ) -> Vec<NipartEvent> {
        vec![
            NipartEvent::new_with_uuid(
                self.uuid,
                NipartUserEvent::None,
                NipartPluginEvent::QueryNetState(opt),
                NipartEventAddress::Commander,
                NipartEventAddress::Group(NipartRole::QueryAndApply),
                self.timeout,
            ),
            NipartEvent::new_with_uuid(
                self.uuid,
                NipartUserEvent::None,
                NipartPluginEvent::QueryDhcpConfig(Box::default()),
                NipartEventAddress::Commander,
                NipartEventAddress::Dhcp,
                self.timeout,
            ),
        ]
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

        ret.push(NipartEvent::new_with_uuid(
            self.uuid,
            NipartUserEvent::None,
            NipartPluginEvent::QueryRelatedNetState(Box::new(desired_state)),
            NipartEventAddress::Commander,
            NipartEventAddress::Group(NipartRole::QueryAndApply),
            self.timeout,
        ));
        // TODO: Only query DHCP config for related  interfaces
        ret.push(NipartEvent::new_with_uuid(
            self.uuid,
            NipartUserEvent::None,
            NipartPluginEvent::QueryDhcpConfig(Box::default()),
            NipartEventAddress::Commander,
            NipartEventAddress::Dhcp,
            self.timeout,
        ));
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
                    merged_state in share data {share_data:?}"
                );
                MergedNetworkState::default()
            }
        };
        let dhcp_changes = merged_state.get_dhcp_changes();
        ret.push(NipartEvent::new_with_uuid(
            self.uuid,
            NipartUserEvent::None,
            NipartPluginEvent::ApplyNetState(Box::new(merged_state), opt),
            NipartEventAddress::Commander,
            NipartEventAddress::Group(NipartRole::QueryAndApply),
            self.timeout,
        ));
        ret.push(NipartEvent::new_with_uuid(
            self.uuid,
            NipartUserEvent::None,
            NipartPluginEvent::ApplyDhcpConfig(Box::new(dhcp_changes)),
            NipartEventAddress::Commander,
            NipartEventAddress::Dhcp,
            self.timeout,
        ));
        ret
    }

    pub(crate) fn gen_request_lock(
        &self,
        share_data: &WorkFlowShareData,
    ) -> Vec<NipartEvent> {
        let merged_state = match share_data.merged_state.as_ref() {
            Some(s) => s,
            None => {
                log::error!(
                    "BUG: gen_request_lock() got None for \
                    merged_state in share data {share_data:?}"
                );
                return Vec::new();
            }
        };
        let locks = gen_locks(merged_state, self.timeout);
        vec![NipartEvent::new_with_uuid(
            self.uuid,
            NipartUserEvent::None,
            NipartPluginEvent::Lock(Box::new(locks)),
            NipartEventAddress::Commander,
            NipartEventAddress::Locker,
            self.timeout,
        )]
    }

    pub(crate) fn gen_request_unlock(
        &self,
        share_data: &WorkFlowShareData,
    ) -> Vec<NipartEvent> {
        let merged_state = match share_data.merged_state.as_ref() {
            Some(s) => s,
            None => {
                log::error!(
                    "BUG: gen_request_lock() got None for \
                    merged_state in share data {share_data:?}"
                );
                return Vec::new();
            }
        };
        let locks: Vec<NipartLockEntry> = gen_locks(merged_state, self.timeout)
            .into_iter()
            .map(|(lock, _)| lock)
            .collect();
        vec![NipartEvent::new_with_uuid(
            self.uuid,
            NipartUserEvent::None,
            NipartPluginEvent::Unlock(Box::new(locks)),
            NipartEventAddress::Commander,
            NipartEventAddress::Locker,
            self.timeout,
        )]
    }
}

pub(crate) fn get_state_from_replies(replies: &[NipartEvent]) -> NetworkState {
    let mut states: Vec<(NetworkState, u32)> = Vec::new();
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

    states.sort_unstable_by(|a, b| a.1.cmp(&b.1));
    let states: Vec<NetworkState> =
        states.into_iter().map(|(state, _)| state).collect();

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

fn gen_locks(
    merged_state: &MergedNetworkState,
    timeout: u32,
) -> Vec<(NipartLockEntry, NipartLockOption)> {
    let mut locks = Vec::new();
    for iface in merged_state
        .interfaces
        .iter()
        .filter_map(|i| i.for_apply.as_ref())
    {
        locks.push((
            NipartLockEntry::new_iface(
                iface.name().to_string(),
                iface.iface_type(),
            ),
            NipartLockOption::new(timeout),
        ));
    }

    if merged_state.dns.is_changed() {
        locks.push((NipartLockEntry::Dns, NipartLockOption::new(timeout)));
    }

    if merged_state.routes.is_changed() {
        locks.push((NipartLockEntry::Route, NipartLockOption::new(timeout)));
    }

    if merged_state.rules.is_changed() {
        locks
            .push((NipartLockEntry::RouteRule, NipartLockOption::new(timeout)));
    }
    locks
}

pub(crate) fn new_verify_task(
    uuid: NipartUuid,
    plugin_count: usize,
    timeout: u32,
) -> Task {
    let mut verify_task = Task::new(
        uuid,
        TaskKind::QueryNetState(Default::default()),
        plugin_count,
        timeout,
        Some(verify_state),
    );
    verify_task.set_retry(VERIFY_RETRY_COUNT, VERIFY_RETRY_INTERVAL);
    verify_task
}
