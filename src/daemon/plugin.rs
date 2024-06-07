// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};
use std::os::unix::fs::PermissionsExt;

use nipart::{
    ErrorKind, NipartConnection, NipartError, NipartEvent, NipartEventAddress,
    NipartLogLevel, NipartNativePlugin, NipartPluginEvent, NipartRole,
    NipartUserEvent,
};
use nipart_plugin_baize::NipartPluginBaize;
use nipart_plugin_mozim::NipartPluginMozim;
use nipart_plugin_nispor::NipartPluginNispor;
use nipart_plugin_sima::NipartPluginSima;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{DEFAULT_TIMEOUT, MPSC_CHANNLE_SIZE};

const PLUGIN_PREFIX: &str = "nipart_plugin_";
const QUERY_PLUGIN_RETRY: usize = 5;
const QUERY_PLUGIN_RETRY_INTERAL: u64 = 500; // milliseconds

pub(crate) type PluginConnections = HashMap<String, PluginConnection>;

#[derive(Debug)]
pub(crate) enum PluginConnection {
    Socket(NipartConnection),
    Mpsc((Sender<NipartEvent>, Receiver<NipartEvent>)),
}

impl PluginConnection {
    pub(crate) async fn recv(&mut self) -> Result<NipartEvent, NipartError> {
        match self {
            Self::Socket(conn) => conn.recv().await,
            Self::Mpsc((_, recver)) => {
                if let Some(event) = recver.recv().await {
                    Ok(event)
                } else {
                    Err(NipartError::new(
                        ErrorKind::Bug,
                        "Native plugin MPSC connection closed".to_string(),
                    ))
                }
            }
        }
    }

