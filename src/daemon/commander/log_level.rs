// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use nipart::{
    NipartError, NipartEvent, NipartEventAction, NipartEventAddress,
    NipartLogLevel, NipartPluginEvent, NipartUserEvent,
};

use crate::{
    Task, TaskCallBackFn, TaskKind, WorkFlow, WorkFlowShareData,
    DEFAULT_TIMEOUT,
};

impl WorkFlow {
    pub(crate) fn new_query_log_level(
        uuid: u128,
        plugin_count: usize,
    ) -> (Self, WorkFlowShareData) {
        let tasks = vec![Task::new(
            uuid,
            TaskKind::QueryLogLevel,
            plugin_count,
            DEFAULT_TIMEOUT,
        )];
        let share_data = WorkFlowShareData::default();

        let call_backs: Vec<Option<TaskCallBackFn>> =
            vec![Some(query_log_level)];

        (
            WorkFlow::new("query_log_level", uuid, tasks, call_backs),
            share_data,
        )
    }

    pub(crate) fn new_change_log_level(
        log_level: NipartLogLevel,
        uuid: u128,
        plugin_count: usize,
    ) -> (Self, WorkFlowShareData) {
        log::set_max_level(log_level.into());
        let tasks = vec![Task::new(
            uuid,
            TaskKind::ChangeLogLevel(log_level),
            plugin_count,
            DEFAULT_TIMEOUT,
        )];
        let share_data = WorkFlowShareData::default();

        let call_backs: Vec<Option<TaskCallBackFn>> =
            vec![Some(query_log_level)];

        (
            WorkFlow::new("change_log_level", uuid, tasks, call_backs),
            share_data,
        )
    }
}

fn query_log_level(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Option<NipartEvent>, NipartError> {
    let mut log_levels = HashMap::new();
    for reply in &task.replies {
        if let NipartPluginEvent::QueryLogLevelReply(l) = &reply.plugin {
            log_levels.insert(reply.src.to_string(), *l);
        } else {
            log::error!("BUG: Unexpected reply for query_log_level {reply:?}");
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
    reply_event.uuid = task.uuid;
    Ok(Some(reply_event))
}

impl Task {
    pub(crate) fn gen_request_query_log_level(&self) -> NipartEvent {
        let mut request = NipartEvent::new(
            NipartEventAction::Request,
            NipartUserEvent::None,
            NipartPluginEvent::QueryLogLevel,
            NipartEventAddress::Commander,
            NipartEventAddress::AllPlugins,
        );
        request.uuid = self.uuid;
        request
    }

    pub(crate) fn gen_request_change_log_level(
        &self,
        level: NipartLogLevel,
    ) -> NipartEvent {
        let mut request = NipartEvent::new(
            NipartEventAction::Request,
            NipartUserEvent::None,
            NipartPluginEvent::ChangeLogLevel(level),
            NipartEventAddress::Commander,
            NipartEventAddress::AllPlugins,
        );
        request.uuid = self.uuid;
        request
    }
}
