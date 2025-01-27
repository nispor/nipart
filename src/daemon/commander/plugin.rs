// SPDX-License-Identifier: Apache-2.0

use nipart::{
    NipartError, NipartEvent, NipartEventAddress, NipartPluginEvent,
    NipartUserEvent, NipartUuid,
};

use super::{Task, TaskKind, WorkFlow, WorkFlowShareData};

impl WorkFlow {
    pub(crate) fn new_query_plugin_info(
        uuid: NipartUuid,
        plugin_count: usize,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        let tasks = vec![Task::new(
            uuid,
            TaskKind::QueryPluginInfo,
            plugin_count,
            timeout,
            Some(query_plugin_info),
        )];
        let share_data = WorkFlowShareData::default();

        (WorkFlow::new("query_plugin_info", uuid, tasks), share_data)
    }

    pub(crate) fn new_quit(
        uuid: NipartUuid,
        plugin_count: usize,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        let tasks = vec![Task::new(
            uuid,
            TaskKind::Quit,
            plugin_count,
            timeout,
            Some(ask_daemon_to_quit),
        )];
        let share_data = WorkFlowShareData::default();

        (WorkFlow::new("quit", uuid, tasks), share_data)
    }
}

fn query_plugin_info(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
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
    Ok(vec![NipartEvent::new_with_uuid(
        task.uuid,
        NipartUserEvent::QueryPluginInfoReply(plugin_infos),
        NipartPluginEvent::None,
        NipartEventAddress::Daemon,
        NipartEventAddress::User,
        task.timeout,
    )])
}

fn ask_daemon_to_quit(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    let mut event = NipartEvent::new_with_uuid(
        task.uuid,
        NipartUserEvent::Quit,
        NipartPluginEvent::Quit,
        NipartEventAddress::Commander,
        NipartEventAddress::Daemon,
        task.timeout,
    );
    // Give plugins 1 second to quit after they replied
    event.postpone_millis = 1000;
    Ok(vec![event])
}

impl Task {
    pub(crate) fn gen_request_query_plugin_info(&self) -> NipartEvent {
        NipartEvent::new_with_uuid(
            self.uuid,
            NipartUserEvent::None,
            NipartPluginEvent::QueryPluginInfo,
            NipartEventAddress::Commander,
            NipartEventAddress::AllPlugins,
            self.timeout,
        )
    }

    pub(crate) fn gen_request_quit(&self) -> NipartEvent {
        NipartEvent::new_with_uuid(
            self.uuid,
            NipartUserEvent::Quit,
            NipartPluginEvent::Quit,
            NipartEventAddress::Commander,
            NipartEventAddress::AllPlugins,
            self.timeout,
        )
    }
}
