// SPDX-License-Identifier: Apache-2.0

use nipart::{ErrorKind, NetworkState, NipartError, NipartstateInterface};

use super::{NipartConfCmd, NipartConfReply, NipartConfWorker};
use crate::TaskManager;

#[derive(Debug, Clone)]
pub(crate) struct NipartConfManager {
    mgr: TaskManager<NipartConfCmd, NipartConfReply>,
}

impl NipartConfManager {
    pub(crate) async fn new() -> Result<Self, NipartError> {
        Ok(Self {
            mgr: TaskManager::new::<NipartConfWorker>("conf").await?,
        })
    }

    /// Override saved state
    pub(crate) async fn save_state(
        &mut self,
        mut state: NetworkState,
    ) -> Result<(), NipartError> {
        // Should remove interface index
        for iface in state.ifaces.kernel_ifaces.values_mut() {
            iface.base_iface_mut().iface_index = None;
        }

        self.mgr.exec(NipartConfCmd::SaveState(Box::new(state))).await?;
        Ok(())
    }

    pub(crate) async fn query_state(
        &mut self,
    ) -> Result<NetworkState, NipartError> {
        let reply = self.mgr.exec(NipartConfCmd::QueryState).await?;
        if let NipartConfReply::State(s) = reply {
            Ok(*s)
        } else {
            Err(NipartError::new(
                ErrorKind::Bug,
                format!(
                    "NipartConfCmd::Query is not replying with \
                     NipartConfReply::State, but {reply:?}"
                ),
            ))
        }
    }
}
