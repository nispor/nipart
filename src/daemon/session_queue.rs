// SPDX-License-Identifier: Apache-2.0

// TODO: We might have problem when laptop suspend, maybe CLOCK_BOOTTIME ?
use std::collections::HashMap;
use std::time::SystemTime;

use nipart::{ErrorKind, NipartError, NipartEvent};

const QUEUE_INIT_CAPACITY: usize = 1024;

#[derive(Debug, Clone)]
pub(crate) struct Session {
    pub(crate) request: NipartEvent,
    pub(crate) expected_reply_count: usize,
    timeout: SystemTime,
    pub(crate) replies: Vec<NipartEvent>,
}

impl Session {
    pub(crate) fn is_expired(&self) -> bool {
        SystemTime::now() >= self.timeout && !self.is_done()
    }

    pub(crate) fn is_done(&self) -> bool {
        self.replies.len() >= self.expected_reply_count
    }
}

pub(crate) struct SessionQueue {
    data: HashMap<u128, Session>,
}

impl SessionQueue {
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
        log::trace!(
            "SessionQueue::new_session: {request:?}, \
            expected_reply_count {expected_reply_count}, timeout {timeout}"
        );
        self.data.insert(
            uuid,
            Session {
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
            } else {
                log::info!(
                    "SessionQueue::push() got event which session is \
                    already timeout or unregistered"
                );
            }
            Ok(())
        } else {
            Err(NipartError::new(
                ErrorKind::Bug,
                format!(
                    "SessionQueue::push got event without ref_uuid \
                    {event:?}"
                ),
            ))
        }
    }

    pub(crate) fn get_timeout_or_finished(
        &mut self,
    ) -> Result<Vec<Session>, NipartError> {
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
