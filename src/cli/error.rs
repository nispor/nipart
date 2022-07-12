//    Copyright 2022 Red Hat, Inc.
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

use nipart::NipartError;

#[derive(Clone, Debug)]
pub(crate) struct CliError {
    msg: String,
}

impl CliError {
    pub(crate) fn new(msg: String) -> Self {
        Self { msg }
    }
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
