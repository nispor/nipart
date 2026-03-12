// SPDX-License-Identifier: Apache-2.0

use nipart::NipartError;

#[derive(Clone, Debug, Default)]
pub(crate) struct CliError {
    pub(crate) nipart_error: Option<NipartError>,
    msg: String,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for CliError {
    fn description(&self) -> &str {
        &self.msg
    }
}

impl From<serde_yaml::Error> for CliError {
    fn from(e: serde_yaml::Error) -> Self {
        Self {
            msg: format!("serde_yaml::Error: {}", e),
            ..Default::default()
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        Self {
            msg: format!("std::io::Error: {}", e),
            ..Default::default()
        }
    }
}

impl From<NipartError> for CliError {
    fn from(e: NipartError) -> Self {
        let msg = format!("NipartError: {}: {}", e.kind, e.msg);
        Self {
            nipart_error: Some(e),
            msg,
        }
    }
}

impl From<&str> for CliError {
    fn from(msg: &str) -> Self {
        Self {
            msg: msg.to_string(),
            ..Default::default()
        }
    }
}

impl From<String> for CliError {
    fn from(msg: String) -> Self {
        Self {
            msg,
            ..Default::default()
        }
    }
}
