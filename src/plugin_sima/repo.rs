// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::Path;

use nipart::{
    ErrorKind, InterfaceType, NetworkCommit, NetworkCommitQueryOption,
    NetworkState, NipartError, NipartInterface, NipartUuid,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CommitStore {
    pub(crate) index: u64,
    pub(crate) commit: NetworkCommit,
}

impl CommitStore {
    pub(crate) fn uuid(&self) -> NipartUuid {
        self.commit.uuid
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SimaCommitRepo {
    // In-memory cache of stored commits
    stored_commits: HashMap<NipartUuid, CommitStore>,
    commit_order: Vec<NipartUuid>,
    index: u64,
    managed_ifaces: HashSet<(String, InterfaceType)>,
    post_state: NetworkState,
    applied_state: NetworkState,
}

impl SimaCommitRepo {
    const COMMIT_STORE_PATH: &str = "/var/lib/nipart/commits";
    const POST_STATE_PATH: &str = "/var/lib/nipart/post_apply_state.yml";
    const STATE_STORE_PATH: &str = "/etc/nipart/states";
    const APPLIED_STATE_PATH: &str = "/etc/nipart/states/applied.yml";

    pub(crate) fn new() -> Result<Self, NipartError> {
        // Create required folders and setup permissions
        std::fs::create_dir_all(Self::COMMIT_STORE_PATH).map_err(|e| {
            NipartError::new(
                ErrorKind::PluginFailure,
                format!(
                    "Failed to create folder {}: {e}",
                    Self::COMMIT_STORE_PATH
                ),
            )
        })?;
        std::fs::create_dir_all(Self::STATE_STORE_PATH).map_err(|e| {
            NipartError::new(
                ErrorKind::PluginFailure,
                format!(
                    "Failed to create folder {}: {e}",
                    Self::STATE_STORE_PATH
                ),
            )
        })?;
        let mut ret = Self::default();
        ret.load_commits()?;
        Ok(ret)
    }

    fn next_index(&mut self) -> u64 {
        self.index += 1;
        self.index
    }

    fn load_commits(&mut self) -> Result<(), NipartError> {
        for entry in
            std::fs::read_dir(Self::COMMIT_STORE_PATH).map_err(|e| {
                NipartError::new(
                    ErrorKind::PluginFailure,
                    format!(
                        "Failed to read folder {}: {e}",
                        Self::COMMIT_STORE_PATH
                    ),
                )
            })?
        {
            let entry = match entry {
                Ok(i) => i,
                Err(e) => {
                    log::warn!(
                        "Failed to read dir {}: {e}",
                        Self::COMMIT_STORE_PATH
                    );
                    continue;
                }
            };
            let path = entry.path();

            let commit = match read_commit_from_file(path.as_path()) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!(
                        "Failed to load commit from file {}: {e}",
                        path.display()
                    );
                    continue;
                }
            };
            self.stored_commits.insert(commit.uuid(), commit);
        }
        let mut ordering: Vec<(NipartUuid, u64)> = self
            .stored_commits
            .iter()
            .map(|(k, v)| (*k, v.index))
            .collect();
        ordering.sort_unstable_by_key(|s| s.1);
        self.index = ordering.last().map(|(_, index)| *index).unwrap_or(0);

        self.commit_order =
            ordering.into_iter().map(|(uuid, _)| uuid).collect();
        self.update_applied_state()?;
        Ok(())
    }

    pub(crate) fn post_start(
        &mut self,
        cur_state: NetworkState,
    ) -> Result<(), NipartError> {
        if self.stored_commits.is_empty() {
            let mut init_state = NetworkState::default();
            init_state.description = "Init".to_string();
            let init_commit = NetworkCommit::new(init_state, &cur_state);
            self.store_commit(init_commit, cur_state)?;
        }
        // TODO: Send request to commander to apply desire state in
        // STATE_STORE_PATH
        Ok(())
    }

    pub(crate) fn post_state(&self) -> &NetworkState {
        &self.post_state
    }

    fn update_applied_state(&mut self) -> Result<(), NipartError> {
        let mut net_state = NetworkState::default();
        for uuid in self.commit_order.as_slice() {
            if let Some(commit_store) = self.stored_commits.get(uuid) {
                net_state.merge(&commit_store.commit.desired_state)?;
            }
        }

        let state_yml = serde_yaml::to_string(&net_state).map_err(|e| {
            NipartError::new(
                ErrorKind::PluginFailure,
                format!("Failed to convert NetworkState to string, error: {e}"),
            )
        })?;
        self.applied_state = net_state;

        let mut fd = std::fs::OpenOptions::new()
            .read(false)
            .write(true)
            .truncate(true)
            .create(true)
            .open(Self::APPLIED_STATE_PATH)
            .map_err(|e| {
                NipartError::new(
                    ErrorKind::PluginFailure,
                    format!(
                        "Failed to write commit file {}, error: {e}",
                        Self::APPLIED_STATE_PATH
                    ),
                )
            })?;
        fd.write(state_yml.as_bytes()).map_err(|e| {
            NipartError::new(
                ErrorKind::PluginFailure,
                format!(
                    "Failed to write apply state file {}, error: {e}",
                    Self::APPLIED_STATE_PATH
                ),
            )
        })?;
        Ok(())
    }

    // TODO
    pub(crate) fn _get_managed_ifaces(
        &self,
    ) -> impl Iterator<Item = &(String, InterfaceType)> {
        self.managed_ifaces.iter()
    }

    fn store_post_apply_state(
        &mut self,
        state: NetworkState,
    ) -> Result<(), NipartError> {
        let state_yml = serde_yaml::to_string(&state).map_err(|e| {
            NipartError::new(
                ErrorKind::PluginFailure,
                format!("Failed to convert commit to string, error: {e}"),
            )
        })?;
        let mut fd = std::fs::OpenOptions::new()
            .read(false)
            .write(true)
            .truncate(true)
            .create(true)
            .open(Self::POST_STATE_PATH)
            .map_err(|e| {
                NipartError::new(
                    ErrorKind::PluginFailure,
                    format!(
                        "Failed to write post apply state file {}, error: {e}",
                        Self::POST_STATE_PATH
                    ),
                )
            })?;

        fd.write(state_yml.as_bytes()).map_err(|e| {
            NipartError::new(
                ErrorKind::PluginFailure,
                format!(
                    "Failed to write post apply state file {}, error: {e}",
                    Self::POST_STATE_PATH
                ),
            )
        })?;

        self.post_state = state;
        Ok(())
    }

    pub(crate) fn store_commit(
        &mut self,
        commit: NetworkCommit,
        post_state: NetworkState,
    ) -> Result<(), NipartError> {
        let store = CommitStore {
            commit,
            index: self.next_index(),
        };

        write_commit_to_file(&store)?;

        for iface in store.commit.desired_state.ifaces.iter() {
            if iface.is_absent() {
                self.managed_ifaces.remove(&(
                    iface.name().to_string(),
                    iface.iface_type().clone(),
                ));
            } else {
                self.managed_ifaces.insert((
                    iface.name().to_string(),
                    iface.iface_type().clone(),
                ));
            }
        }

        self.commit_order.push(store.uuid());
        self.stored_commits.insert(store.uuid(), store);
        self.store_post_apply_state(post_state)?;
        self.update_applied_state()?;

        Ok(())
    }

    pub(crate) fn query_commits(
        &self,
        opt: NetworkCommitQueryOption,
    ) -> Vec<NetworkCommit> {
        let mut ret: Vec<NetworkCommit> = Vec::new();
        let all_len = self.commit_order.len();
        let need_count = if opt.count == 0 || (!opt.uuids.is_empty()) {
            all_len
        } else {
            opt.count as usize
        };
        for uuid in
            self.commit_order.as_slice()[all_len - need_count..all_len].iter()
        {
            if opt.uuids.is_empty() || opt.uuids.contains(uuid) {
                if let Some(commit) = self.stored_commits.get(uuid) {
                    ret.push(commit.commit.clone())
                }
            }
        }
        ret
    }

    pub(crate) fn remove_commits(
        &mut self,
        uuids: Vec<NipartUuid>,
        post_state: NetworkState,
    ) -> Result<NetworkState, NipartError> {
        let new_commit_order: Vec<NipartUuid> = self
            .commit_order
            .clone()
            .into_iter()
            .filter(|uuid| !uuids.contains(uuid))
            .collect();

        self.commit_order = new_commit_order;
        for uuid in &uuids {
            self.stored_commits.remove(uuid);
        }

        self.store_post_apply_state(post_state)?;
        self.update_applied_state()?;
        Ok(self.applied_state.clone())
    }
}

fn read_commit_from_file(file_path: &Path) -> Result<CommitStore, NipartError> {
    let content: String = std::fs::read_to_string(file_path).map_err(|e| {
        NipartError::new(
            ErrorKind::PluginFailure,
            format!("Failed to read file {}: {e}", file_path.display()),
        )
    })?;
    let commit: CommitStore = serde_yaml::from_str(&content).map_err(|e| {
        NipartError::new(
            ErrorKind::PluginFailure,
            format!(
                "Corrupted commit file {}, content: {content}, error: {e}",
                file_path.display()
            ),
        )
    })?;
    Ok(commit)
}

fn write_commit_to_file(commit: &CommitStore) -> Result<(), NipartError> {
    let file_path = format!(
        "{}/{}.yml",
        SimaCommitRepo::COMMIT_STORE_PATH,
        commit.uuid()
    );
    let commit_yml = serde_yaml::to_string(&commit).map_err(|e| {
        NipartError::new(
            ErrorKind::PluginFailure,
            format!("Failed to convert commit to string, error: {e}"),
        )
    })?;

    let mut fd = std::fs::OpenOptions::new()
        .read(false)
        .write(true)
        .truncate(true)
        .create(true)
        .open(&file_path)
        .map_err(|e| {
            NipartError::new(
                ErrorKind::PluginFailure,
                format!("Failed to write commit file {file_path}, error: {e}"),
            )
        })?;

    fd.write(commit_yml.as_bytes()).map_err(|e| {
        NipartError::new(
            ErrorKind::PluginFailure,
            format!("Failed to write commit file {file_path}, error: {e}"),
        )
    })?;
    Ok(())
}
