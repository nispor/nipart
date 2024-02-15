// SPDX-License-Identifier: Apache-2.0



use nipart::{
    NipartError, NipartEvent, NipartEventAddress, NipartPluginEvent,
    NipartUserEvent,
};

use super::{Task, TaskCallBackFn, TaskKind, WorkFlow, WorkFlowShareData};


impl WorkFlow {
    pub(crate) fn new_query_plugin_info(
        uuid: u128,
        plugin_count: usize,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        let tasks = vec![Task::new(
            uuid,
            TaskKind::QueryPluginInfo,
            plugin_count,
            timeout,
        )];
        let share_data = WorkFlowShareData::default();

        let call_backs: Vec<Option<TaskCallBackFn>> =
            vec![Some(query_plugin_info)];

        (
            WorkFlow::new("query_plugin_info", uuid, tasks, call_backs),
            share_data,
        )
    }

    pub(crate) fn new_quit(
        uuid: u128,
        plugin_count: usize,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        let tasks =
            vec![Task::new(uuid, TaskKind::Quit, plugin_count, timeout)];
        let share_data = WorkFlowShareData::default();

        let call_backs: Vec<Option<TaskCallBackFn>> =
            vec![Some(ask_daemon_to_quit)];

        (WorkFlow::new("quit", uuid, tasks, call_backs), share_data)
    }
}

fn query_plugin_info(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Option<NipartEvent>, NipartError> {
    let mut plugin_infos = Vec::new();
    for reply in &task.replies {
        if let NipartPluginEvent::QueryPluginInfoReply(i) = &reply.plugin {
            plugin_infos.push(i.clone());
        } else {
            log::error!(
                "BUG: Got unexpected reply for query_plugin_info: {reply:?}"
            );
        }
    }
    let mut reply_event = NipartEvent::new(
        NipartUserEvent::QueryPluginInfoReply(plugin_infos),
        NipartPluginEvent::None,
        NipartEventAddress::Daemon,
        NipartEventAddress::User,
        task.timeout,
    );
    reply_event.uuid = task.uuid;
    Ok(Some(reply_event))
}

fn ask_daemon_to_quit(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Option<NipartEvent>, NipartError> {
    let mut event = NipartEvent::new(
        NipartUserEvent::Quit,
        NipartPluginEvent::Quit,
        NipartEventAddress::Commander,
        NipartEventAddress::Daemon,
        task.timeout,
    );
    event.uuid = task.uuid;
    Ok(Some(event))
}

impl Task {
    pub(crate) fn gen_request_query_plugin_info(&self) -> NipartEvent {
        let mut request = NipartEvent::new(
            NipartUserEvent::None,
            NipartPluginEvent::QueryPluginInfo,
            NipartEventAddress::Commander,
            NipartEventAddress::AllPlugins,
            self.timeout,
        );
        request.uuid = self.uuid;
        request
    }

    pub(crate) fn gen_request_quit(&self) -> NipartEvent {
        let mut request = NipartEvent::new(
            NipartUserEvent::Quit,
            NipartPluginEvent::Quit,
            NipartEventAddress::Commander,
            NipartEventAddress::AllPlugins,
            self.timeout,
        );
        request.uuid = self.uuid;
        request
    }
}
