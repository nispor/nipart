// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use nipart::{
    NipartError, NipartEvent, NipartExternalPlugin, NipartPlugin, NipartRole,
};
use tokio::sync::mpsc::Sender;

#[derive(Debug, Default)]
struct NipartPluginBaiZeShareData {}

impl NipartPluginBaiZeShareData {
    fn _clear(&mut self) {}
}

#[derive(Debug)]
struct NipartPluginBaiZe {
    _data: Mutex<NipartPluginBaiZeShareData>,
}

impl NipartPlugin for NipartPluginBaiZe {
    const PLUGIN_NAME: &'static str = "baize";
    const LOG_SUFFIX: &'static str = " (plugin baize)\n";

    fn roles() -> Vec<NipartRole> {
        vec![NipartRole::Monitor]
    }

    async fn init() -> Result<Self, NipartError> {
        Ok(Self {
            _data: Mutex::new(NipartPluginBaiZeShareData::default()),
        })
    }

    async fn handle_event(
        _plugin: &Arc<Self>,
        _to_daemon: &Sender<NipartEvent>,
        event: NipartEvent,
    ) -> Result<(), NipartError> {
        log::warn!("Plugin baize got unknown event {event:?}");
        Ok(())
    }
}

impl NipartExternalPlugin for NipartPluginBaiZe {}

#[tokio::main]
async fn main() -> Result<(), NipartError> {
    NipartPluginBaiZe::run().await
}
