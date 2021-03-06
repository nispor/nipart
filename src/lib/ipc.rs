//    Copyright 2021 Red Hat, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fs::remove_file;

use serde::{Deserialize, Serialize};
use serde_yaml;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

use crate::{
    ErrorKind, NipartError, NipartPluginInfo, NipartPluginIpcMessage,
    NipartState,
};

const DEFAULT_SOCKET_PATH: &str = "/tmp/nipart_socket";
const IPC_SAFE_SIZE: usize = 1024 * 1024 * 10; // 10 MiB

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
pub struct NipartQueryOption {}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
pub struct NipartApplyOption {}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum NipartIpcMessage {
    Error(NipartError),
    QueryPluginInfo,
    QueryPluginInfoReply(NipartPluginInfo),
    QueryState(NipartQueryOption),
    QueryStateReply(NipartState),
    ApplyState(NipartState, NipartApplyOption),
    ApplyStateReply,
    ConnectionClosed,
    Plugin(NipartPluginIpcMessage),
    None,
}

impl NipartIpcMessage {
    pub fn from_result(result: Result<Self, NipartError>) -> Self {
        match result {
            Ok(i) => i,
            Err(e) => NipartIpcMessage::Error(e),
        }
    }
}

pub fn ipc_bind() -> Result<UnixListener, NipartError> {
    ipc_bind_with_path(DEFAULT_SOCKET_PATH)
}

pub fn ipc_bind_with_path(
    socket_path: &str,
) -> Result<UnixListener, NipartError> {
    remove_file(socket_path).ok();
    match UnixListener::bind(socket_path) {
        Err(e) => Err(NipartError::new(
            ErrorKind::Bug,
            format!("Failed to bind socket {}: {}", socket_path, e),
        )),
        Ok(l) => Ok(l),
    }
}

pub async fn ipc_connect() -> Result<UnixStream, NipartError> {
    ipc_connect_with_path(DEFAULT_SOCKET_PATH).await
}

pub async fn ipc_connect_with_path(
    socket_path: &str,
) -> Result<UnixStream, NipartError> {
    match UnixStream::connect(socket_path).await {
        Err(e) => Err(NipartError::new(
            ErrorKind::Bug,
            format!("Failed to connect socket {}: {}", socket_path, e),
        )),
        Ok(l) => Ok(l),
    }
}

pub async fn ipc_send(
    stream: &mut UnixStream,
    message: &NipartIpcMessage,
) -> Result<(), NipartError> {
    let message_string = match serde_yaml::to_string(message) {
        Ok(s) => s,
        Err(e) => {
            let e = NipartError::new(
                ErrorKind::InvalidArgument,
                format!(
                    "Invalid IPC message - failed to serialize {:?}: {}",
                    &message, e
                ),
            );
            log::error!("{}", e);
            return Err(e);
        }
    };
    let message_bytes = message_string.as_bytes();
    if let Err(e) = stream.write_u32(message_bytes.len() as u32).await {
        let e = NipartError::new(
            ErrorKind::Bug,
            format!(
                "Failed to write message size {} to socket: {}",
                message_bytes.len(),
                e
            ),
        );
        log::error!("{}", e);
        return Err(e);
    };
    if let Err(e) = stream.write_all(message_bytes).await {
        let e = NipartError::new(
            ErrorKind::Bug,
            format!("Failed to write message to socket: {}", e),
        );
        log::error!("{}", e);
        Err(e)
    } else {
        Ok(())
    }
}

async fn ipc_recv_get_size(
    stream: &mut UnixStream,
) -> Result<usize, NipartError> {
    match stream.read_u32().await {
        Err(e) => {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                return Ok(0); // connection closed.
            } else {
                // TODO: Handle the client closed the connection.
                return Err(NipartError::new(
                    ErrorKind::Bug,
                    format!("Failed to read message size: {:?}", e),
                ));
            }
        }
        Ok(s) => Ok(s as usize),
    }
}

async fn ipc_recv_get_data(
    stream: &mut UnixStream,
    message_size: usize,
) -> Result<NipartIpcMessage, NipartError> {
    let mut buffer = vec![0u8; message_size];

    if let Err(e) = stream.read_exact(&mut buffer).await {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            return Ok(NipartIpcMessage::ConnectionClosed);
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
    match serde_yaml::from_slice::<NipartIpcMessage>(&buffer) {
        Err(e) => Err(NipartError::new(
            ErrorKind::Bug,
            format!("Invalid message recieved: {:?}: {}", buffer, e),
        )),
        Ok(NipartIpcMessage::Error(e)) => Err(e.clone()),
        Ok(m) => Ok(m),
    }
}

pub async fn ipc_recv(
    stream: &mut UnixStream,
) -> Result<NipartIpcMessage, NipartError> {
    let message_size = ipc_recv_get_size(stream).await?;
    if message_size == 0 {
        return Ok(NipartIpcMessage::ConnectionClosed);
    }
    ipc_recv_get_data(stream, message_size).await
}

// Return error if data size exceed IPC_SAFE_SIZE
// Normally used by daemon where client can not be trusted.
pub async fn ipc_recv_safe(
    stream: &mut UnixStream,
) -> Result<NipartIpcMessage, NipartError> {
    let message_size = ipc_recv_get_size(stream).await?;
    if message_size == 0 {
        return Ok(NipartIpcMessage::ConnectionClosed);
    }
    if message_size > IPC_SAFE_SIZE {
        return Err(NipartError::new(
            ErrorKind::InvalidArgument,
            format!(
                "Invalid IPC message: message size execeed the limit({})",
                IPC_SAFE_SIZE
            ),
        ));
    }
    ipc_recv_get_data(stream, message_size).await
}

pub async fn ipc_exec(
    stream: &mut UnixStream,
    message: &NipartIpcMessage,
) -> Result<NipartIpcMessage, NipartError> {
    ipc_send(stream, message).await?;
    ipc_recv(stream).await
}
