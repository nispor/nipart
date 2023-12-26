// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use nipart::{
    MergedNetworkState, NipartConnection, NipartError, NipartEvent,
    NipartEventAction, NipartEventAddress, NipartPlugin, NipartPluginEvent,
    NipartRole, NipartUserEvent,
};

use crate::{nispor_apply, nispor_retrieve};

const STATE_PRIORITY: u32 = 50;

#[derive(Debug, Default)]
struct NipartPluginNisporShareData {}

impl NipartPluginNisporShareData {
    fn _clear(&mut self) {}
}

#[derive(Debug)]
pub(crate) struct NipartPluginNispor {
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
        event: NipartEvent,
    ) -> Result<Option<NipartEvent>, NipartError> {
        log::trace!("Plugin nispor got event {:?}", event);
        match event.plugin {
            NipartPluginEvent::QueryNetState(_) => {
                let state = nispor_retrieve(false).await?;
                let mut reply = NipartEvent::new(
                    NipartEventAction::Done,
                    NipartUserEvent::None,
                    NipartPluginEvent::QueryNetStateReply(
                        Box::new(state),
                        STATE_PRIORITY,
                    ),
                    NipartEventAddress::Unicast(Self::PLUGIN_NAME.to_string()),
                    NipartEventAddress::Commander,
                );
                reply.ref_uuid = Some(event.uuid);
                Ok(Some(reply))
            }
            NipartPluginEvent::ApplyNetState(net_states, _opt) => {
                let (desire_state, current_state) = *net_states;
                let merged_state = MergedNetworkState::new(
                    desire_state,
                    current_state,
                    false,
                    false,
                )?;
                nispor_apply(&merged_state).await?;
                let mut reply = NipartEvent::new(
                    NipartEventAction::Done,
                    NipartUserEvent::None,
                    NipartPluginEvent::ApplyNetStateReply,
                    NipartEventAddress::Unicast(Self::PLUGIN_NAME.to_string()),
                    NipartEventAddress::Commander,
                );
                reply.ref_uuid = Some(event.uuid);
                Ok(Some(reply))
            }
            _ => {
                log::warn!("Plugin nispor got unknown event {event:?}");
                Ok(None)
            }
        }
    }
}
