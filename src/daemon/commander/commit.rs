// SPDX-License-Identifier: Apache-2.0

use nipart::{
    ErrorKind, NetworkCommit, NetworkCommitQueryOption, NetworkState,
    NipartApplyOption, NipartError, NipartEvent, NipartEventAddress,
    NipartPluginEvent, NipartRole, NipartUserEvent, NipartUuid,
};

use super::{
    Task, TaskKind, WorkFlow, WorkFlowShareData,
    state::gen_apply_net_state_tasks,
};
use crate::PluginRoles;

impl WorkFlow {
    pub(crate) fn new_query_net_state_in_commits(
        plugin_count: usize,
        uuid: NipartUuid,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        let tasks = vec![Task::new(
            uuid,
            TaskKind::QueryCommits(Default::default()),
            plugin_count,
            timeout,
            Some(query_net_state_from_commits),
        )];
        let share_data = WorkFlowShareData::default();

        (WorkFlow::new("query_commit", uuid, tasks), share_data)
    }

    pub(crate) fn new_query_post_commit_net_state(
        plugin_count: usize,
        uuid: NipartUuid,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        let tasks = vec![Task::new(
            uuid,
            TaskKind::QueryLastCommitState,
            plugin_count,
            timeout,
            Some(handle_query_last_commit_state_reply),
        )];
        let share_data = WorkFlowShareData::default();

        (WorkFlow::new("query_commit", uuid, tasks), share_data)
    }

    pub(crate) fn new_query_commits(
        opt: NetworkCommitQueryOption,
        plugin_count: usize,
        uuid: NipartUuid,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        let tasks = vec![Task::new(
            uuid,
            TaskKind::QueryCommits(opt),
            plugin_count,
            timeout,
            Some(query_net_commits),
        )];
        let share_data = WorkFlowShareData::default();

        (WorkFlow::new("query_commit", uuid, tasks), share_data)
    }

    pub(crate) fn new_remove_commits(
        uuids: Vec<NipartUuid>,
        plugin_roles: &PluginRoles,
        event_uuid: NipartUuid,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        let mut commit_opt = NetworkCommitQueryOption::default();
        commit_opt.uuids = uuids.clone();
        let mut apply_opt = NipartApplyOption::default();
        apply_opt.memory_only = true;
        // TODO: Valid whether remains commits is depend on removing commits
        let mut tasks = vec![Task::new(
            event_uuid,
            TaskKind::QueryCommits(commit_opt),
            plugin_roles.get_plugin_count(NipartRole::Commit),
            timeout,
            Some(gen_net_state_for_removed_commits),
        )];

        let mut apply_tasks = gen_apply_net_state_tasks(
            &apply_opt,
            event_uuid,
            plugin_roles,
            timeout,
        );

        tasks.append(&mut apply_tasks);

        tasks.push(Task::new(
            event_uuid,
            TaskKind::RemoveCommits(uuids),
            plugin_roles.get_plugin_count(NipartRole::Commit),
            timeout,
            Some(process_remove_commits_reply),
        ));

        let share_data = WorkFlowShareData::default();

        (
            WorkFlow::new("remove_commit", event_uuid, tasks),
            share_data,
        )
    }
}

fn query_net_commits(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    let event = if task.replies.is_empty() {
        NipartEvent::new_with_uuid(
            task.uuid,
            NipartUserEvent::Error(NipartError::new(
                ErrorKind::Timeout,
                "Not plugin replied the query network commits call".into(),
            )),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
            task.timeout,
        )
    } else {
        let mut ret_commits: Vec<NetworkCommit> = Vec::new();
        for reply in task.replies.as_slice() {
            if let NipartPluginEvent::QueryCommitsReply(commits) = &reply.plugin
            {
                ret_commits.extend_from_slice(commits.as_slice());
            }
        }
        NipartEvent::new_with_uuid(
            task.uuid,
            NipartUserEvent::QueryCommitsReply(Box::new(ret_commits)),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
            task.timeout,
        )
    };
    Ok(vec![event])
}

fn query_net_state_from_commits(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    let event = if task.replies.is_empty() {
        NipartEvent::new_with_uuid(
            task.uuid,
            NipartUserEvent::Error(NipartError::new(
                ErrorKind::Timeout,
                "Not plugin replied the query network commits call".into(),
            )),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
            task.timeout,
        )
    } else {
        let mut net_state = NetworkState::default();
        // TODO: Introduce plugin priority for merging network states
        for reply in task.replies.as_slice() {
            if let NipartPluginEvent::QueryCommitsReply(commits) = &reply.plugin
            {
                for commit in commits.as_slice() {
                    net_state.merge(&commit.desired_state)?;
                }
            }
        }
        NipartEvent::new_with_uuid(
            task.uuid,
            NipartUserEvent::QueryNetStateReply(Box::new(net_state)),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
            task.timeout,
        )
    };
    Ok(vec![event])
}

