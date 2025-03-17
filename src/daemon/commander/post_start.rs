// SPDX-License-Identifier: Apache-2.0

use nipart::{
    NipartError, NipartEvent, NipartEventAddress, NipartPluginEvent,
    NipartPostStartData, NipartQueryOption, NipartRole, NipartUserEvent,
    NipartUuid,
};

use super::{
    Task, TaskKind, WorkFlow, WorkFlowShareData, state::get_state_from_replies,
};
use crate::PluginRoles;

impl WorkFlow {
    pub(crate) fn new_daemon_post_start(
        plugins: &PluginRoles,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        let uuid = NipartUuid::new();
        let query_opt = NipartQueryOption::default();

        let query_apply_plugin_count = plugins
            .get_plugin_count(NipartRole::QueryAndApply)
            + plugins.get_plugin_count(NipartRole::Dhcp);
        let tasks = vec![
            Task::new(
                uuid,
                TaskKind::QueryNetState(query_opt),
                query_apply_plugin_count,
                timeout,
                Some(query_net_state),
            ),
            Task::new(uuid, TaskKind::PostStart, 0, timeout, None),
        ];

        let share_data = WorkFlowShareData::default();

        (WorkFlow::new("daemon_post_start", uuid, tasks), share_data)
    }
}

fn query_net_state(
    task: &Task,
    share_data: &mut WorkFlowShareData,
) -> Result<Vec<NipartEvent>, NipartError> {
    if !task.replies.is_empty() {
        let state = get_state_from_replies(task.replies.as_slice());
        share_data.pre_apply_state = Some(state);
    };
    Ok(Vec::new())
}

impl Task {
    pub(crate) fn gen_request_post_start(
        &self,
        share_data: &WorkFlowShareData,
    ) -> Vec<NipartEvent> {
        let cur_state = if let Some(c) = share_data.pre_apply_state.as_ref() {
            c.clone()
        } else {
            log::error!(
                "BUG: Task::gen_request_post_start() got None for \
                share_data.pre_apply_state : {share_data:?}"
            );
            return Vec::new();
        };

        let mut post_start_data = NipartPostStartData::default();
        post_start_data.current_state = cur_state;

        vec![NipartEvent::new_with_uuid(
            self.uuid,
            NipartUserEvent::None,
            NipartPluginEvent::PostStart(Box::new(post_start_data)),
            NipartEventAddress::Commander,
            NipartEventAddress::AllPlugins,
            self.timeout,
        )]
    }
}
