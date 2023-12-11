// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use nipart::{
    ErrorKind, NipartConnection, NipartConnectionListener, NipartError,
    NipartEvent,
};
use tokio::net::UnixListener;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::MPSC_CHANNLE_SIZE;

pub(crate) async fn start_api_listener(
) -> Result<(Receiver<NipartEvent>, Sender<NipartEvent>), NipartError> {
    let (from_api_thread_tx, mut from_api_thread_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (to_api_thread_tx, mut to_api_thread_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (from_switch_tx, from_switch_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (to_switch_tx, to_switch_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);

    tokio::spawn(async move {
        api_thread(to_api_thread_rx, from_api_thread_tx).await;
    });

    tokio::spawn(async move {
        api_message_exchange_thread(
            to_api_thread_tx,
            from_api_thread_rx,
            to_switch_tx,
            from_switch_rx,
        )
        .await
    });

    Ok((to_switch_rx, from_switch_tx))
}

async fn api_message_exchange_thread(
    to_api_thread_tx: Sender<NipartEvent>,
    mut from_api_thread_rx: Receiver<NipartEvent>,
    to_switch_tx: Sender<NipartEvent>,
    mut from_switch_rx: Receiver<NipartEvent>,
) {
    loop {
        tokio::select! {
            Some(event) = from_api_thread_rx.recv() => {
                if let Err(e) = to_switch_tx.send(event.clone()).await {
                    log::warn!("Failed to forward API user request \
                    {event:?} to daemon");
                }
            }
            Some(event) = from_switch_rx.recv() => {
                if let Err(e) = to_api_thread_tx.send(event.clone()).await {
                    log::warn!("Failed to reply to user {event:?}");
                }
            }
        }
    }
}

async fn api_thread(
    mut to_api_thread_rx: Receiver<NipartEvent>,
    to_switch_tx: Sender<NipartEvent>,
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

    let (from_client_tx, from_client_rx) =
        tokio::sync::mpsc::channel::<NipartEvent>(MPSC_CHANNLE_SIZE);
    loop {
        tokio::select! {
            Ok(np_conn) = listener.accept() => {
                clean_up_tracking_queue(tracking_queue.clone());
                let tracking_queue_clone = tracking_queue.clone();
                let to_switch_tx_clone = to_switch_tx.clone();
                tokio::task::spawn(async move {
                    handle_client(
                        tracking_queue_clone,
                        to_switch_tx_clone,
                        np_conn
                    ).await
                });
            }

            Some(event) = to_api_thread_rx.recv() => {
                // Clean up the queue for dead senders
                clean_up_tracking_queue(tracking_queue.clone());
                // Need to search out the connection for event to send
                let tx = if let Some(ref_uuid) = event.ref_uuid.as_ref() {
                    if let Ok(mut queue) =  tracking_queue.lock() {
                        if let Some(tx) = queue.remove(ref_uuid) {
                            Some(tx)
                        } else {
                            None
                        }
                    } else {None}
                } else {None};
                if let Some(tx) = tx {
                    if let Err(e) = tx.send(event.clone()).await {
                        log::warn!("Failed to reply event to \
                                   user {event:?}") ;
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
    to_switch_tx: Sender<NipartEvent>,
    mut np_conn: NipartConnection,
) {
    let (to_client_tx, mut to_client_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    loop {
        tokio::select! {
            Ok(event) = np_conn.recv::<NipartEvent>() => {
                if let Ok(mut queue) =  tracking_queue.lock() {
                    queue.insert(event.uuid, to_client_tx.clone());
                }
                if let Err(e) = to_switch_tx.send(event.clone()).await {
                    log::warn!(
                        "Failed to redirect user event to daemon \
                        {event:?}: {e}"
                    );
                    break;
                }
            }
            Some(event) = to_client_rx.recv() => {
                if let Err(e) = np_conn.send(&event).await {
                    log::warn!(
                        "Failed to send reply to user {event:?}: {e}"
                    );
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
