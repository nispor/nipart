// SPDX-License-Identifier: Apache-2.0

use nipart::{
    NipartError, NipartEvent, NipartEventAddress, NipartLogLevel,
    NipartNativePlugin, NipartPluginEvent, NipartPostStartData, NipartRole,
    NipartUserEvent, NipartUuid,
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::repo::SimaCommitRepo;

#[derive(Debug)]
pub struct NipartPluginSima {
    log_level: NipartLogLevel,
    to_daemon: Sender<NipartEvent>,
    from_daemon: Receiver<NipartEvent>,
    repo: SimaCommitRepo,
}

impl NipartNativePlugin for NipartPluginSima {
    const PLUGIN_NAME: &'static str = "sima";

    fn get_log_level(&self) -> NipartLogLevel {
        self.log_level
    }

    fn set_log_level(&mut self, level: NipartLogLevel) {
        self.log_level = level;
    }

    async fn init(
        log_level: NipartLogLevel,
        to_daemon: Sender<NipartEvent>,
        from_daemon: Receiver<NipartEvent>,
    ) -> Result<Self, NipartError> {
        Ok(Self {
            log_level,
            to_daemon: to_daemon.clone(),
            from_daemon,
            repo: SimaCommitRepo::new()?,
        })
    }

    fn recver_from_daemon(&mut self) -> &mut Receiver<NipartEvent> {
        &mut self.from_daemon
    }

    fn sender_to_daemon(&self) -> &Sender<NipartEvent> {
        &self.to_daemon
    }

    fn roles() -> Vec<NipartRole> {
        vec![NipartRole::Commit]
    }

    async fn handle_plugin_event_post_start(
        &mut self,
        _event_uuid: NipartUuid,
        post_start_data: NipartPostStartData,
    ) -> Result<(), NipartError> {
        self.repo.post_start(post_start_data.current_state)?;
        Ok(())
    }

    async fn handle_event(
        &mut self,
        event: NipartEvent,
    ) -> Result<(), NipartError> {
        match event.plugin {
            NipartPluginEvent::CreateCommit(data) => {
                let (commit, post_state) = *data;
                log::trace!(
                    "Committing {commit:?} with post state {post_state:?}"
                );
                self.repo.store_commit(commit, post_state)?;
                let mut reply = NipartEvent::new(
                    NipartUserEvent::None,
                    NipartPluginEvent::CreateCommitReply,
                    NipartEventAddress::Unicast(Self::PLUGIN_NAME.to_string()),
                    event.src,
                    event.timeout,
                );
                reply.uuid = event.uuid;
                self.sender_to_daemon().send(reply).await?;
            }
            NipartPluginEvent::QueryCommits(opt) => {
                log::trace!("Querying commits with option {opt}");
                let commits = self.repo.query_commits(opt);
                log::trace!("Replying commits {commits:?}");
                let mut reply = NipartEvent::new(
                    NipartUserEvent::None,
                    NipartPluginEvent::QueryCommitsReply(Box::new(commits)),
                    NipartEventAddress::Unicast(Self::PLUGIN_NAME.to_string()),
                    event.src,
                    event.timeout,
                );
                reply.uuid = event.uuid;
                self.sender_to_daemon().send(reply).await?;
            }
            NipartPluginEvent::RemoveCommits(data) => {
                let (uuids, post_state) = *data;
                log::trace!("Removing commits {uuids:?}");
                self.repo.remove_commits(uuids, post_state)?;
                let post_state = self.repo.post_state().clone();

                let mut reply = NipartEvent::new(
                    NipartUserEvent::None,
                    NipartPluginEvent::RemoveCommitsReply(Box::new(post_state)),
                    NipartEventAddress::Unicast(Self::PLUGIN_NAME.to_string()),
                    event.src,
                    event.timeout,
                );
                reply.uuid = event.uuid;
                self.sender_to_daemon().send(reply).await?;
            }
            NipartPluginEvent::QueryLastCommitState => {
                log::trace!("Querying network state after last commit");
                let state = self.repo.post_state().clone();
                log::trace!("Replying {state:?}");
                let mut reply = NipartEvent::new(
                    NipartUserEvent::None,
                    NipartPluginEvent::QueryLastCommitStateReply(Box::new(
                        state,
                    )),
                    NipartEventAddress::Unicast(Self::PLUGIN_NAME.to_string()),
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

// impl NipartPluginSima {}
