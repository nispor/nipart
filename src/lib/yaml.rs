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

use serde_yaml;

use crate::NipartError;

// Only merge top level keys
pub fn merge_yaml_mappings(yml_strs: &[&str]) -> Result<String, NipartError> {
    let mut full_obj = serde_yaml::Mapping::new();

    for yml_str in yml_strs {
        let cur_obj: serde_yaml::Value = match serde_yaml::from_str(yml_str) {
            Ok(i) => i,
            Err(e) => {
                return Err(NipartError::plugin_error(format!(
                    "Invalid format of YAML reply from plugin: {}, {}",
                    yml_str, e
                )))
            }
        };
        match cur_obj.as_mapping() {
            Some(cur) => {
                for (key, value) in cur.iter() {
                    if full_obj.contains_key(key) {
                        let old_value = &full_obj[key];
                        if old_value != value {
                            return Err(NipartError::plugin_error(format!(
                                "Duplicate key: {:?} new: {:?}, old: {:?}",
                                key, value, old_value
                            )));
                        }
                    } else {
                        full_obj.insert(key.clone(), value.clone());
                    }
                }
            }
            None => {
                return Err(NipartError::plugin_error(format!(
                    "WARN: {:?} is not mapping",
                    cur_obj
                )));
            }
        }
    }

    match serde_yaml::to_string(&full_obj) {
        Ok(s) => Ok(s),
        Err(e) => Err(NipartError::bug(format!(
            "This should never happen: \
            Failed to convert serde_yaml::Mapping to string: {:?} {}",
            &full_obj, e
        ))),
    }
}

pub fn merge_yaml_lists(yml_strs: &[&str]) -> Result<String, NipartError> {
    let mut full_obj = serde_yaml::Sequence::new();

    for yml_str in yml_strs {
        let cur_obj: serde_yaml::Value = match serde_yaml::from_str(yml_str) {
            Ok(i) => i,
            Err(e) => {
                return Err(NipartError::plugin_error(format!(
                    "Invalid format of YAML reply from plugin: {}, {}",
                    yml_str, e
                )))
            }
        };
        match cur_obj.as_sequence() {
            Some(cur) => {
                for item in cur.iter() {
                    full_obj.push(item.clone());
                }
            }
            None => {
                return Err(NipartError::plugin_error(format!(
                    "WARN: {:?} is not sequence/list",
                    cur_obj
                )));
            }
        }
    }

    match serde_yaml::to_string(&full_obj) {
        Ok(s) => Ok(s),
        Err(e) => Err(NipartError::bug(format!(
            "Failed to convert serde_yaml::Sequence to string: {:?} {}",
            &full_obj, e
        ))),
    }
}
