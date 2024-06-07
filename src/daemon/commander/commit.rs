// SPDX-License-Identifier: Apache-2.0

use nipart::{
    ErrorKind, NetworkCommit, NetworkCommitQueryOption, NetworkState,
    NipartError, NipartEvent, NipartEventAddress, NipartPluginEvent,
    NipartUserEvent,
};

use super::{Task, TaskCallBackFn, TaskKind, WorkFlow, WorkFlowShareData};

impl WorkFlow {
    pub(crate) fn new_query_commits(
        opt: NetworkCommitQueryOption,
        uuid: u128,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        // Only single tracking plugin allowed for now.
        let plugin_count = 1;
        let tasks = vec![Task::new(
            uuid,
            TaskKind::QueryCommits(opt),
            plugin_count,
            timeout,
        )];
        let share_data = WorkFlowShareData::default();

        let call_backs: Vec<Option<TaskCallBackFn>> =
            vec![Some(query_net_commits)];

        (
            WorkFlow::new("query_commit", uuid, tasks, call_backs),
            share_data,
        )
    }
}

fn query_net_commits(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    let mut event = if task.replies.is_empty() {
        NipartEvent::new(
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
        NipartEvent::new(
            NipartUserEvent::QueryCommitsReply(Box::new(ret_commits)),
            NipartPluginEvent::None,
            NipartEventAddress::Daemon,
            NipartEventAddress::User,
            task.timeout,
        )
    };
    event.uuid = task.uuid;
    Ok(vec![event])
}

impl Task {
    pub(crate) fn gen_request_query_commits(
        &self,
        opt: NetworkCommitQueryOption,
    ) -> Vec<NipartEvent> {
        let mut event = NipartEvent::new(
            NipartUserEvent::None,
            NipartPluginEvent::QueryCommits(opt),
            NipartEventAddress::Commander,
            NipartEventAddress::Track,
            self.timeout,
        );
        event.uuid = self.uuid;
        vec![event]
    }

    pub(crate) fn gen_request_commit(
        &self,
        share_data: &WorkFlowShareData,
    ) -> Vec<NipartEvent> {
        let state = if let Some(s) = share_data.desired_state.as_ref() {
            s.clone()
        } else {
            log::error!(
                "BUG: Task::gen_request_commit() got None for \
                share_data.desired_state: {share_data:?}"
            );
            NetworkState::default()
        };

        let mut event = NipartEvent::new(
            NipartUserEvent::None,
            NipartPluginEvent::Commit(Box::new(state)),
            NipartEventAddress::Commander,
            NipartEventAddress::Track,
            self.timeout,
        );
        event.uuid = self.uuid;
        vec![event]
    }
}
