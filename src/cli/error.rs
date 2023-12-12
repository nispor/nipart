// SPDX-License-Identifier: Apache-2.0

use nipart::NipartError;

#[derive(Clone, Debug)]
pub(crate) struct CliError {
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
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        Self {
            msg: format!("std::io::Error: {}", e),
        }
    }
}

impl From<NipartError> for CliError {
    fn from(e: NipartError) -> Self {
        Self {
            msg: format!("NipartError: {}", e),
        }
    }
}

impl From<&str> for CliError {
    fn from(msg: &str) -> Self {
        Self {
            msg: msg.to_string(),
        }
    }
}

impl From<String> for CliError {
    fn from(msg: String) -> Self {
        Self { msg }
    }
}
