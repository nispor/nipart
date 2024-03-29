// SPDX-License-Identifier: Apache-2.0

// SPDX-License-Identifier: Apache-2.0

use std::fs::remove_file;
use std::os::linux::net::SocketAddrExt;

use tokio::net::UnixListener;

use crate::{ErrorKind, NipartConnection, NipartError};

#[derive(Debug)]
#[non_exhaustive]
pub struct NipartConnectionListener {
    path: String,
    socket: UnixListener,
}

impl NipartConnectionListener {
    pub fn new(path: &str) -> Result<Self, NipartError> {
        remove_file(path).ok();
        Ok(Self {
            path: path.to_string(),
            socket: UnixListener::bind(path).map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!("Failed to bind UnixListener {e}"),
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
        let socket = std::os::unix::net::UnixListener::bind_addr(&addr)
            .map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!("Failed to bind abstract UNIX socket {name}: {e}"),
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
            socket: UnixListener::from_std(socket).map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!(
                        "Failed to convert abstract UNIX socket {name} to \
                        tokio UnixListener: {e}"
                    ),
                )
            })?,
        })
    }

    pub async fn accept(&self) -> Result<NipartConnection, NipartError> {
        let (stream, addr) = self.socket.accept().await.map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to accept socket connection {e}"),
            )
        })?;
        log::trace!("Accepted Unix socket({}) connection", self.path,);
        Ok(NipartConnection::new_with_stream(
            format!("{:?}", addr).as_str(),
            stream,
        ))
    }
}
