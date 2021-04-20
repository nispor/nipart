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

use futures::future::join_all;
use serde::{Deserialize, Serialize};

use crate::{
    ipc_connect_with_path, ipc_recv, ipc_send, NipartError, NipartIpcMessage,
};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NipartPluginCapacity {
    NetQuery,         // For querying network status
    NetApply,         // For applying network config
    ConfigFileManage, // For managing network config files
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct NipartPluginInfo {
    pub name: String,
    pub socket_path: String,
    pub capacities: Vec<NipartPluginCapacity>,
}

impl NipartPluginInfo {
    pub fn new(name: &str, capacities: Vec<NipartPluginCapacity>) -> Self {
        NipartPluginInfo {
            name: name.into(),
            socket_path: "".into(),
            capacities: capacities,
        }
    }
}

pub async fn ipc_plugin_exec(
    plugin_info: &NipartPluginInfo,
    ipc_msg: &NipartIpcMessage,
) -> Result<NipartIpcMessage, NipartError> {
    let mut stream = ipc_connect_with_path(&plugin_info.socket_path).await?;
    ipc_send(&mut stream, ipc_msg).await?;
    // TODO: Handle timeout
    ipc_recv(&mut stream).await
}

//TODO: save plugin name in return also, so we know which plugin to blame.
pub async fn ipc_plugins_exec(
    ipc_msg: &NipartIpcMessage,
    plugins: &[NipartPluginInfo],
    capacity: &NipartPluginCapacity,
) -> Vec<NipartIpcMessage> {
    let mut replys_async = Vec::new();
    for plugin_info in plugins {
        if plugin_info.capacities.contains(capacity) {
            replys_async.push(ipc_plugin_exec(plugin_info, &ipc_msg));
        }
    }
    let replys = join_all(replys_async).await;

    let mut reply_msgs = Vec::new();
    for reply in replys {
        match reply {
            Ok(r) => reply_msgs.push(r),
            Err(e) => {
                eprintln!("WARN: got error from plugin: {:?}", e);
            }
        }
    }
    reply_msgs
}
