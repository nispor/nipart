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
struct NipartPluginNisporShareData {}

impl NipartPluginNisporShareData {
    fn clear(&mut self) {}
}

#[derive(Debug)]
struct NipartPluginNispor {
    socket_path: String,
    data: Mutex<NipartPluginNisporShareData>,
}

impl NipartPlugin for NipartPluginNispor {
    const PLUGIN_NAME: &'static str = "nispor";

    fn get_socket_path(&self) -> &str {
        self.socket_path.as_str()
    }

    fn roles(&self) -> Vec<NipartRole> {
        vec![NipartRole::Kernel]
    }

    async fn init(socket_path: &str) -> Result<Self, NipartError> {
        Ok(Self {
            socket_path: socket_path.to_string(),
            data: Mutex::new(NipartPluginNisporShareData::default()),
        })
    }

    fn handle_event(
        plugin: Arc<Self>,
        np_conn: &mut NipartConnection,
        event: NipartEvent,
    ) -> impl std::future::Future<Output = Result<Vec<NipartEvent>, NipartError>>
           + Send {
        log::debug!("Plugin nispor got event {:?}", event);
        async move {
            match event.data {
                _ => {
                    log::warn!("Plugin nispor got unknown event {event:?}");
                    Ok(Vec::new())
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), NipartError> {
    NipartPluginNispor::run().await
}
