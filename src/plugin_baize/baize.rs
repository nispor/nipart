// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use nipart::{
    NipartConnection, NipartError, NipartEvent, NipartPlugin, NipartRole,
};

#[derive(Debug, Default)]
struct NipartPluginBaiZeShareData {}

impl NipartPluginBaiZeShareData {
    fn _clear(&mut self) {}
}

#[derive(Debug)]
struct NipartPluginBaiZe {
    socket_path: String,
    _data: Mutex<NipartPluginBaiZeShareData>,
}

impl NipartPlugin for NipartPluginBaiZe {
    const PLUGIN_NAME: &'static str = "baize";

    fn get_socket_path(&self) -> &str {
        self.socket_path.as_str()
    }

    fn roles(&self) -> Vec<NipartRole> {
        vec![NipartRole::Monitor]
    }

    async fn init(socket_path: &str) -> Result<Self, NipartError> {
        Ok(Self {
            socket_path: socket_path.to_string(),
            _data: Mutex::new(NipartPluginBaiZeShareData::default()),
        })
    }

    fn handle_event(
        _plugin: Arc<Self>,
        _np_conn: &mut NipartConnection,
        event: NipartEvent,
    ) -> impl std::future::Future<Output = Result<Vec<NipartEvent>, NipartError>>
           + Send {
        log::trace!("Plugin baize got event {:?}", event);
        async move {
            {
                log::warn!("Plugin baize got unknown event {event:?}");
                Ok(Vec::new())
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), NipartError> {
    NipartPluginBaiZe::run().await
}