// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, hash_map::Entry};
use std::time::{Duration, SystemTime};

use nipart::{
    ErrorKind, NipartError, NipartEvent, NipartEventAddress, NipartLockEntry,
    NipartLockOption, NipartLogLevel, NipartNativePlugin, NipartPluginEvent,
    NipartRole, NipartUserEvent, NipartUuid,
};
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmithLockOwner {
    pub(crate) uuid: NipartUuid,
    pub(crate) timeout: SystemTime,
}

impl SmithLockOwner {
    pub(crate) fn new(
        uuid: NipartUuid,
        timeout_seconds: u32,
    ) -> Result<Self, NipartError> {
        if let Some(timeout) = SystemTime::now()
            .checked_add(Duration::from_secs(timeout_seconds.into()))
        {
            Ok(Self { uuid, timeout })
        } else {
            Err(NipartError::new(
                ErrorKind::InvalidArgument,
                format!("Overflow caused by lock timeout {timeout_seconds}s"),
            ))
        }
    }
}

#[derive(Debug)]
pub struct NipartPluginSmith {
    log_level: NipartLogLevel,
    to_daemon: Sender<NipartEvent>,
    from_daemon: Receiver<NipartEvent>,
    vault: HashMap<NipartLockEntry, SmithLockOwner>,
}

impl NipartNativePlugin for NipartPluginSmith {
    const PLUGIN_NAME: &'static str = "smith";

    fn get_log_level(&self) -> NipartLogLevel {
        self.log_level
    }

    fn set_log_level(&mut self, level: NipartLogLevel) {
        self.log_level = level;
    }

    async fn init(
        log_level: NipartLogLevel,
        to_daemon: Sender<NipartEvent>,
        from_daemon: Receiver<NipartEvent>,
    ) -> Result<Self, NipartError> {
        Ok(Self {
            log_level,
            to_daemon: to_daemon.clone(),
            from_daemon,
            vault: HashMap::new(),
        })
    }

    fn recver_from_daemon(&mut self) -> &mut Receiver<NipartEvent> {
        &mut self.from_daemon
    }

    fn sender_to_daemon(&self) -> &Sender<NipartEvent> {
        &self.to_daemon
    }

    fn roles() -> Vec<NipartRole> {
        vec![NipartRole::Locker]
    }

    async fn handle_event(
        &mut self,
        event: NipartEvent,
    ) -> Result<(), NipartError> {
        match event.plugin {
            NipartPluginEvent::Lock(lock_entries) => {
                log::trace!("Locking {lock_entries:?}");
                self.lock(*lock_entries, event.uuid)?;
                let mut reply = NipartEvent::new(
                    NipartUserEvent::None,
                    NipartPluginEvent::LockReply,
                    NipartEventAddress::Locker,
                    NipartEventAddress::Commander,
                    event.timeout,
                );
                reply.uuid = event.uuid;
                self.sender_to_daemon().send(reply).await?;
            }
            NipartPluginEvent::Unlock(lock_entries) => {
                log::trace!("Unlocking {lock_entries:?}");
                self.unlock(lock_entries.as_slice(), event.uuid);
                let mut reply = NipartEvent::new(
                    NipartUserEvent::None,
                    NipartPluginEvent::UnlockReply,
                    NipartEventAddress::Locker,
                    NipartEventAddress::Commander,
                    event.timeout,
                );
                reply.uuid = event.uuid;
                self.sender_to_daemon().send(reply).await?;
            }
            _ => log::warn!("Plugin smith got unknown event {event}"),
        }
        Ok(())
    }
}

impl NipartPluginSmith {
    fn lock(
        &mut self,
        lock_entries: Vec<(NipartLockEntry, NipartLockOption)>,
        uuid: NipartUuid,
    ) -> Result<(), NipartError> {
        for (lock_entry, lock_opt) in lock_entries {
            let lock_owner =
                SmithLockOwner::new(uuid, lock_opt.timeout_seconds)?;

            match self.vault.entry(lock_entry.clone()) {
                Entry::Occupied(o) => {
                    let cur_lock_owner = o.into_mut();
                    // Whether current owner expired
                    if cur_lock_owner.timeout >= SystemTime::now() {
                        return Err(NipartError::new(
                            ErrorKind::InvalidArgument,
                            format!(
                                "{lock_entry} is already locked by session {}",
                                cur_lock_owner.uuid
                            ),
                        ));
                    } else {
                        log::debug!("Locking {lock_entry} to session {uuid}");
                        cur_lock_owner.clone_from(&lock_owner);
                    }
                }
                Entry::Vacant(v) => {
                    log::debug!("Locking {lock_entry} to session {uuid}");
                    v.insert(lock_owner);
                }
            }
        }
        Ok(())
    }

    fn unlock(&mut self, lock_entries: &[NipartLockEntry], uuid: NipartUuid) {
        for lock_entry in lock_entries {
            if let Some(cur_owner) = self.vault.get(lock_entry) {
                if cur_owner.uuid == uuid {
                    log::debug!(
                        "Unlocking {lock_entry} owned by session {uuid}"
                    );
                    self.vault.remove(lock_entry);
                } else {
                    // Some entry might be owned by other session after
                    // timeout, which is legal action, hence this is not a
                    // warning or error.
                    log::debug!(
                        "Cannot unlock {lock_entry} on behave of \
                        session {} because it is owned by other session {}",
                        uuid,
                        cur_owner.uuid
                    );
                }
            }
        }
    }
}
