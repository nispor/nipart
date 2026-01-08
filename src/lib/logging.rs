// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{ErrorKind, NipartCanIpc, NipartError, NipartIpcConnection};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize,
)]
#[repr(usize)]
#[serde(rename_all = "kebab-case")]
pub enum NipartLogLevel {
    Off = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl NipartLogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

impl From<log::Level> for NipartLogLevel {
    fn from(d: log::Level) -> Self {
        match d {
            log::Level::Error => Self::Error,
            log::Level::Warn => Self::Warn,
            log::Level::Info => Self::Info,
            log::Level::Debug => Self::Debug,
            log::Level::Trace => Self::Trace,
        }
    }
}

impl From<NipartLogLevel> for log::Level {
    fn from(v: NipartLogLevel) -> Self {
        match v {
            NipartLogLevel::Off => Self::Error,
            NipartLogLevel::Error => Self::Error,
            NipartLogLevel::Warn => Self::Warn,
            NipartLogLevel::Info => Self::Info,
            NipartLogLevel::Debug => Self::Debug,
            NipartLogLevel::Trace => Self::Trace,
        }
    }
}

impl From<log::LevelFilter> for NipartLogLevel {
    fn from(d: log::LevelFilter) -> Self {
        match d {
            log::LevelFilter::Off => Self::Off,
            log::LevelFilter::Error => Self::Error,
            log::LevelFilter::Warn => Self::Warn,
            log::LevelFilter::Info => Self::Info,
            log::LevelFilter::Debug => Self::Debug,
            log::LevelFilter::Trace => Self::Trace,
        }
    }
}

impl From<NipartLogLevel> for log::LevelFilter {
    fn from(v: NipartLogLevel) -> Self {
        match v {
            NipartLogLevel::Off => Self::Off,
            NipartLogLevel::Error => Self::Error,
            NipartLogLevel::Warn => Self::Warn,
            NipartLogLevel::Info => Self::Info,
            NipartLogLevel::Debug => Self::Debug,
            NipartLogLevel::Trace => Self::Trace,
        }
    }
}

impl std::fmt::Display for NipartLogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for NipartLogLevel {
    type Err = NipartError;

    fn from_str(s: &str) -> Result<Self, NipartError> {
        match s {
            "off" => Ok(Self::Off),
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            _ => Err(NipartError::new(
                ErrorKind::InvalidLogLevel,
                format!("Invalid logging level {s}"),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct NipartLogEntry {
    pub source: String,
    pub level: NipartLogLevel,
    pub message: String,
}

impl NipartLogEntry {
    pub const IPC_KIND: &'static str = "log";

    pub fn new(source: String, level: NipartLogLevel, message: String) -> Self {
        Self {
            source,
            level,
            message,
        }
    }

    pub fn new_trace(source: String, message: String) -> Self {
        Self {
            level: NipartLogLevel::Trace,
            source,
            message,
        }
    }

    pub fn new_debug(source: String, message: String) -> Self {
        Self {
            level: NipartLogLevel::Debug,
            source,
            message,
        }
    }

    pub fn new_info(source: String, message: String) -> Self {
        Self {
            level: NipartLogLevel::Info,
            source,
            message,
        }
    }

    pub fn new_warn(source: String, message: String) -> Self {
        Self {
            level: NipartLogLevel::Warn,
            source,
            message,
        }
    }

    pub fn new_error(source: String, message: String) -> Self {
        Self {
            level: NipartLogLevel::Error,
            source,
            message,
        }
    }

    pub fn emit(&self) {
        let source = format!("nipart.{}", self.source);
        match self.level {
            NipartLogLevel::Off => (),
            NipartLogLevel::Error => {
                log::error!(target: &source, "{}", self.message)
            }
            NipartLogLevel::Warn => {
                log::warn!(target: &source, "{}", self.message)
            }
            NipartLogLevel::Info => {
                log::info!(target: &source, "{}", self.message)
            }
            NipartLogLevel::Debug => {
                log::debug!(target: &source, "{}", self.message)
            }
            NipartLogLevel::Trace => {
                log::trace!(target: &source, "{}", self.message)
            }
        }
    }
}

impl NipartCanIpc for NipartLogEntry {
    fn ipc_kind(&self) -> String {
        Self::IPC_KIND.to_string()
    }
}

impl NipartIpcConnection {
    /// Emit trace log and also send this log via [NipartIpcConnection] to
    /// remote end(ignore failure of transmission).
    pub async fn log_trace(&mut self, msg: String) {
        log::trace!(target: &self.log_target, "{msg}");
        self.send(Ok(NipartLogEntry {
            source: self.log_target.to_string(),
            level: NipartLogLevel::Trace,
            message: msg,
        }))
        .await
        .ok();
    }

    /// Emit debug log and also send this log via [NipartIpcConnection] to
    /// remote end(ignore failure of transmission).
    pub async fn log_debug(&mut self, msg: String) {
        log::debug!(target: &self.log_target, "{msg}");
        self.send(Ok(NipartLogEntry {
            source: self.log_target.to_string(),
            level: NipartLogLevel::Debug,
            message: msg,
        }))
        .await
        .ok();
    }

    /// Emit info log and also send this log via [NipartIpcConnection] to remote
    /// end(ignore failure of transmission).
    pub async fn log_info(&mut self, msg: String) {
        log::info!(target: &self.log_target, "{msg}");
        self.send(Ok(NipartLogEntry {
            source: self.log_target.to_string(),
            level: NipartLogLevel::Info,
            message: msg,
        }))
        .await
        .ok();
    }

    /// Emit warn log and also send this log via [NipartIpcConnection] to remote
    /// end(ignore failure of transmission).
    pub async fn log_warn(&mut self, msg: String) {
        log::warn!(target: &self.log_target, "{msg}");
        self.send(Ok(NipartLogEntry {
            source: self.log_target.to_string(),
            level: NipartLogLevel::Warn,
            message: msg,
        }))
        .await
        .ok();
    }

    /// Emit warn log and also send this log via [NipartIpcConnection] to remote
    /// end(ignore failure of transmission).
    pub async fn log_error(&mut self, msg: String) {
        log::error!(target: &self.log_target, "{msg}");
        self.send(Ok(NipartLogEntry {
            source: self.log_target.to_string(),
            level: NipartLogLevel::Error,
            message: msg,
        }))
        .await
        .ok();
    }
}
