// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use nipart::{
    NipartConnection, NipartError, NipartEvent, NipartPlugin, NipartRole,
};

#[derive(Debug, Default)]
struct NipartPluginNisporShareData {}

impl NipartPluginNisporShareData {
    fn _clear(&mut self) {}
}

#[derive(Debug)]
struct NipartPluginNispor {
    socket_path: String,
    _data: Mutex<NipartPluginNisporShareData>,
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
            _data: Mutex::new(NipartPluginNisporShareData::default()),
        })
    }

    async fn handle_event(
        _plugin: Arc<Self>,
        _np_conn: &mut NipartConnection,
        event: NipartEvent,
    ) -> Result<Vec<NipartEvent>, NipartError> {
        log::trace!("Plugin nispor got event {:?}", event);
        log::warn!("Plugin nispor got unknown event {event:?}");
        Ok(Vec::new())
    }
}

#[tokio::main]
async fn main() -> Result<(), NipartError> {
    NipartPluginNispor::run().await
}
