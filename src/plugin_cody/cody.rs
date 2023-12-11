// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use nipart::{
    NipartConnection, NipartError, NipartEvent, NipartPlugin, NipartRole,
};

#[derive(Debug)]
struct NipartPluginCody {
    plugin_name_map: Mutex<HashMap<String, Vec<NipartRole>>>,
    plugin_role_map: Mutex<HashMap<NipartRole, Vec<String>>>,
}

impl NipartPlugin for NipartPluginCody {
    const PLUGIN_NAME: &'static str = "cody";

    fn roles(&self) -> Vec<NipartRole> {
        vec![NipartRole::Commander]
    }

    async fn init() -> Result<Self, NipartError> {
        Ok(Self {
            plugin_name_map: Mutex::new(HashMap::new()),
            plugin_role_map: Mutex::new(HashMap::new()),
        })
    }

    fn handle_event(
        plugin: Arc<Self>,
        connection: &mut NipartConnection,
        event: NipartEvent,
    ) -> impl std::future::Future<Output = Result<Vec<NipartEvent>, NipartError>>
           + Send {
        log::warn!("HAHA Got event {:?}", event);
        async { Ok(vec![]) }
    }
}

#[tokio::main]
async fn main() -> Result<(), NipartError> {
    NipartPluginCody::run().await
}
