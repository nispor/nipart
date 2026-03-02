// SPDX-License-Identifier: Apache-2.0

use nipart::NipartError;

use super::{
    super::{commander::NipartCommander, link_event::NipartLinkEvent},
    NipartEventCmd, NipartEventReply, NipartEventWorker,
};
use crate::TaskManager;

// Responsibilities of NipartEventManager:
//  * Redirect NipartLinkEvent sequentially to NipartEventWorker

#[derive(Debug, Clone)]
pub(crate) struct NipartEventManager {
    mgr: TaskManager<NipartEventCmd, NipartEventReply>,
}

impl NipartEventManager {
    pub(crate) async fn new() -> Result<Self, NipartError> {
        Ok(Self {
            mgr: TaskManager::new::<NipartEventWorker>("event").await?,
        })
    }

    pub(crate) async fn set_commander(
        &mut self,
        commander: NipartCommander,
    ) -> Result<(), NipartError> {
        self.mgr
            .exec(NipartEventCmd::SetCommander(Box::new(commander)))
            .await?;
        Ok(())
    }

    pub(crate) async fn handle_event(
        &mut self,
        event: NipartLinkEvent,
    ) -> Result<(), NipartError> {
        self.mgr
            .exec(NipartEventCmd::HandleEvent(Box::new(event)))
            .await?;
        Ok(())
    }
}
