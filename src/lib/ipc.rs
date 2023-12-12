// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::os::linux::net::SocketAddrExt;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::{
    ErrorKind, NipartError, NipartEvent, NipartEventAction, NipartEventAddress,
    NipartEventData, NipartNetConfig, NipartNetState, NipartPluginInfo,
    NipartQueryConfigOption, NipartQueryStateOption, NipartRole,
};

#[derive(Debug)]
#[non_exhaustive]
pub struct NipartConnection {
    pub timeout: u64,
    pub path: String,
    pub(crate) socket: UnixStream,
    pub buffer: HashMap<u128, Vec<NipartEvent>>,
}

impl NipartConnection {
    pub const DEFAULT_SOCKET_PATH: &'static str = "/tmp/nipart_socket";
    // Only accept size smaller than 10 MiB
    pub const IPC_MAX_SIZE: usize = 1024 * 1024 * 10;
    const BUFFER_SIZE: usize = 32;
    const DEFAULT_TIMEOUT: u64 = 5000;

    pub async fn new() -> Result<Self, NipartError> {
        Self::new_with_path(Self::DEFAULT_SOCKET_PATH).await
    }

    pub fn set_timeout(&mut self, timeout: u64) {
        self.timeout = timeout;
    }

    pub async fn new_with_path(socket_path: &str) -> Result<Self, NipartError> {
        Ok(Self::new_with_stream(
            socket_path,
            UnixStream::connect(socket_path).await.map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!("Failed to connect socket {}: {}", socket_path, e),
                )
            })?,
        ))
    }

    pub(crate) fn new_with_stream(path: &str, stream: UnixStream) -> Self {
        Self {
            path: path.to_string(),
            socket: stream,
            buffer: HashMap::with_capacity(Self::BUFFER_SIZE),
            timeout: Self::DEFAULT_TIMEOUT,
        }
    }

    pub fn new_abstract(name: &str) -> Result<Self, NipartError> {
        let addr =
            std::os::unix::net::SocketAddr::from_abstract_name(name.as_bytes())
                .map_err(|e| {
                    NipartError::new(
                        ErrorKind::InvalidArgument,
                        format!(
                            "Invalid name for abstract UNIX socket {name}: {e}"
                        ),
                    )
                })?;
        let socket = std::os::unix::net::UnixStream::connect_addr(&addr)
            .map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!("Failed to abstract UNIX socket {name}: {e}"),
                )
            })?;
        socket.set_nonblocking(true).map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!(
                    "Failed to set abstract UNIX socket {name} \
                    as non_blocking: {e}"
                ),
            )
        })?;
        Ok(Self::new_with_stream(
            name,
            UnixStream::from_std(socket).map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Failed to convert std UnixStream {name} to \
                        tokio UnixStream {e}"
                    ),
                )
            })?,
        ))
    }

    pub async fn query_plugin_info(
        &mut self,
    ) -> Result<Vec<NipartPluginInfo>, NipartError> {
        let request = NipartEvent::new(
            NipartEventAction::Request,
            NipartEventData::UserQueryPluginInfo,
            NipartEventAddress::User,
            NipartEventAddress::Daemon,
        );
        self.send(&request).await?;
        for event in self.recv_reply(request.uuid, self.timeout, 0).await? {
            if let NipartEventData::UserQueryPluginInfoReply(i) = event.data {
                return Ok(i);
            }
        }
        Ok(Vec::new())
    }

    pub async fn query_saved_config(
        &mut self,
        option: &NipartQueryConfigOption,
    ) -> Result<NipartNetConfig, NipartError> {
        todo!()
    }

    pub async fn send<T>(&mut self, data: &T) -> Result<(), NipartError>
    where
        T: std::fmt::Debug + Serialize,
    {
        let json_str = serde_json::to_string(data).map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to generate JSON string for {data:?}: {e}",),
            )
        })?;
        let data = json_str.as_bytes();
        let length = &data.len().to_ne_bytes();
        self.socket.write_all(length).await.map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to send data size to UnixStream: {e}",),
            )
        })?;
        self.socket.write_all(&data).await.map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to send data to UnixStream: {e}",),
            )
        })?;
        Ok(())
    }

    /// Since plugins are all working asynchronously, recv() might receive
    /// message we are not interested, this function will only store
    /// irrelevant reply to internal buffer.
    ///
    /// # Returns
    ///  * Function will return when a matching event with
    ///    `NipartEventAction::Done`received.
    ///  * When `expected_count` is set to non-zero, function returns once got
    ///    enough received event.
    ///  * If timeout with any reply received, return `Ok(events)`.
    ///  * If timeout without any reply received, return `Err()` with
    ///    `ErrorKind::Timeout`.
    pub async fn recv_reply(
        &mut self,
        uuid: u128,
        timeout_ms: u64,
        expected_count: usize,
    ) -> Result<Vec<NipartEvent>, NipartError> {
        // Search buffer first, we might already cached it
        let mut events = if let Some(events) = self.buffer.remove(&uuid) {
            if is_reply_enough(events.as_slice(), expected_count) {
                return Ok(events);
            } else {
                events
            }
        } else {
            Vec::new()
        };

        let mut remain_time = Duration::from_millis(timeout_ms);
        while remain_time > Duration::ZERO {
            let now = std::time::Instant::now();
            match tokio::time::timeout(remain_time, self.recv::<NipartEvent>())
                .await
            {
                Ok(Ok(event)) => {
                    let elapsed = now.elapsed();
                    if elapsed >= remain_time {
                        remain_time = Duration::ZERO;
                    } else {
                        remain_time -= elapsed;
                    }
                    if event.ref_uuid == Some(uuid) {
                        events.push(event);
                        if is_reply_enough(events.as_slice(), expected_count) {
                            return Ok(events);
                        }
                    } else {
                        if let Some(ref_uuid) = event.ref_uuid {
                            self.buffer
                                .entry(ref_uuid)
                                .or_insert(Vec::new())
                                .push(event);
                        } else {
                            log::warn!(
                                "Discarding reply event due to \
                                missing ref_uuid: {event:?}"
                            );
                        }
                    }
                }
                Ok(Err(e)) => {
                    let elapsed = now.elapsed();
                    if elapsed >= remain_time {
                        remain_time = Duration::ZERO;
                    } else {
                        remain_time -= elapsed;
                    }
                    log::debug!("Got NipartConnection::recv() error {e}");
                }
                Err(_) => {
                    break;
                }
            }
        }
        if events.is_empty() {
            Err(NipartError::new(
                ErrorKind::Timeout,
                format!("No reply for {uuid} received before time"),
            ))
        } else {
            Ok(events)
        }
    }

    /// This function is for plugin and daemon use only.
    /// Please use [NipartConnection::recv_reply] instead.
    pub async fn recv<T>(&mut self) -> Result<T, NipartError>
    where
        T: serde::de::DeserializeOwned + std::fmt::Debug,
    {
        let mut message_size_bytes = 0usize.to_ne_bytes();
        self.socket
            .read_exact(&mut message_size_bytes)
            .await
            .map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!("Failed to read socket message length: {e}"),
                )
            })?;
        let message_size = usize::from_ne_bytes(message_size_bytes);
        if message_size == 0 {
            return Err(NipartError::new(
                ErrorKind::IpcClosed,
                "The IPC connection is closed by remote".to_string(),
            ));
        }
        if message_size >= Self::IPC_MAX_SIZE {
            return Err(NipartError::new(
                ErrorKind::IpcMessageTooLarge,
                format!(
                    "The size({}) of IPC message exceeded the \
                    maximum support({})",
                    message_size,
                    Self::IPC_MAX_SIZE
                ),
            ));
        }
        let mut buffer = vec![0u8; message_size];

        if let Err(e) = self.socket.read_exact(&mut buffer).await {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                return Err(NipartError::new(
                    ErrorKind::IpcClosed,
                    "IPC connection closed by other end".to_string(),
                ));
            } else {
                return Err(NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Failed to read message to buffer with size {}: {}",
                        message_size, e
                    ),
                ));
            }
        }
        Ok(serde_json::from_slice::<T>(&buffer).map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!(
                    "Failed to convert received [u8] buffer to {}: {e}, \
                    {:?}",
                    std::any::type_name::<T>(),
                    std::str::from_utf8(&buffer),
                ),
            )
        })?)
    }
}

fn is_reply_enough(events: &[NipartEvent], expected_count: usize) -> bool {
    if expected_count != 0 && events.len() == expected_count {
        true
    } else if events.iter().any(|e| e.is_done()) {
        true
    } else {
        false
    }
}
