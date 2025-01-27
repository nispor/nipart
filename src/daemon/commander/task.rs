// SPDX-License-Identifier: Apache-2.0

use std::time::SystemTime;

use nipart::{
    NetworkCommitQueryOption, NipartApplyOption, NipartDhcpLease, NipartError,
    NipartEvent, NipartLogLevel, NipartQueryOption, NipartUuid,
};

use super::WorkFlowShareData;

pub(crate) type TaskCallBackFn =
    fn(&Task, &mut WorkFlowShareData) -> Result<Vec<NipartEvent>, NipartError>;

#[derive(Debug, Clone)]
pub(crate) struct Task {
    pub(crate) uuid: NipartUuid,
    pub(crate) kind: TaskKind,
    pub(crate) expected_reply_count: usize,
    pub(crate) replies: Vec<NipartEvent>,
    /// timeout in seconds
    pub(crate) timeout: u32,
    pub(crate) deadline: SystemTime,
    pub(crate) retry_interval_mills: u32,
    pub(crate) retry_count: u32,
    pub(crate) max_retry_count: u32,
    /// Function been invoked after task got enough reply.
    /// The callback function will return a Vec of NipartEvent which will be
    /// send to daemon switch.
    pub(crate) callback_fn: Option<TaskCallBackFn>,
    /// Whether callback function is invoked.
    pub(crate) callback_invoked: bool,
}

impl std::fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.uuid, self.kind)
    }
}

impl Task {
    pub(crate) fn new(
        uuid: NipartUuid,
        kind: TaskKind,
        expected_reply_count: usize,
        timeout: u32,
        callback_fn: Option<TaskCallBackFn>,
    ) -> Self {
        Self {
            uuid,
            kind,
            expected_reply_count,
            replies: Vec::new(),
            timeout,
            deadline: {
                SystemTime::now()
                    .checked_add(std::time::Duration::from_millis(
                        timeout.into(),
                    ))
                    .unwrap_or_else(|| {
                        log::warn!(
                            "Timeout {timeout} has cause SystemTime overflow"
                        );
                        SystemTime::now()
                    })
            },
            retry_interval_mills: 0,
            retry_count: 0,
            max_retry_count: 0,
            callback_invoked: callback_fn.is_none(),
            callback_fn,
        }
    }

    pub(crate) fn is_expired(&self) -> bool {
        SystemTime::now() >= self.deadline && !self.is_done()
    }

    pub(crate) fn is_done(&self) -> bool {
        self.replies.len() >= self.expected_reply_count
    }

    pub(crate) fn is_callback_invoked(&self) -> bool {
        self.callback_invoked
    }

    pub(crate) fn can_retry(&self) -> bool {
        self.max_retry_count > self.retry_count
    }

    pub(crate) fn set_retry(
        &mut self,
        max_retry_count: u32,
        retry_interval_mills: u32,
    ) {
        self.max_retry_count = max_retry_count;
        self.retry_interval_mills = retry_interval_mills;
    }

    pub(crate) fn retry(&mut self) {
        if self.can_retry() {
            self.retry_count += 1;
            log::debug!(
                "Retry {}/{} for task {self}",
                self.retry_count,
                self.max_retry_count
            );
            self.replies.clear();
        } else {
            log::error!(
                "Bug: Task::retry() been invoked but it cannot retry {self:?}"
            );
        }
    }

    pub(crate) fn add_reply(&mut self, reply: NipartEvent) {
        self.replies.push(reply)
    }

    pub(crate) fn gen_request(
        &self,
        share_data: &WorkFlowShareData,
    ) -> Result<Vec<NipartEvent>, NipartError> {
        let mut events = match &self.kind {
            TaskKind::Callback => Vec::new(),
            TaskKind::QueryPluginInfo => {
                vec![self.gen_request_query_plugin_info()]
            }
            TaskKind::Quit => vec![self.gen_request_quit()],
            TaskKind::QueryNetState(opt) => {
                self.gen_request_query_net_state(opt.clone())
            }
            TaskKind::QueryRelatedNetState => {
                self.gen_request_query_related(share_data)
            }
            TaskKind::ApplyNetState(opt) => {
                self.gen_request_apply(opt.clone(), share_data)
            }
            TaskKind::QueryLogLevel => vec![self.gen_request_query_log_level()],
            TaskKind::ChangeLogLevel(l) => {
                vec![self.gen_request_change_log_level(*l)]
            }
            TaskKind::ApplyDhcpLease(lease) => {
                vec![self.gen_apply_dhcp_lease(lease.clone())]
            }
            TaskKind::QueryCommits(opt) => {
                self.gen_request_query_commits(opt.clone())
            }
            TaskKind::RemoveCommits(uuids) => {
                self.gen_request_remove_commits(uuids.as_slice(), share_data)?
            }
            TaskKind::CreateCommit => {
                self.gen_request_create_commit(share_data)
            }
            TaskKind::PostStart => self.gen_request_post_start(share_data),
            TaskKind::Lock => self.gen_request_lock(share_data),
            TaskKind::Unlock => self.gen_request_unlock(share_data),
            TaskKind::QueryLastCommitState => {
                self.gen_request_query_last_commit_state()
            }
        };
        if self.retry_count != 0 {
            for event in &mut events {
                event.postpone_millis = self.retry_interval_mills;
            }
        }
        Ok(events)
    }

    pub(crate) fn callback(
        &mut self,
        share_data: &mut WorkFlowShareData,
    ) -> Result<Vec<NipartEvent>, NipartError> {
        self.callback_invoked = true;
        if let Some(callback_fn) = self.callback_fn.as_ref() {
            log::debug!("Invoking callback function task {}", self.kind);
            callback_fn(self, share_data)
        } else {
            Ok(Vec::new())
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) enum TaskKind {
    /// No action, will invoke post call back function immediately.
    #[default]
    Callback,
    QueryPluginInfo,
    QueryNetState(NipartQueryOption),
    QueryRelatedNetState,
    ApplyNetState(NipartApplyOption),
    QueryLogLevel,
    ChangeLogLevel(NipartLogLevel),
    ApplyDhcpLease(NipartDhcpLease),
    Quit,
    QueryCommits(NetworkCommitQueryOption),
    RemoveCommits(Vec<NipartUuid>),
    /// Instruct plugin to do post start action after daemon fully started
    PostStart,
    /// Create new commit
    CreateCommit,
    /// Querying the running network state after last commit.
    QueryLastCommitState,
    Lock,
    Unlock,
}

impl std::fmt::Display for TaskKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::QueryPluginInfo => "query_plugin_info",
                Self::QueryNetState(_) => "query_net_state",
                Self::QueryRelatedNetState => "query_related_net_state",
                Self::ApplyNetState(_) => "apply_state",
                Self::QueryLogLevel => "query_log_level",
                Self::ChangeLogLevel(_) => "change_log_level",
                Self::ApplyDhcpLease(_) => "apply_dhcp_lease",
                Self::Quit => "quit",
                Self::QueryCommits(_) => "query_commits",
                Self::RemoveCommits(_) => "remove_commits",
                Self::PostStart => "post_start",
                Self::CreateCommit => "create_commit",
                Self::Lock => "lock",
                Self::Unlock => "unlock",
                Self::Callback => "callback",
                Self::QueryLastCommitState => "query_last_commit_state",
            }
        )
    }
}
