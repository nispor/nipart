// SPDX-License-Identifier: Apache-2.0

use std::time::SystemTime;

use nipart::{
    NipartApplyOption, NipartDhcpLease, NipartEvent, NipartLogLevel,
    NipartQueryOption,
};

use super::WorkFlowShareData;
use crate::u128_to_uuid_string;

#[derive(Debug, Clone)]
pub(crate) struct Task {
    pub(crate) uuid: u128,
    pub(crate) kind: TaskKind,
    pub(crate) expected_reply_count: usize,
    pub(crate) replies: Vec<NipartEvent>,
    pub(crate) timeout: u32,
    pub(crate) deadline: SystemTime,
    pub(crate) retry_interval_mills: u32,
    pub(crate) retry_count: u32,
    pub(crate) max_retry_count: u32,
}

impl std::fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", u128_to_uuid_string(self.uuid), self.kind)
    }
}

impl Task {
    pub(crate) fn new(
        uuid: u128,
        kind: TaskKind,
        expected_reply_count: usize,
        timeout: u32,
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
        }
    }

    pub(crate) fn is_expired(&self) -> bool {
        SystemTime::now() >= self.deadline && !self.is_done()
    }

    pub(crate) fn is_done(&self) -> bool {
        self.replies.len() >= self.expected_reply_count
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
    ) -> Vec<NipartEvent> {
        let mut events = match &self.kind {
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
        };
        if self.retry_count != 0 {
            for event in &mut events {
                event.postpone_millis = self.retry_interval_mills;
            }
        }
        events
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) enum TaskKind {
    #[default]
    QueryPluginInfo,
    QueryNetState(NipartQueryOption),
    QueryRelatedNetState,
    ApplyNetState(NipartApplyOption),
    QueryLogLevel,
    ChangeLogLevel(NipartLogLevel),
    ApplyDhcpLease(NipartDhcpLease),
    Quit,
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
            }
        )
    }
}
