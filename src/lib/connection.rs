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

use serde::{Deserialize, Serialize};

use crate::{merge_yaml_mappings, NipartError};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct NipartConnection {
    pub uuid: Option<String>,
    pub name: Option<String>,
    pub config: String,
    // More option like auto_connect, auto_connect_ports, volatile and etc
}

impl NipartConnection {
    pub fn new(config: String) -> Self {
        NipartConnection {
            uuid: None,
            name: None,
            config: config,
        }
    }
    pub fn merge_from(
        &mut self,
        nip_cons: &[NipartConnection],
    ) -> Result<(), NipartError> {
        let mut conf_strs = Vec::new();
        conf_strs.push(self.config.as_str());
        let self_name = match &self.name {
            Some(n) => n,
            None => {
                return Err(NipartError::bug(format!(
                    "Self NipartConnection has no name: {:?}",
                    self
                )))
            }
        };
        let self_uuid = match &self.uuid {
            Some(u) => u,
            None => {
                return Err(NipartError::bug(format!(
                    "Self NipartConnection has no uuid: {:?}",
                    self
                )))
            }
        };
        for nip_con in nip_cons {
            if let Some(name) = &nip_con.name {
                if name != self_name {
                    return Err(NipartError::plugin_error(format!(
                        "WARN: NipartConnection to merge is holding \
                        different name: origin {}, to merge {:?}",
                        &self_name, &nip_con.name
                    )));
                }
            }

            if let Some(uuid) = &nip_con.uuid {
                if uuid != self_uuid {
                    return Err(NipartError::plugin_error(format!(
                        "WARN: NipartConnection to merge is holding \
                        different uuid: origin {}, to merge {:?}",
                        &self_uuid, &nip_con.uuid
                    )));
                }
            }

            conf_strs.push(nip_con.config.as_str());
        }
        self.config = merge_yaml_mappings(conf_strs.as_slice())?;
        Ok(())
    }
}
