// SPDX-License-Identifier: Apache-2.0

use nipart::{
    ErrorKind, NetworkState, NipartError, NipartstateApplyOption,
    NipartstateQueryOption,
};

use super::{NipartPluginCmd, NipartPluginReply, NipartPluginWorker};
use crate::TaskManager;

#[derive(Debug, Clone)]
pub(crate) struct NipartPluginManager {
    mgr: TaskManager<NipartPluginCmd, NipartPluginReply>,
}

impl NipartPluginManager {
    pub(crate) async fn new() -> Result<Self, NipartError> {
        Ok(Self {
            mgr: TaskManager::new::<NipartPluginWorker>("plugin").await?,
        })
    }

    // TODO: Support redirect logs from plugin to user
    pub(crate) async fn query_network_state(
        &mut self,
        opt: NipartstateQueryOption,
    ) -> Result<Vec<NetworkState>, NipartError> {
        let reply = self
            .mgr
            .exec(NipartPluginCmd::QueryNetworkState(Box::new(opt)))
            .await?;
        if let NipartPluginReply::States(s) = reply {
            Ok(s)
        } else {
            Err(NipartError::new(
                ErrorKind::Bug,
                format!(
                    "NipartPluginCmd::QueryNetworkState is not replying with \
                     NipartPluginReply::States, but {reply:?}"
                ),
            ))
        }
    }

    // TODO: Support redirect logs from plugin to user
    pub(crate) async fn apply_network_state(
        &mut self,
        state: &NetworkState,
        opt: &NipartstateApplyOption,
    ) -> Result<(), NipartError> {
        self.mgr
            .exec(NipartPluginCmd::ApplyNetworkState(Box::new((
                state.clone(),
                opt.clone(),
            ))))
            .await?;
        Ok(())
    }
}
