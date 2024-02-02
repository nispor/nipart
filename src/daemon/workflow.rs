// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use nipart::{
    ErrorKind, MergedNetworkState, NetworkState, NipartError, NipartEvent,
    NipartEventAction, NipartEventAddress, NipartPluginEvent, NipartUserEvent,
};

use crate::{u128_to_uuid_string, Plugins, Task};

pub(crate) type TaskCallBackFn = fn(
    &Task,
    &mut WorkFlowShareData,
)
    -> Result<Option<NipartEvent>, NipartError>;

#[derive(Debug, Clone, Default)]
pub(crate) struct WorkFlowShareData {
    pub(crate) desired_state: Option<NetworkState>,
    pub(crate) pre_apply_state: Option<NetworkState>,
    pub(crate) merged_state: Option<MergedNetworkState>,
    pub(crate) plugins: Option<Arc<Mutex<Plugins>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkFlow {
    pub(crate) kind: String,
    pub(crate) uuid: u128,
    pub(crate) tasks: Vec<Task>,
    pub(crate) task_callbacks: Vec<Option<TaskCallBackFn>>,
    pub(crate) cur_task_idx: usize,
    init_request_sent: bool,
    is_fail: bool,
}

impl std::fmt::Display for WorkFlow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.kind, u128_to_uuid_string(self.uuid))
    }
}

impl WorkFlow {
    pub(crate) fn new(
        kind: &str,
        uuid: u128,
        tasks: Vec<Task>,
        task_callbacks: Vec<Option<TaskCallBackFn>>,
    ) -> Self {
        Self {
            kind: kind.to_string(),
            uuid,
            tasks,
            task_callbacks,
            cur_task_idx: 0,
            init_request_sent: false,
            is_fail: false,
        }
    }

    pub(crate) fn gen_cur_task_request_event(
        &self,
        share_data: &mut WorkFlowShareData,
    ) -> Result<NipartEvent, NipartError> {
        if let Some(task) = self.cur_task() {
            Ok(task.gen_request(share_data))
        } else {
            Err(NipartError::new(
                ErrorKind::Bug,
                format!(
                    "Current task of workflow {} is None: {:?}",
                    self.kind, self
                ),
            ))
        }
    }

    pub(crate) fn cur_task(&self) -> Option<&Task> {
        self.tasks.get(self.cur_task_idx)
    }

    pub(crate) fn cur_task_mut(&mut self) -> Option<&mut Task> {
        self.tasks.get_mut(self.cur_task_idx)
    }

    pub(crate) fn cur_task_callback(
        &mut self,
        share_data: &mut WorkFlowShareData,
    ) -> Result<Option<NipartEvent>, NipartError> {
        let callback_fn = match self.task_callbacks.get(self.cur_task_idx) {
            Some(Some(f)) => f.clone(),
            _ => {
                return Ok(None);
            }
        };
        let result = match self.cur_task_mut() {
            Some(t) => callback_fn(t, share_data),
            None => {
                return Ok(None);
            }
        };

        match result {
            Ok(e) => Ok(e),
            Err(e) => {
                if let Some(cur_task) = self.cur_task_mut() {
                    if cur_task.can_retry() {
                        log::debug!("Retry on error {e}");
                        cur_task.retry();
                        return Ok(Some(cur_task.gen_request(share_data)));
                    }
                }
                Err(e)
            }
        }
    }

    pub(crate) fn is_expired(&self) -> bool {
        self.cur_task().map(|t| t.is_expired()).unwrap_or_else(|| {
            log::error!("BUG: Current task is None {self:?}");
            true
        })
    }

    pub(crate) fn cur_task_is_done(&self) -> bool {
        self.cur_task().map(|t| t.is_done()).unwrap_or_else(|| {
            log::error!("BUG: Current task is None {self:?}");
            false
        })
    }

    pub(crate) fn is_done(&self) -> bool {
        self.tasks.len() == self.cur_task_idx + 1 && self.cur_task_is_done()
    }

