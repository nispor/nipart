// SPDX-License-Identifier: Apache-2.0

use std::fs::remove_file;

use tokio::net::UnixListener;

use crate::{ErrorKind, NipartError, NipartIpcConnection};

#[derive(Debug)]
pub struct NipartIpcListener {
    path: String,
    socket: UnixListener,
}

impl NipartIpcListener {
    pub fn new(path: &str) -> Result<Self, NipartError> {
        remove_file(path).ok();

        let dir_path = match std::path::Path::new(path).parent() {
            Some(d) => d,
            None => {
                return Err(NipartError::new(
                    ErrorKind::IpcFailure,
                    format!("Failed to find folder path of {path}"),
                ));
            }
        };

        if !dir_path.exists() {
            std::fs::create_dir_all(dir_path).map_err(|e| {
                NipartError::new(
                    ErrorKind::IpcFailure,
                    format!("Failed to create dir {}: {e}", dir_path.display()),
                )
            })?;
        }

        Ok(Self {
            path: path.to_string(),
            socket: UnixListener::bind(path).map_err(|e| {
                NipartError::new(
                    ErrorKind::IpcFailure,
                    format!("Failed to bind UnixListener to {path}: {e}"),
                )
            })?,
        })
    }

    pub async fn accept(&self) -> Result<NipartIpcConnection, NipartError> {
        let (stream, _) = self.socket.accept().await.map_err(|e| {
            NipartError::new(
                ErrorKind::IpcFailure,
                format!("Failed to accept socket connection {e}"),
            )
        })?;
        log::trace!("Accepted Unix socket({}) connection", self.path);
        Ok(NipartIpcConnection::new_with_stream(
            stream, "daemon", "client",
        ))
    }
}
