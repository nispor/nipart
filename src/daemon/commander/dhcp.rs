// SPDX-License-Identifier: Apache-2.0

use nipart::{
    NipartDhcpLease, NipartEvent, NipartEventAddress, NipartPluginEvent,
    NipartRole, NipartUserEvent, NipartUuid,
};

use super::{Task, TaskKind, WorkFlow, WorkFlowShareData};
use crate::PluginRoles;

impl WorkFlow {
    pub(crate) fn new_apply_dhcp_lease(
        uuid: NipartUuid,
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
            None,
        )];
        let share_data = WorkFlowShareData::default();

        (WorkFlow::new("apply_dhcp_lease", uuid, tasks), share_data)
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
