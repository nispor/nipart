//    Copyright 2021-2022 Red Hat, Inc.
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

use async_trait::async_trait;

use nipart::{NipartError, NipartPlugin, NipartPluginCapacity, NipartState};

#[derive(Debug)]
struct NipartPluginConnMan {}

#[async_trait]
impl NipartPlugin for NipartPluginConnMan {
    fn name() -> &'static str {
        "connman"
    }

    fn capacities() -> Vec<NipartPluginCapacity> {
        vec![NipartPluginCapacity::Config]
    }

    fn save_config(_state: &NipartState) -> Result<(), NipartError> {
        log::warn!("save_config() not implemented, ignoring");
        Ok(())
    }
}

#[tokio::main()]
async fn main() {
    NipartPluginConnMan::run().await
}
