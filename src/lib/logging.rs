// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{ErrorKind, NipartError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
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
    fn as_str(&self) -> &'static str {
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
                ErrorKind::InvalidArgument,
                format!("Invalid logging level {s}"),
            )),
        }
    }
}
