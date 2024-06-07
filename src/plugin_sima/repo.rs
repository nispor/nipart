// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, SystemTime};

use gix::ThreadSafeRepository;
use nipart::{
    ErrorKind, NetworkCommit, NetworkCommitQueryOption, NetworkState,
    NipartError,
};

use crate::{state::flatten_net_state, NipartPluginSima};

const ETC_REPO_PATH: &'static str = "/etc/nipart/states";

const GIT_USER_NAME: &str = "Gris Ge";
const GIT_USER_EMAIL: &str = "fge@redhat.com";

impl NipartPluginSima {
    pub(crate) fn commit(
        &mut self,
        state: NetworkState,
    ) -> Result<(), NipartError> {
        log::trace!("Plugin sima: Committing {state:?}");
        let new_state =
            if let Some(cur_etc_commit) = self.get_saved_current()? {
                let mut pre_state = cur_etc_commit.state;
                pre_state.merge_desire(&state);
                pre_state
            } else {
                state
            };
        let mut states = flatten_net_state(new_state);
        let mut local_repo = self.config_repo.to_thread_local();
        let head = get_git_head(&local_repo)?.detach();
        let tmp_repo = gen_git_snapshot_mut(&mut local_repo)?;
        let mut files = Vec::new();
        let mut tree = gix::objs::Tree::empty();

        for (name, state) in states.drain() {
            let file_name = format!("{name}.yml");
            files.push(file_name.clone());
            let yaml_content = serde_yaml::to_string(&state).map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!("Failed to generate YAML for {state:?}: {e}"),
                )
            })?;
            let blob_id =
                tmp_repo.write_blob(yaml_content.as_bytes()).map_err(|e| {
                    NipartError::new(
                        ErrorKind::Bug,
                        format!("Failed to create git blob: {e}"),
                    )
                })?;
            let state_entry = gix::objs::tree::Entry {
                mode: gix::objs::tree::EntryKind::Blob.into(),
                oid: blob_id.into(),
                filename: file_name.into(),
            };
            tree.entries.push(state_entry);
        }
        tree.entries.sort_unstable();
        let tree_id = tmp_repo.write_object(&tree).map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to git write_object: {e}"),
            )
        })?;
        let new_head_id = tmp_repo
            .commit(
                "HEAD",
                "PLACE HOLDER for commit comment",
                tree_id,
                [head.id],
            )
            .map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!("Failed to git commit: {e}"),
                )
            })?;

        // TODO: There is no good way to commit on disk files in gix yet.
        //       invoking git command to checkout
        std::process::Command::new("git")
            .arg("reset")
            .arg("--hard")
            .arg(new_head_id.to_string())
            .current_dir(ETC_REPO_PATH)
            .output()
            .map_err(|e| {
                NipartError::new(
                    ErrorKind::Bug,
                    format!("Failed to do git reset --hard {new_head_id}: {e}"),
                )
            })?;

        Ok(())
    }

    pub(crate) fn get_saved_current(
        &self,
    ) -> Result<Option<NetworkCommit>, NipartError> {
        let mut opt = NetworkCommitQueryOption::default();
        opt.count = 1;
        opt.persisted_only = true;
        self.query_commits(&opt)
            .map(|commits| commits.get(0).cloned())
    }

    pub(crate) fn query_commit(
        &self,
        commit_id: gix::Object,
    ) -> Option<NetworkCommit> {
        log::debug!("Retrieving NetworkState from commit {}", commit_id.id);
        let mut state = NetworkState::default();
        let mut ret = NetworkCommit::default();
        let repo = self.config_repo.to_thread_local();
        ret.persisted = true;
        ret.id = commit_id.id.to_string();
        match commit_id.try_to_commit_ref() {
            Ok(commit_ref) => {
                let dur =
                    commit_ref.time().seconds + commit_ref.time().offset as i64;
                ret.time = if dur > 0 {
                    SystemTime::UNIX_EPOCH + Duration::from_secs(dur as u64)
                } else {
                    SystemTime::UNIX_EPOCH
                };
            }
            Err(e) => {
                log::debug!(
                    "{} is not a valid NetworkState commit, skipping: {e}",
                    commit_id.id
                );
                return None;
            }
        }

        let tree = match commit_id.peel_to_tree() {
            Ok(t) => t,
            Err(e) => {
                log::debug!("Failed to convert object to tree:{e}");
                return None;
            }
        };
        log::debug!("Reading tree {}", tree.id);

        for entry in tree
            .iter()
            .filter_map(|res| res.ok().map(|entry| entry.inner))
        {
            let file_name = entry.filename;
            log::debug!("Loading file {file_name} to NetworkState");
            let object = match repo.find_object(entry.oid) {
                Ok(o) => o,
                Err(e) => {
                    log::info!("Ignore not found obj {}: {e}", entry.oid);
                    return None;
                }
            };
            match std::str::from_utf8(object.data.as_slice()) {
                Ok(content) => {
                    match serde_yaml::from_str::<NetworkState>(&content) {
                        Ok(s) => state.merge_desire(&s),
                        Err(e) => {
                            log::debug!(
                                "Invalid YAML content for \
                                {file_name}: {content}: {e}"
                            );
                            return None;
                        }
                    }
                }
                Err(e) => {
                    log::info!("Invalid content for file {file_name}: {e}",);
                    return None;
                }
            }
        }
        if state == NetworkState::default() {
            return None;
        } else {
            ret.state = state;
            return Some(ret);
        }
    }

    pub(crate) fn query_commits(
        &self,
        opt: &NetworkCommitQueryOption,
    ) -> Result<Vec<NetworkCommit>, NipartError> {
        let mut ret = Vec::new();
        log::trace!("Querying commit with option {opt}");
        let repo = self.config_repo.to_thread_local();
        if let Ok(head) = get_git_head(&repo) {
            if let Ok(parents) = head.id().ancestors().all() {
                for parent in parents.filter_map(|p| p.ok()) {
                    match repo.find_object(parent.id) {
                        Ok(obj) => {
                            if let Some(commit) = self.query_commit(obj) {
                                ret.push(commit)
                            }
                        }
                        Err(e) => {
                            log::debug!(
                                "Failed to find object {}: {e}",
                                parent.id
                            );
                            continue;
                        }
                    }
                }
            }
            if let Some(commit) = self.query_commit(head) {
                ret.insert(0, commit);
            }
        }
        Ok(ret)
    }
}

