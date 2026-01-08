// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicI32, Ordering};

use tokio::sync::{Mutex, MutexGuard};

static CUR_LOCKER_PID: AtomicI32 = AtomicI32::new(0);
static LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Clone)]
pub(crate) struct NipartLockManager;

impl NipartLockManager {
    pub(crate) fn cur_locker_pid() -> Option<i32> {
        let cur_pid = CUR_LOCKER_PID.load(Ordering::Relaxed);
        if cur_pid == 0 { None } else { Some(cur_pid) }
    }

    pub(crate) async fn lock(pid: i32) -> MutexGuard<'static, ()> {
        let ret = LOCK.lock().await;
        CUR_LOCKER_PID.store(pid, Ordering::Relaxed);
        ret
    }
}
