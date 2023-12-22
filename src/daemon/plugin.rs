// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;

use nipart::{
    ErrorKind, NipartError, NipartLogLevel, NipartPluginInfo, NipartRole,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct Plugins {
    data: HashMap<NipartRole, Vec<String>>,
}

impl Plugins {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn len(&self) -> usize {
        self.data.len()
    }

    pub(crate) fn push(&mut self, plugin_info: NipartPluginInfo) {
        for role in plugin_info.roles {
            self.data
                .entry(role)
                .and_modify(|n| n.push(plugin_info.name.clone()))
                .or_insert(vec![plugin_info.name.clone()]);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.data.clear();
    }

    pub(crate) fn get_plugin_count_with_role(&self, role: NipartRole) -> usize {
        self.data.get(&role).map(|r| r.len()).unwrap_or_default()
    }
}

const PLUGIN_PREFIX: &str = "nipart_plugin_";

// Each plugin will be invoked in a thread with a socket path string as its
// first argument. The plugin should listen on that socket and wait command
// from plugin.
//
pub(crate) fn load_plugins() -> Vec<(String, String)> {
    let mut socket_paths = Vec::new();
    for (plugin_exec, plugin_name) in search_plugins() {
        let socket_path = format!("{}{}", PLUGIN_PREFIX, plugin_name);
        match plugin_start(&plugin_exec, &plugin_name, &socket_path) {
            Ok(()) => {
                log::debug!(
                    "Plugin {} started at {}",
                    &plugin_name,
                    &socket_path,
                );
                socket_paths.push((plugin_name, socket_path));
            }
            Err(e) => {
                log::error!("Failed to start plugin {plugin_exec}: {e}");
                continue;
            }
        }
    }
    socket_paths
}

fn plugin_start(
    plugin_exec_path: &str,
    _plugin_name: &str,
    socket_path: &str,
) -> Result<(), NipartError> {
    // Invoke the plugin in child.
    match std::process::Command::new(plugin_exec_path)
        .arg(socket_path)
        .arg(NipartLogLevel::from(log::max_level()).as_str())
        .spawn()
    {
        Ok(_) => {
            log::debug!(
                "Plugin {} started at {}",
                plugin_exec_path,
                &socket_path
            );
            Ok(())
        }
        Err(e) => Err(NipartError::new(
            ErrorKind::PluginError,
            format!(
                "Failed to start plugin {} {}: {}",
                plugin_exec_path, &socket_path, e
            ),
        )),
    }
}

fn is_executable(file_path: &str) -> bool {
    if let Ok(attr) = std::fs::metadata(file_path) {
        attr.permissions().mode() & 0o100 != 0
    } else {
        false
    }
}

fn get_current_exec_folder() -> String {
    if let Ok(mut exec_path) = std::env::current_exe() {
        exec_path.pop();
        if let Some(dir_path) = exec_path.to_str() {
            return dir_path.into();
        }
    }
    "/usr/bin".into()
}

fn search_plugins() -> Vec<(String, String)> {
    let mut ret = Vec::new();
    let search_folder = match std::env::var("NIPART_PLUGIN_FOLDER") {
        Ok(d) => d,
        Err(_) => get_current_exec_folder(),
    };
    log::debug!("Searching plugin at {}", search_folder);
    if let Ok(dir) = std::fs::read_dir(&search_folder) {
        for entry in dir {
            if let Ok(file_name) = entry.map(|e| e.file_name()) {
                let file_name = match file_name.to_str() {
                    Some(i) => i,
                    None => continue,
                };
                if file_name.starts_with(PLUGIN_PREFIX) {
                    let plugin_exec_path =
                        format!("{}/{}", &search_folder, file_name);
                    if !is_executable(&plugin_exec_path) {
                        continue;
                    }
                    let plugin_name =
                        match file_name.strip_prefix(PLUGIN_PREFIX) {
                            Some(n) => n,
                            None => {
                                log::error!(
                                    "file_name {} not started with {}",
                                    file_name,
                                    PLUGIN_PREFIX,
                                );
                                continue;
                            }
                        };
                    log::info!(
                        "Found plugin {plugin_name} at {plugin_exec_path}"
                    );
                    ret.push((plugin_exec_path, plugin_name.to_string()));
                }
            }
        }
    }
    if ret.is_empty() {
        log::error!("No plugin found in {search_folder}");
    }
    ret
}
