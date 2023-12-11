// SPDX-License-Identifier: Apache-2.0

use std::os::linux::net::SocketAddrExt;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::{
    ErrorKind, NipartError, NipartEvent, NipartNetConfig, NipartNetState,
    NipartQueryConfigOption, NipartQueryStateOption, NipartRole,
};

#[derive(Debug)]
#[non_exhaustive]
pub struct NipartConnection {
    pub path: String,
    pub(crate) socket: UnixStream,
}

impl NipartConnection {
    pub const DEFAULT_SOCKET_PATH: &'static str = "/tmp/nipart_socket";
    pub const IPC_SAFE_SIZE: usize = 1024 * 1024 * 10; // 10 MiB

    pub async fn new() -> Result<Self, NipartError> {
        Self::new_with_path(Self::DEFAULT_SOCKET_PATH).await
    }

    pub async fn new_with_path(socket_path: &str) -> Result<Self, NipartError> {
        Ok(Self {
            path: socket_path.to_string(),
            socket: UnixStream::connect(socket_path).await.map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!("Failed to connect socket {}: {}", socket_path, e),
                )
            })?,
        })
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
        Ok(Self {
            path: name.to_string(),
            socket: UnixStream::from_std(socket).map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Failed to convert std UnixStream {name} to \
                        tokio UnixStream {e}"
                    ),
                )
            })?,
        })
    }

    pub async fn query_running_state(
        &mut self,
        option: &NipartQueryStateOption,
    ) -> Result<NipartNetState, NipartError> {
        todo!()
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
        if message_size >= Self::IPC_SAFE_SIZE {
            return Err(NipartError::new(
                ErrorKind::IpcMessageTooLarge,
                format!(
                    "The size({}) of IPC message exceeded the \
                    maximum support({})",
                    message_size,
                    Self::IPC_SAFE_SIZE
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
