// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::os::linux::net::SocketAddrExt;
use std::time::Duration;

use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::{
    ErrorKind, NetworkCommit, NetworkState, NipartApplyOption, NipartError,
    NipartEvent, NipartEventAddress, NipartPluginEvent, NipartPluginInfo,
    NipartQueryOption, NipartUserEvent, NipartUuid,
};

pub const DEFAULT_TIMEOUT: u32 = 30000;

#[derive(Debug)]
#[non_exhaustive]
pub struct NipartConnection {
    pub timeout: u32,
    pub path: String,
    pub(crate) socket: UnixStream,
    pub buffer: HashMap<NipartUuid, NipartEvent>,
}

impl NipartConnection {
    pub const DEFAULT_SOCKET_PATH: &'static str = "/tmp/nipart_socket";
    // Only accept size smaller than 10 MiB
    pub const IPC_MAX_SIZE: usize = 1024 * 1024 * 10;
    const EVENT_BUFFER_SIZE: usize = 1024;

    pub async fn new() -> Result<Self, NipartError> {
        Self::new_with_path(Self::DEFAULT_SOCKET_PATH).await
    }

    pub fn set_timeout(&mut self, timeout: u32) {
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
            buffer: HashMap::with_capacity(Self::EVENT_BUFFER_SIZE),
            timeout: DEFAULT_TIMEOUT,
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
            NipartUserEvent::QueryPluginInfo,
            NipartPluginEvent::None,
            NipartEventAddress::User,
            NipartEventAddress::Daemon,
            self.timeout,
        );
        self.send(&request).await?;
        let event = self.recv_reply(request.uuid, self.timeout).await?;

        if let NipartUserEvent::QueryPluginInfoReply(i) = event.user {
            Ok(i)
        } else {
            Err(NipartError::new(
                ErrorKind::Bug,
                format!("Invalid reply {event:?} for QueryPluginInfo"),
            ))
        }
    }

    pub async fn query_net_state(
        &mut self,
        option: NipartQueryOption,
    ) -> Result<NetworkState, NipartError> {
        let request = NipartEvent::new(
            NipartUserEvent::QueryNetState(option),
            NipartPluginEvent::None,
            NipartEventAddress::User,
            NipartEventAddress::Daemon,
            self.timeout,
        );
        self.send(&request).await?;
        let event = self.recv_reply(request.uuid, self.timeout).await?;
        if let NipartUserEvent::QueryNetStateReply(s) = event.user {
            Ok(*s)
        } else {
            Err(NipartError::new(
                ErrorKind::Bug,
                format!("Invalid reply {event:?} for QueryNetState"),
            ))
        }
    }

    pub async fn apply_net_state(
        &mut self,
        state: NetworkState,
        option: NipartApplyOption,
    ) -> Result<Option<NetworkCommit>, NipartError> {
        let request = NipartEvent::new(
            NipartUserEvent::ApplyNetState(Box::new(state), option),
            NipartPluginEvent::None,
            NipartEventAddress::User,
            NipartEventAddress::Daemon,
            self.timeout,
        );
        self.send(&request).await?;
        let event = self.recv_reply(request.uuid, self.timeout).await?;
        if let NipartUserEvent::ApplyNetStateReply(commit) = event.user {
            Ok(*commit)
        } else {
            Err(NipartError::new(
                ErrorKind::Bug,
                format!("Invalid reply {event:?} for ApplyNetState"),
            ))
        }
    }

    pub async fn stop_daemon(&mut self) -> Result<(), NipartError> {
        let request = NipartEvent::new(
            NipartUserEvent::Quit,
            NipartPluginEvent::None,
            NipartEventAddress::User,
            NipartEventAddress::Daemon,
            self.timeout,
        );
        self.send(&request).await?;
        Ok(())
    }

    pub async fn send<T>(&mut self, data: &T) -> Result<(), NipartError>
    where
        T: std::fmt::Debug + Serialize,
    {
        log::trace!("Sending {data:?}");
        let json_str = serde_json::to_string(data).map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to generate JSON string for {data:?}: {e}",),
            )
        })?;
        let data = json_str.as_bytes();
        let length = &data.len().to_ne_bytes();
        self.socket.write_all(length).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                NipartError::new(
                    ErrorKind::IpcClosed,
                    "Connection closed".to_string(),
                )
            } else {
                NipartError::new(
                    ErrorKind::Bug,
                    format!("Failed to send data size to UnixStream: {e}",),
                )
            }
        })?;
        self.socket.write_all(data).await.map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to send data to UnixStream: {e}",),
            )
        })?;
        Ok(())
    }

    /// Since daemon is working asynchronously and client might send multiple
    /// requests via single connection, recv() might receive message we are not
    /// interested, this function will only store irrelevant reply to internal
    /// buffer.
    /// When multiple requests invoked on single connection, the log will
    /// be mixed.
    ///
    /// # Returns
    ///  * Function will return when a matching event received.
    ///  * If timeout without any reply received, return `Err()` with
    ///    `ErrorKind::Timeout`.
    pub async fn recv_reply(
        &mut self,
        uuid: NipartUuid,
        timeout_ms: u32,
    ) -> Result<NipartEvent, NipartError> {
        if let Some(event) = self.buffer.remove(&uuid) {
            event.into_result()
        } else {
            let mut remain_time = Duration::from_millis(timeout_ms.into());
            while remain_time > Duration::ZERO {
                let now = std::time::Instant::now();
                match tokio::time::timeout(
                    remain_time,
                    self.recv::<NipartEvent>(),
                )
                .await
                {
                    Ok(Ok(event)) => {
                        if event.is_log() {
                            event.emit_log();
                            continue;
                        }
                        let elapsed = now.elapsed();
                        if elapsed >= remain_time {
                            remain_time = Duration::ZERO;
                        } else {
                            remain_time -= elapsed;
                        }
                        if event.uuid == uuid {
                            return event.into_result();
                        } else {
                            self.buffer.insert(event.uuid, event);
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
            Err(NipartError::new(
                ErrorKind::Timeout,
                format!("Timeout on waiting reply for event {uuid}"),
            ))
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
        let ret = serde_json::from_slice::<T>(&buffer).map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!(
                    "Failed to convert received [u8] buffer to {}: {e}, \
                    {:?}",
                    std::any::type_name::<T>(),
                    std::str::from_utf8(&buffer),
                ),
            )
        });
        if let Ok(ref t) = ret {
            log::trace!("Received {t:?}");
        }
        ret
    }
}