    pub(crate) fn is_fail(&self) -> bool {
        self.is_fail
    }

    pub(crate) fn add_reply(&mut self, reply: NipartEvent) {
        if let Some(task) = self.cur_task_mut() {
            task.add_reply(reply);
        } else {
            log::error!("BUG: Current task is None {self:?}");
        }
    }

    pub(crate) fn process(
        &mut self,
        share_data: &mut WorkFlowShareData,
    ) -> Result<Vec<NipartEvent>, NipartError> {
        let mut ret: Vec<NipartEvent> = Vec::new();
        if !self.init_request_sent {
            ret.push(self.gen_cur_task_request_event(share_data)?);
            self.init_request_sent = true;
            return Ok(ret);
        }

        if self.is_expired() {
            return Ok(vec![NipartEvent::new(
                NipartEventAction::Done,
                NipartUserEvent::Error(NipartError::new(
                    ErrorKind::Timeout,
                    format!("Timeout on action {} {}", self.uuid, self.kind),
                )),
                NipartPluginEvent::None,
                NipartEventAddress::Daemon,
                NipartEventAddress::User,
            )]);
        }

        if self.cur_task_is_done() {
            match self.cur_task_callback(share_data) {
                Ok(Some(e)) => ret.push(e),
                Ok(None) => (),
                Err(e) => {
                    self.is_fail = true;
                    let mut error_event: NipartEvent = e.into();
                    error_event.uuid = self.uuid;
                    return Ok(vec![error_event]);
                }
            }
            if self.cur_task_idx + 1 < self.tasks.len() {
                self.cur_task_idx += 1;
                ret.push(self.gen_cur_task_request_event(share_data)?);
            }
        }

        Ok(ret)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WorkFlowQueue {
    pub(crate) workflows: HashMap<u128, WorkFlow>,
    pub(crate) share_data: HashMap<u128, WorkFlowShareData>,
}

impl WorkFlowQueue {
    const INIT_CAPACITY: usize = 1024;

    pub(crate) fn new() -> Self {
        Self {
            workflows: HashMap::with_capacity(Self::INIT_CAPACITY),
            share_data: HashMap::with_capacity(Self::INIT_CAPACITY),
        }
    }

    pub(crate) fn add_workflow(
        &mut self,
        workflow: WorkFlow,
        share_data: WorkFlowShareData,
    ) {
        self.share_data.insert(workflow.uuid, share_data);
        self.workflows.insert(workflow.uuid, workflow);
    }

    pub(crate) fn add_reply(&mut self, reply: NipartEvent) {
        if let Some(workflow) = self.workflows.get_mut(&reply.uuid) {
            workflow.add_reply(reply);
        }
    }

    // Check whether any task finished or expired
    pub(crate) fn process(&mut self) -> Result<Vec<NipartEvent>, NipartError> {
        let mut ret: Vec<NipartEvent> = Vec::new();

        for workflow in self.workflows.values_mut() {
            if let Some(share_data) = self.share_data.get_mut(&workflow.uuid) {
                ret.extend(workflow.process(share_data)?);
            } else {
                return Err(NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Failed to find share data for uuid, but \
                        workflow exists {self:?}"
                    ),
                ));
            }
        }

        let pending_removal_workflow_uuids: Vec<u128> = self
            .workflows
            .values()
            .filter_map(|w| {
                if w.is_expired() || w.is_done() || w.is_fail() {
                    Some(w.uuid)
                } else {
                    None
                }
            })
            .collect();

        for uuid in pending_removal_workflow_uuids {
            if let Some(workflow) = self.workflows.remove(&uuid) {
                if workflow.is_done() {
                    log::debug!("Workflow {workflow} finished");
                } else if workflow.is_expired() {
                    log::debug!("Workflow {workflow} expired");
                }
            }
        }

        Ok(ret)
    }
}
