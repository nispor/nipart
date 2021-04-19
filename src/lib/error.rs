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

// Try not implement From for NipartError here unless you are sure this
// error should always convert to certain type of ErrorKind.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    InvalidArgument,
    NipartBug,
    PluginError,
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NipartError {
    pub kind: ErrorKind,
    pub msg: String,
}

impl NipartError {
    pub fn bug(message: String) -> NipartError {
        NipartError {
            kind: ErrorKind::NipartBug,
            msg: message,
        }
    }
    pub fn invalid_argument(message: String) -> NipartError {
        NipartError {
            kind: ErrorKind::InvalidArgument,
            msg: message,
        }
    }
    pub fn plugin_error(message: String) -> NipartError {
        NipartError {
            kind: ErrorKind::PluginError,
            msg: message,
        }
    }
}

impl std::fmt::Display for NipartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for NipartError {
    /* TODO */
}
