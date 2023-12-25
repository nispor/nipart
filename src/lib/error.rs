// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::NipartEvent;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ErrorKind {
    IpcClosed,
    IpcMessageTooLarge,
    InvalidArgument,
    Bug,
    PluginError,
    NotImplementedError,
    DependencyError,
    VerificationError,
    NotSupportedError,
    KernelIntegerRoundedError,
    SrIovVfNotFound,
    Timeout,
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Try not implement From for NipartError here unless you are sure this
// error should always convert to certain type of ErrorKind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct NipartError {
    pub kind: ErrorKind,
    pub msg: String,
}

impl NipartError {
    pub fn new(kind: ErrorKind, msg: String) -> Self {
        Self { kind, msg }
    }
}

impl std::error::Error for NipartError {}

impl std::fmt::Display for NipartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl From<tokio::sync::mpsc::error::SendError<NipartEvent>> for NipartError {
    fn from(e: tokio::sync::mpsc::error::SendError<NipartEvent>) -> Self {
        Self::new(ErrorKind::Bug, format!("std::sync::mpsc::SendError: {e}"))
    }
}

impl From<serde_json::Error> for NipartError {
    fn from(e: serde_json::Error) -> Self {
        Self::new(ErrorKind::Bug, format!("serde_json::Error: {e}"))
    }
}

impl From<std::net::AddrParseError> for NipartError {
    fn from(e: std::net::AddrParseError) -> Self {
        NipartError::new(
            ErrorKind::InvalidArgument,
            format!("Invalid IP address : {e}"),
        )
    }
}