fn handle_query_last_commit_state_reply(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    let mut net_state = NetworkState::default();
    for reply in task.replies.as_slice() {
        if let NipartPluginEvent::QueryLastCommitStateReply(sub_state) =
            &reply.plugin
        {
            net_state.merge(sub_state)?;
        }
    }
    Ok(vec![NipartEvent::new_with_uuid(
        task.uuid,
        NipartUserEvent::QueryNetStateReply(Box::new(net_state)),
        NipartPluginEvent::None,
        NipartEventAddress::Daemon,
        NipartEventAddress::User,
        task.timeout,
    )])
}

impl Task {
    pub(crate) fn gen_request_query_commits(
        &self,
        opt: NetworkCommitQueryOption,
    ) -> Vec<NipartEvent> {
        vec![NipartEvent::new_with_uuid(
            self.uuid,
            NipartUserEvent::None,
            NipartPluginEvent::QueryCommits(opt),
            NipartEventAddress::Commander,
            NipartEventAddress::Group(NipartRole::Commit),
            self.timeout,
        )]
    }

    pub(crate) fn gen_request_create_commit(
        &self,
        share_data: &WorkFlowShareData,
    ) -> Vec<NipartEvent> {
        let (commit, post_state) = if let (Some(d), Some(s)) = (
            share_data.commit.as_ref(),
            share_data.post_apply_state.as_ref(),
        ) {
            (d.clone(), s.clone())
        } else {
            log::error!(
                "BUG: Task::gen_request_commit() got None for \
                share_data.commit or post_apply_state: {share_data:?}"
            );
            return Vec::new();
        };

        vec![NipartEvent::new_with_uuid(
            self.uuid,
            NipartUserEvent::None,
            NipartPluginEvent::CreateCommit(Box::new((commit, post_state))),
            NipartEventAddress::Commander,
            NipartEventAddress::Group(NipartRole::Commit),
            self.timeout,
        )]
    }

    pub(crate) fn gen_request_query_last_commit_state(
        &self,
    ) -> Vec<NipartEvent> {
        vec![NipartEvent::new_with_uuid(
            self.uuid,
            NipartUserEvent::None,
            NipartPluginEvent::QueryLastCommitState,
            NipartEventAddress::Commander,
            NipartEventAddress::Group(NipartRole::Commit),
            self.timeout,
        )]
    }

    pub(crate) fn gen_request_remove_commits(
        &self,
        uuids: &[NipartUuid],
        share_data: &WorkFlowShareData,
    ) -> Result<Vec<NipartEvent>, NipartError> {
        if let Some(post_state) = share_data.post_apply_state.as_ref() {
            Ok(vec![NipartEvent::new_with_uuid(
                self.uuid,
                NipartUserEvent::None,
                NipartPluginEvent::RemoveCommits(Box::new((
                    uuids.to_vec(),
                    post_state.clone(),
                ))),
                NipartEventAddress::Commander,
                NipartEventAddress::Group(NipartRole::Commit),
                self.timeout,
            )])
        } else {
            Err(NipartError::new(
                ErrorKind::Bug,
                format!(
                    "gen_request_remove_commits() invoked with \
                    empty share_data.post_apply_state: {:?}",
                    self
                ),
            ))
        }
    }
}

fn process_remove_commits_reply(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    let mut net_state = NetworkState::new();
    for reply in task.replies.as_slice() {
        if let NipartPluginEvent::RemoveCommitsReply(state) = &reply.plugin {
            net_state.merge(state)?;
        }
    }
    Ok(vec![NipartEvent::new_with_uuid(
        task.uuid,
        NipartUserEvent::RemoveCommitsReply(Box::new(net_state)),
        NipartPluginEvent::None,
        NipartEventAddress::Daemon,
        NipartEventAddress::User,
        task.timeout,
    )])
}

fn gen_net_state_for_removed_commits(
    task: &Task,
    share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    let mut net_state = NetworkState::new();
    for reply in task.replies.as_slice() {
        if let NipartPluginEvent::QueryCommitsReply(commits) = &reply.plugin {
            for commit in commits.iter().rev() {
                net_state.merge(&commit.revert_state)?;
            }
        }
    }
    share_data.desired_state = Some(net_state);
    Ok(vec![])
}