pub(crate) fn load_config_repo() -> Result<ThreadSafeRepository, NipartError> {
    let etc_path = std::path::Path::new(ETC_REPO_PATH);
    if !etc_path.is_dir() || !is_git_repo(etc_path) {
        init_config_repo(&etc_path)
    } else {
        open_config_repo(&etc_path)
    }
}

fn is_git_repo(path: &std::path::Path) -> bool {
    match gix::discover::is_git(path) {
        Err(gix::discover::is_git::Error::MissingHead) | Ok(_) => true,
        _ => false,
    }
}

fn init_config_repo(
    path: &std::path::Path,
) -> Result<ThreadSafeRepository, NipartError> {
    // TODO: Change folder permission to 700
    std::fs::create_dir_all(path).map_err(|e| {
        NipartError::new(
            ErrorKind::Bug,
            format!("Failed to create config folder {}: {e}", path.display()),
        )
    })?;
    let repo = ThreadSafeRepository::init(
        path,
        gix::create::Kind::WithWorktree,
        gix::create::Options::default(),
    )
    .map_err(|e| {
        NipartError::new(
            ErrorKind::Bug,
            format!("Failed to do git init in folder {}: {e}", path.display()),
        )
    })?;

    let mut local_repo = repo.to_thread_local();
    let tree = gix::objs::Tree::empty();
    let empty_tree_id = local_repo
        .write_object(&tree)
        .map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to git write_object: {e}"),
            )
        })?
        .detach();

    let tmp_repo = gen_git_snapshot_mut(&mut local_repo)?;
    tmp_repo
        .commit(
            "HEAD",
            "Initial empty commit",
            empty_tree_id,
            gix::commit::NO_PARENT_IDS,
        )
        .map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to git commit: {e}"),
            )
        })?;

    Ok(repo)
}

fn open_config_repo(
    path: &std::path::Path,
) -> Result<ThreadSafeRepository, NipartError> {
    ThreadSafeRepository::open(path).map_err(|e| {
        NipartError::new(
            ErrorKind::Bug,
            format!("Failed to do open git folder {}: {e}", path.display()),
        )
    })
}

fn gen_git_snapshot_mut(
    local_repo: &mut gix::Repository,
) -> Result<gix::config::CommitAutoRollback, NipartError> {
    let mut config = local_repo.config_snapshot_mut();
    config
        .set_raw_value("user", None, "name", GIT_USER_NAME)
        .map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to do git set author name: {e}"),
            )
        })?;
    config
        .set_raw_value("user", None, "email", GIT_USER_EMAIL)
        .map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to do git set author email: {e}"),
            )
        })?;

    config.commit_auto_rollback().map_err(|e| {
        NipartError::new(
            ErrorKind::Bug,
            format!("Failed to do git commit_auto_rollback(): {e}"),
        )
    })
}

fn get_git_head(repo: &gix::Repository) -> Result<gix::Object, NipartError> {
    let head = repo.head().map_err(|e| {
        NipartError::new(
            ErrorKind::Bug,
            format!("Failed to get config git head: {e}"),
        )
    })?;
    match head.into_peeled_object() {
        Ok(o) => Ok(o),
        Err(e) => Err(NipartError::new(
            ErrorKind::Bug,
            format!("Failed to convert head to object: {e}"),
        )),
    }
}
