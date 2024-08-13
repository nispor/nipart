// SPDX-License-Identifier: Apache-2.0

use gix::ThreadSafeRepository;
use nipart::{
    NipartError, NipartEvent, NipartEventAddress, NipartNativePlugin,
    NipartPluginEvent, NipartRole, NipartUserEvent,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::repo::load_config_repo;

#[derive(Debug)]
pub struct NipartPluginSima {
    to_daemon: Sender<NipartEvent>,
    from_daemon: Receiver<NipartEvent>,
    pub(crate) config_repo: ThreadSafeRepository,
}

impl NipartNativePlugin for NipartPluginSima {
    const PLUGIN_NAME: &'static str = "sima";

    async fn init(
        to_daemon: Sender<NipartEvent>,
        from_daemon: Receiver<NipartEvent>,
    ) -> Result<Self, NipartError> {
        Ok(Self {
            to_daemon: to_daemon.clone(),
            from_daemon,
            config_repo: load_config_repo()?,
        })
    }

    fn recver_from_daemon(&mut self) -> &mut Receiver<NipartEvent> {
        &mut self.from_daemon
    }

    fn sender_to_daemon(&self) -> &Sender<NipartEvent> {
        &self.to_daemon
    }

    fn roles() -> Vec<NipartRole> {
        vec![NipartRole::Track]
    }

    async fn handle_event(
        &mut self,
        event: NipartEvent,
    ) -> Result<(), NipartError> {
        match event.plugin {
            NipartPluginEvent::Commit(state) => {
                log::trace!("Committing NetworkState {state:?}");
                self.commit(*state)?;
                let mut reply = NipartEvent::new(
                    NipartUserEvent::None,
                    NipartPluginEvent::CommitReply,
                    NipartEventAddress::Track,
                    event.src,
                    event.timeout,
                );
                reply.uuid = event.uuid;
                self.sender_to_daemon().send(reply).await?;
            }
            NipartPluginEvent::QueryCommits(opt) => {
                log::trace!("Querying commits with option {opt}");
                let commits = self.query_commits(&opt)?;
                log::trace!("Replying commits {commits:?}");
                let mut reply = NipartEvent::new(
                    NipartUserEvent::None,
                    NipartPluginEvent::QueryCommitsReply(Box::new(commits)),
                    NipartEventAddress::Track,
                    event.src,
                    event.timeout,
                );
                reply.uuid = event.uuid;
                self.sender_to_daemon().send(reply).await?;
            }
            _ => log::warn!("Plugin sima got unknown event {event}"),
        }
        Ok(())
    }
}
