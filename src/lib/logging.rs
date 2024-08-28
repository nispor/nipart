// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{
    ErrorKind, NipartError, NipartEvent, NipartEventAddress, NipartPluginEvent,
    NipartUserEvent,
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize,
)]
#[repr(usize)]
#[serde(rename_all = "lowercase")]
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
            Self::Off => "OFF",
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
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
                ErrorKind::InvalidArgument,
                format!("Invalid logging level {s}"),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub struct NipartLogEntry {
    pub level: NipartLogLevel,
    pub message: String,
}

impl NipartLogEntry {
    pub fn new(level: NipartLogLevel, message: String) -> Self {
        Self { level, message }
    }

    pub fn emit_log(&self, source: &str) {
        match self.level {
            NipartLogLevel::Off => (),
            NipartLogLevel::Error => {
                log::error!(target: source, "{}", self.message)
            }
            NipartLogLevel::Warn => {
                log::warn!(target: source, "{}", self.message)
            }
            NipartLogLevel::Info => {
                log::info!(target: source, "{}", self.message)
            }
            NipartLogLevel::Debug => {
                log::debug!(target: source, "{}", self.message)
            }
            NipartLogLevel::Trace => {
                log::trace!(target: source, "{}", self.message)
            }
        }
    }

    pub fn to_event(self, uuid: u128, src: NipartEventAddress) -> NipartEvent {
        NipartEvent::new_with_uuid(
            uuid,
            NipartUserEvent::Log(self),
            NipartPluginEvent::None,
            src,
            NipartEventAddress::User,
            crate::DEFAULT_TIMEOUT,
        )
    }
}
