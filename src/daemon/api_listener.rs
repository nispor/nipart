// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use nipart::{
    ErrorKind, NipartConnection, NipartConnectionListener, NipartError,
    NipartEvent, NipartEventAddress,
};

use tokio::sync::mpsc::{Receiver, Sender};

use crate::MPSC_CHANNLE_SIZE;

// Each user API connection has a tokio spawn, then collect NipartEvent and
// sent to switch.
// For data from switch to user, we use ref_uuid to find the correct UnixStream
// to reply.
pub(crate) async fn start_api_listener_thread(
) -> Result<(Receiver<NipartEvent>, Sender<NipartEvent>), NipartError> {
    let (switch_to_user_tx, switch_to_user_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (user_to_switch_tx, user_to_switch_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);

    tokio::spawn(async move {
        api_thread(switch_to_user_rx, user_to_switch_tx).await;
    });

    Ok((user_to_switch_rx, switch_to_user_tx))
}

async fn api_thread(
    mut switch_to_user: Receiver<NipartEvent>,
    user_to_switch: Sender<NipartEvent>,
) {
    let listener = match NipartConnectionListener::new(
        NipartConnection::DEFAULT_SOCKET_PATH,
    ) {
        Ok(l) => l,
        Err(e) => {
            log::error!("Failed to start API listener thread {e}");
            return;
        }
    };

    let tracking_queue: Arc<Mutex<BTreeMap<u128, Sender<NipartEvent>>>> =
        Arc::new(Mutex::new(BTreeMap::new()));

    loop {
        tokio::select! {
            Ok(np_conn) = listener.accept() => {
                clean_up_tracking_queue(tracking_queue.clone());
                let tracking_queue_clone = tracking_queue.clone();
                let user_to_switch_clone = user_to_switch.clone();
                tokio::task::spawn(async move {
                    handle_client(
                        tracking_queue_clone,
                        user_to_switch_clone,
                        np_conn
                    ).await
                });
            }

            Some(event) = switch_to_user.recv() => {
                log::trace!("api_thread(): to user {:?}", event);
                // Clean up the queue for dead senders
                clean_up_tracking_queue(tracking_queue.clone());
                // Need to search out the connection for event to send
                let tx = if let Some(ref_uuid) = event.ref_uuid.as_ref() {
                    if let Ok(mut queue) =  tracking_queue.lock() {
                        queue.remove(ref_uuid)
                    } else {None}
                } else {None};
                if let Some(tx) = tx {
                    if let Err(e) = tx.send(event.clone()).await {
                        log::warn!("Failed to reply event to user {e}") ;
                    }
                } else {
                    log::warn!("Discarding event without ref_uuid {event:?}");
                }
            }
        }
    }
}

async fn handle_client(
    tracking_queue: Arc<Mutex<BTreeMap<u128, Sender<NipartEvent>>>>,
    use_to_switch: Sender<NipartEvent>,
    mut np_conn: NipartConnection,
) {
    let (switch_to_user_tx, mut switch_to_user_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    loop {
        tokio::select! {
            Ok(mut event) = np_conn.recv::<NipartEvent>() => {
                log::trace!("handle_client(): from user {event:?}");
                // Redirect user request to Commander
                event.dst = NipartEventAddress::Commander;
                if let Ok(mut queue) =  tracking_queue.lock() {
                    queue.insert(event.uuid, switch_to_user_tx.clone());
                }
                if let Err(e) = use_to_switch.send(event.clone()).await {
                    log::warn!(
                        "Failed to send user event to switch \
                        {event:?}: {e}"
                    );
                    break;
                }
            }
            Some(event) = switch_to_user_rx.recv() => {
                log::trace!("handle_client(): to user {event:?}");
                if let Err(e) = np_conn.send(&event).await {
                    if e.kind == ErrorKind::IpcClosed {
                        log::info!(
                            "Discard event {} {:?} as user disconnected",
                            event.uuid, event.user
                        );
                    } else {
                        log::warn!(
                            "Failed to send reply to user {event:?}: {e}"
                        );
                    }
                    break;
                }
            }
        }
    }
}

fn clean_up_tracking_queue(
    tracking_queue: Arc<Mutex<BTreeMap<u128, Sender<NipartEvent>>>>,
) {
    let mut pending_changes: Vec<u128> = Vec::new();
    match tracking_queue.lock() {
        Ok(queue) => {
            for (uuid, tx) in queue.iter() {
                if tx.is_closed() {
                    pending_changes.push(*uuid);
                }
            }
        }
        Err(e) => {
            log::warn!("Failed to acquire lock on tracking_queue: {e}");
        }
    }
    if let Ok(mut queue) = tracking_queue.lock() {
        for uuid in pending_changes {
            log::debug!(
                "Removing tracking event {uuid} as client connection dropped"
            );
            queue.remove(&uuid);
        }
    }
}
