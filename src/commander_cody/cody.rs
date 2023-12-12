// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use nipart::{
    ErrorKind, NipartConnection, NipartError, NipartEvent, NipartEventAction,
    NipartEventAddress, NipartEventData, NipartPlugin, NipartPluginInfo,
    NipartRole,
};

const TIMEOUT: u64 = 5000;

#[derive(Debug, Default)]
struct NipartPluginCodyShareData {
    plugin_name_map: HashMap<String, Vec<NipartRole>>,
    plugin_role_map: HashMap<NipartRole, Vec<String>>,
    all_except_me: Vec<String>,
}

impl NipartPluginCodyShareData {
    fn clear(&mut self) {
        *self = Default::default();
    }
}

#[derive(Debug)]
struct NipartPluginCody {
    socket_path: String,
    data: Mutex<NipartPluginCodyShareData>,
}

impl NipartPlugin for NipartPluginCody {
    const PLUGIN_NAME: &'static str = "cody";

    fn get_socket_path(&self) -> &str {
        self.socket_path.as_str()
    }

    fn roles(&self) -> Vec<NipartRole> {
        vec![NipartRole::Commander]
    }

    async fn init(socket_path: &str) -> Result<Self, NipartError> {
        Ok(Self {
            socket_path: socket_path.to_string(),
            data: Mutex::new(NipartPluginCodyShareData::default()),
        })
    }

    fn handle_event(
        plugin: Arc<Self>,
        np_conn: &mut NipartConnection,
        event: NipartEvent,
    ) -> impl std::future::Future<Output = Result<Vec<NipartEvent>, NipartError>>
           + Send {
        log::debug!("Commander Cody got event {:?}", event);
        async move {
            match event.data {
                NipartEventData::UpdateAllPluginInfo(infos) => {
                    handle_update_plugin_infos(plugin, infos)
                }

                NipartEventData::UserQueryPluginInfo => {
                    handle_plugin_info_query(plugin, np_conn, &event).await
                }
                _ => {
                    log::warn!("Commander Cody got unknown event {event:?}");
                    Ok(Vec::new())
                }
            }
        }
    }
}

async fn handle_plugin_info_query(
    plugin: Arc<NipartPluginCody>,
    np_conn: &mut NipartConnection,
    event: &NipartEvent,
) -> Result<Vec<NipartEvent>, NipartError> {
    // This line is required to convince compiler this function is Send as
    // we will not using Mutex inside of await.
    let plugin_count = {
        if let Ok(data) = plugin.data.lock() {
            data.all_except_me.len()
        } else {
            0
        }
    };
    let mut plugin_infos = Vec::new();

    if plugin_count != 0 {
        log::debug!("Sending QueryPluginInfo to {plugin_count} plugins");
        let mut request = NipartEvent::new(
            NipartEventAction::Request,
            NipartEventData::QueryPluginInfo,
            NipartEventAddress::Commander,
            NipartEventAddress::AllPluginNoCommander,
        );
        np_conn.send(&request).await?;
        let replies = np_conn
            .recv_reply(request.uuid, TIMEOUT, plugin_count)
            .await?;
        for reply in replies {
            if let NipartEventData::QueryPluginInfoReply(i) = reply.data {
                plugin_infos.push(i);
            }
        }
    }
    plugin_infos.push(plugin.get_plugin_info());

    let mut reply_event = NipartEvent::new(
        NipartEventAction::Done,
        NipartEventData::UserQueryPluginInfoReply(plugin_infos),
        NipartEventAddress::Daemon,
        NipartEventAddress::User,
    );
    reply_event.ref_uuid = Some(event.uuid);

    Ok(vec![reply_event])
}

fn handle_update_plugin_infos(
    plugin: Arc<NipartPluginCody>,
    plugin_infos: Vec<NipartPluginInfo>,
) -> Result<Vec<NipartEvent>, NipartError> {
    let mut data = plugin.data.lock().map_err(map_lock_err)?;
    data.clear();
    for plugin_info in plugin_infos {
        if plugin_info.roles.contains(&NipartRole::Commander) {
            continue;
        }
        let name = plugin_info.name.to_string();
        data.plugin_name_map
            .insert(name.clone(), plugin_info.roles.clone());
        for role in plugin_info.roles {
            data.plugin_role_map
                .entry(role)
                .or_insert(Vec::new())
                .push(name.clone())
        }
        data.all_except_me.push(name.clone());
    }

    Ok(Vec::new())
}

fn map_lock_err<T>(e: PoisonError<T>) -> NipartError {
    NipartError::new(
        ErrorKind::Bug,
        format!("Failed to lock the NipartPluginCodyShareData: {e}"),
    )
}

#[tokio::main]
async fn main() -> Result<(), NipartError> {
    NipartPluginCody::run().await
}
