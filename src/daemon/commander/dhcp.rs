// SPDX-License-Identifier: Apache-2.0

use nipart::{
    NipartDhcpLease, NipartEvent, NipartEventAddress, NipartPluginEvent,
    NipartRole, NipartUserEvent,
};

use super::{Task, TaskCallBackFn, TaskKind, WorkFlow, WorkFlowShareData};
use crate::PluginRoles;

impl WorkFlow {
    pub(crate) fn new_apply_dhcp_lease(
        uuid: u128,
        lease: NipartDhcpLease,
        plugins: &PluginRoles,
        timeout: u32,
    ) -> (Self, WorkFlowShareData) {
        let plugin_count = plugins.get_plugin_count(NipartRole::ApplyDhcpLease);
        let tasks = vec![Task::new(
            uuid,
            TaskKind::ApplyDhcpLease(lease),
            plugin_count,
            timeout,
        )];
        let share_data = WorkFlowShareData::default();

        let call_backs: Vec<Option<TaskCallBackFn>> = vec![None];

        (
            WorkFlow::new("apply_dhcp_lease", uuid, tasks, call_backs),
            share_data,
        )
    }
}

impl Task {
    pub(crate) fn gen_apply_dhcp_lease(
        &self,
        lease: NipartDhcpLease,
    ) -> NipartEvent {
        NipartEvent::new_with_uuid(
            self.uuid,
            NipartUserEvent::None,
            NipartPluginEvent::ApplyDhcpLease(Box::new(lease)),
            NipartEventAddress::Commander,
            NipartEventAddress::Group(NipartRole::ApplyDhcpLease),
            self.timeout,
        )
    }
}
