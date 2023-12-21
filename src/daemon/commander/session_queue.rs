// SPDX-License-Identifier: Apache-2.0

// TODO: We might have problem when laptop suspend, maybe CLOCK_BOOTTIME ?
use std::collections::HashMap;
use std::time::SystemTime;

use nipart::{ErrorKind, NipartError, NipartEvent};

const QUEUE_INIT_CAPACITY: usize = 1024;

#[derive(Debug, Clone)]
pub(crate) struct CommanderSession {
    pub(crate) request: NipartEvent,
    expected_reply_count: usize,
    timeout: SystemTime,
    pub(crate) replies: Vec<NipartEvent>,
}

impl CommanderSession {
    pub(crate) fn is_expired(&self) -> bool {
        SystemTime::now() >= self.timeout
    }

    pub(crate) fn is_done(&self) -> bool {
        self.replies.len() >= self.expected_reply_count
    }
}

pub(crate) struct CommanderSessionQueue {
    data: HashMap<u128, CommanderSession>,
}

impl CommanderSessionQueue {
    pub(crate) fn new() -> Self {
        Self {
            data: HashMap::with_capacity(QUEUE_INIT_CAPACITY),
        }
    }

    pub(crate) fn new_session(
        &mut self,
        uuid: u128,
        request: NipartEvent,
        expected_reply_count: usize,
        timeout: u64,
    ) {
        self.data.insert(
            uuid,
            CommanderSession {
                request,
                expected_reply_count,
                timeout: SystemTime::now()
                    .checked_add(std::time::Duration::from_millis(timeout))
                    .expect("SystemTime overflow"),
                replies: Vec::new(),
            },
        );
    }

    // Append
    pub(crate) fn push(
        &mut self,
        event: NipartEvent,
    ) -> Result<(), NipartError> {
        if let Some(uuid) = event.ref_uuid {
            if let Some(session) = self.data.get_mut(&uuid) {
                session.replies.push(event);
                Ok(())
            } else {
                Err(NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "CommanderSessionQueue::push got event does not have
                        session registered before {event:?}"
                    ),
                ))
            }
        } else {
            Err(NipartError::new(
                ErrorKind::Bug,
                format!(
                    "CommanderSessionQueue::push got event without ref_uuid \
                    {event:?}"
                ),
            ))
        }
    }

    // Return Some(Vec<Event>) when for Event timeout or reach
    // expected_reply_count
    pub(crate) fn get_timeout_or_finished(
        &mut self,
    ) -> Result<Vec<CommanderSession>, NipartError> {
        let uuids_to_remove: Vec<u128> = self
            .data
            .iter()
            .filter_map(|(k, v)| {
                if v.is_expired() || v.is_done() {
                    Some(*k)
                } else {
                    None
                }
            })
            .collect();

        let mut ret = Vec::new();
        for uuid in uuids_to_remove {
            if let Some(s) = self.data.remove(&uuid) {
                ret.push(s);
            }
        }
        Ok(ret)
    }
}
