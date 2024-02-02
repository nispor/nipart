// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use nipart::{
    ErrorKind, NipartError, NipartEvent, NipartEventAction, NipartEventAddress,
    NipartPluginEvent, NipartUserEvent,
};

use crate::{
    Plugins, Task, TaskCallBackFn, TaskKind, WorkFlow, WorkFlowShareData,
    DEFAULT_TIMEOUT,
};

impl WorkFlow {
    pub(crate) fn new_refresh_plugins(
        plugins: Arc<Mutex<Plugins>>,
        uuid: u128,
        new_plugin_count: usize,
    ) -> (Self, WorkFlowShareData) {
        let tasks = vec![Task::new(
            uuid,
            TaskKind::RefreshPluginInfo,
            new_plugin_count,
            DEFAULT_TIMEOUT,
        )];
        let share_data = WorkFlowShareData {
            plugins: Some(plugins),
            ..Default::default()
        };

        let call_backs: Vec<Option<TaskCallBackFn>> =
            vec![Some(refresh_plugin_info)];

        (
            WorkFlow::new("refresh_plugin_info", uuid, tasks, call_backs),
            share_data,
        )
    }

    pub(crate) fn new_query_plugin_info(
        uuid: u128,
        plugin_count: usize,
    ) -> (Self, WorkFlowShareData) {
        let tasks = vec![Task::new(
            uuid,
            TaskKind::QueryPluginInfo,
            plugin_count,
            DEFAULT_TIMEOUT,
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
    ) -> (Self, WorkFlowShareData) {
        let tasks = vec![Task::new(
            uuid,
            TaskKind::Quit,
            plugin_count,
            DEFAULT_TIMEOUT,
        )];
        let share_data = WorkFlowShareData::default();

        let call_backs: Vec<Option<TaskCallBackFn>> =
            vec![Some(ask_daemon_to_quit)];

        (WorkFlow::new("quit", uuid, tasks, call_backs), share_data)
    }
}

fn refresh_plugin_info(
    task: &Task,
    share_data: &mut WorkFlowShareData,
) -> Result<Option<NipartEvent>, NipartError> {
    if let Some(plugins) = share_data.plugins.as_ref() {
        match plugins.lock() {
            Ok(mut plugins) => {
                plugins.clear();
                for reply in &task.replies {
                    if let NipartPluginEvent::QueryPluginInfoReply(i) =
                        &reply.plugin
                    {
                        log::debug!(
                            "Commander is aware of plugin {} with roles {:?}",
                            i.name,
                            i.roles
                        );
                        plugins.push(i.clone());
                    } else {
                        log::error!(
                            "BUG: Unexpected reply {reply:?} for \
                            refresh_plugin_info()"
                        );
                    }
                }
                Ok(None)
            }
            Err(e) => Err(NipartError::new(
                ErrorKind::Bug,
                format!("Failed to lock plugins in share data: {e}"),
            )),
        }
    } else {
        Err(NipartError::new(
            ErrorKind::Bug,
            format!("Got None plugins in share data: {share_data:?}"),
        ))
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
        NipartEventAction::Done,
        NipartUserEvent::QueryPluginInfoReply(plugin_infos),
        NipartPluginEvent::None,
        NipartEventAddress::Daemon,
        NipartEventAddress::User,
    );
    reply_event.uuid = task.uuid;
    Ok(Some(reply_event))
}

fn ask_daemon_to_quit(
    task: &Task,
    _share_data: &mut WorkFlowShareData,
) -> Result<Option<NipartEvent>, NipartError> {
    let mut event = NipartEvent::new(
        NipartEventAction::Request,
        NipartUserEvent::Quit,
        NipartPluginEvent::Quit,
        NipartEventAddress::Commander,
        NipartEventAddress::Daemon,
    );
    event.uuid = task.uuid;
    Ok(Some(event))
}

impl Task {
    pub(crate) fn gen_request_query_plugin_info(&self) -> NipartEvent {
        let mut request = NipartEvent::new(
            NipartEventAction::Request,
            NipartUserEvent::None,
            NipartPluginEvent::QueryPluginInfo,
            NipartEventAddress::Commander,
            NipartEventAddress::AllPlugins,
        );
        request.uuid = self.uuid;
        request
    }

    pub(crate) fn gen_request_refresh_plugin(&self) -> NipartEvent {
        let mut request = NipartEvent::new(
            NipartEventAction::Request,
            NipartUserEvent::None,
            NipartPluginEvent::QueryPluginInfo,
            NipartEventAddress::Commander,
            NipartEventAddress::AllPlugins,
        );
        request.uuid = self.uuid;
        request
    }

    pub(crate) fn gen_request_quit(&self) -> NipartEvent {
        let mut request = NipartEvent::new(
            NipartEventAction::Request,
            NipartUserEvent::Quit,
            NipartPluginEvent::Quit,
            NipartEventAddress::Commander,
            NipartEventAddress::AllPlugins,
        );
        request.uuid = self.uuid;
        request
    }
}
