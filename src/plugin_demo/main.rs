// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use nipart::{NipartError, NipartEvent, NipartExternalPlugin, NipartRole};
use tokio::sync::mpsc::Sender;

#[derive(Debug, Default)]
struct NipartPluginDemoShareData {}

impl NipartPluginDemoShareData {
    fn _clear(&mut self) {}
}

#[derive(Debug)]
struct NipartPluginDemo {
    _data: Mutex<NipartPluginDemoShareData>,
}

impl NipartExternalPlugin for NipartPluginDemo {
    const PLUGIN_NAME: &'static str = "demo";
    const LOG_SUFFIX: &'static str = " (plugin demo)\n";

    fn roles() -> Vec<NipartRole> {
        vec![NipartRole::Monitor]
    }

    async fn init() -> Result<Self, NipartError> {
        Ok(Self {
            _data: Mutex::new(NipartPluginDemoShareData::default()),
        })
    }

    async fn handle_event(
        _plugin: &Arc<Self>,
        _to_daemon: &Sender<NipartEvent>,
        _event: NipartEvent,
    ) -> Result<(), NipartError> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), NipartError> {
    NipartPluginDemo::run().await
}