    pub(crate) async fn send(
        &mut self,
        event: &NipartEvent,
    ) -> Result<(), NipartError> {
        match self {
            Self::Socket(conn) => conn.send(event).await,
            Self::Mpsc((sender, _)) => Ok(sender.send(event.clone()).await?),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PluginRoles(HashMap<NipartRole, Vec<String>>);

impl PluginRoles {
    pub(crate) fn insert(&mut self, name: &str, roles: Vec<NipartRole>) {
        for role in roles {
            self.0
                .entry(role)
                .and_modify(|roles| roles.push(name.to_string()))
                .or_insert(vec![name.to_string()]);
        }
    }

    pub(crate) fn get(&self, role: NipartRole) -> Option<&[String]> {
        self.0.get(&role).map(|v| v.as_slice())
    }

    pub(crate) fn all_plugin_count(&self) -> usize {
        let mut all_plugins: HashSet<&str> = HashSet::new();
        for plugin_names in self.0.values() {
            for plugin_name in plugin_names {
                all_plugins.insert(plugin_name);
            }
        }
        all_plugins.len()
    }

    pub(crate) fn get_plugin_count(&self, role: NipartRole) -> usize {
        self.0.get(&role).map(|p| p.len()).unwrap_or_default()
    }
}

#[derive(Debug, Default)]
pub(crate) struct Plugins {
    pub(crate) roles: PluginRoles,
    pub(crate) connections: PluginConnections,
}

impl Plugins {
    pub(crate) fn insert(
        &mut self,
        name: &str,
        roles: Vec<NipartRole>,
        connection: PluginConnection,
    ) {
        self.roles.insert(name, roles);
        self.connections.insert(name.to_string(), connection);
    }

    // TODO(Gris): Allow disable plugins
    pub(crate) async fn start() -> Result<Plugins, NipartError> {
        let mut ret = Self::default();
        ret.load_external_plugins().await?;
        ret.load_native_plugins().await?;
        // Check whether we have DHCP plugin loaded.
        ret.get_dhcp_connection_mut()?;
        Ok(ret)
    }

    // Each plugin will be invoked in a thread with a socket path string as its
    // first argument. The plugin should listen on that socket and wait command
    // from switch.
    async fn load_external_plugins(&mut self) -> Result<(), NipartError> {
        for (plugin_exec, plugin_name) in search_external_plugins() {
            let socket_path = format!("{}{}", PLUGIN_PREFIX, plugin_name);
            match external_plugin_start(
                &plugin_exec,
                &plugin_name,
                &socket_path,
            ) {
                Ok(()) => {
                    log::debug!(
                        "Plugin {} started at {}",
                        &plugin_name,
                        &socket_path,
                    );
                    match connect_external_plugin(&plugin_name, &socket_path)
                        .await
                    {
                        Ok((conn, roles)) => {
                            self.insert(&plugin_name, roles, conn);
                        }
                        Err(e) => {
                            log::warn!(
                            "Failed to check plugin role {plugin_name}: {e}. \
                            Ignoring this plugin"
                        );
                        }
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to start plugin {plugin_exec}: {e}. \
                    Ignoring this plugin"
                    );
                }
            }
        }
        Ok(())
    }

    async fn load_native_plugins(&mut self) -> Result<(), NipartError> {
        self.insert(
            "nispor",
            NipartPluginNispor::roles(),
            start_nispor_plugin().await?,
        );
        self.insert(
            "mozim",
            NipartPluginMozim::roles(),
            start_mozim_plugin().await?,
        );
        self.insert(
            "baize",
            NipartPluginBaize::roles(),
            start_baize_plugin().await?,
        );
        self.insert(
            "sima",
            NipartPluginSima::roles(),
            start_sima_plugin().await?,
        );
        Ok(())
    }

    pub(crate) fn get_dhcp_connection_mut(
        &mut self,
    ) -> Result<&mut PluginConnection, NipartError> {
        if let Some(plugin_name) =
            self.roles.get(NipartRole::Dhcp).and_then(|p| p.first())
        {
            if let Some(connection) = self.connections.get_mut(plugin_name) {
                return Ok(connection);
            }
        }

        Err(NipartError::new(
            ErrorKind::Bug,
            "No DHCP plugin found".to_string(),
        ))
    }

    pub(crate) fn get_track_connection_mut(
        &mut self,
    ) -> Result<&mut PluginConnection, NipartError> {
        if let Some(plugin_name) =
            self.roles.get(NipartRole::Track).and_then(|p| p.first())
        {
            if let Some(connection) = self.connections.get_mut(plugin_name) {
                return Ok(connection);
            }
        }

        Err(NipartError::new(
            ErrorKind::Bug,
            "No track plugin found".to_string(),
        ))
    }
}

fn external_plugin_start(
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
            ErrorKind::PluginFailure,
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

fn search_external_plugins() -> Vec<(String, String)> {
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

async fn connect_external_plugin(
    plugin_name: &str,
    plugin_socket: &str,
) -> Result<(PluginConnection, Vec<NipartRole>), NipartError> {
    let mut cur_count = 0usize;
    while cur_count < QUERY_PLUGIN_RETRY {
        let result = get_external_plugin_info(plugin_name, plugin_socket).await;
        match result {
            Ok(i) => return Ok(i),
            Err(e) => {
                if cur_count == QUERY_PLUGIN_RETRY - 1 {
                    return Err(e);
                }
                std::thread::sleep(std::time::Duration::from_millis(
                    QUERY_PLUGIN_RETRY_INTERAL,
                ));
                cur_count += 1;
                continue;
            }
        }
    }
    Err(NipartError::new(
        ErrorKind::Bug,
        "BUG: connect_external_plugin() unreachable".to_string(),
    ))
}

async fn get_external_plugin_info(
    plugin_name: &str,
    plugin_socket: &str,
) -> Result<(PluginConnection, Vec<NipartRole>), NipartError> {
    let event = NipartEvent::new(
        NipartUserEvent::None,
        NipartPluginEvent::QueryPluginInfo,
        NipartEventAddress::Daemon,
        NipartEventAddress::Unicast(plugin_name.to_string()),
        DEFAULT_TIMEOUT,
    );
    let mut np_conn = NipartConnection::new_abstract(plugin_socket)?;
    np_conn.send(&event).await?;
    let reply: NipartEvent = np_conn.recv().await?;
    if let NipartPluginEvent::QueryPluginInfoReply(i) = reply.plugin {
        log::debug!("Got plugin info {i:?}");
        Ok((PluginConnection::Socket(np_conn), i.roles))
    } else {
        Err(NipartError::new(
            ErrorKind::Bug,
            format!("invalid reply {event:?}"),
        ))
    }
}

async fn start_nispor_plugin() -> Result<PluginConnection, NipartError> {
    let (nispor_to_switch_tx, nispor_to_switch_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (switch_to_nispor_tx, switch_to_nispor_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);

    let mut niport_plugin =
        NipartPluginNispor::init(nispor_to_switch_tx, switch_to_nispor_rx)
            .await?;

    tokio::spawn(async move { niport_plugin.run().await });
    log::info!("Native plugin nispor started");
    Ok(PluginConnection::Mpsc((
        switch_to_nispor_tx,
        nispor_to_switch_rx,
    )))
}

async fn start_mozim_plugin() -> Result<PluginConnection, NipartError> {
    let (mozim_to_switch_tx, mozim_to_switch_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (switch_to_mozim_tx, switch_to_mozim_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);

    let mut mozim_plugin =
        NipartPluginMozim::init(mozim_to_switch_tx, switch_to_mozim_rx).await?;

    tokio::spawn(async move { mozim_plugin.run().await });
    log::info!("Native plugin mozim started");
    Ok(PluginConnection::Mpsc((
        switch_to_mozim_tx,
        mozim_to_switch_rx,
    )))
}

async fn start_baize_plugin() -> Result<PluginConnection, NipartError> {
    let (baize_to_switch_tx, baize_to_switch_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (switch_to_baize_tx, switch_to_baize_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);

    let mut baize_plugin =
        NipartPluginBaize::init(baize_to_switch_tx, switch_to_baize_rx).await?;

    tokio::spawn(async move { baize_plugin.run().await });
    log::info!("Native plugin baize started");
    Ok(PluginConnection::Mpsc((
        switch_to_baize_tx,
        baize_to_switch_rx,
    )))
}

async fn start_sima_plugin() -> Result<PluginConnection, NipartError> {
    let (sima_to_switch_tx, sima_to_switch_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);
    let (switch_to_sima_tx, switch_to_sima_rx) =
        tokio::sync::mpsc::channel(MPSC_CHANNLE_SIZE);

    let mut sima_plugin =
        NipartPluginSima::init(sima_to_switch_tx, switch_to_sima_rx).await?;

    tokio::spawn(async move { sima_plugin.run().await });
    log::info!("Native plugin sima started");
    Ok(PluginConnection::Mpsc((
        switch_to_sima_tx,
        sima_to_switch_rx,
    )))
}
