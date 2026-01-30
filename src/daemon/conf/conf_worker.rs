// SPDX-License-Identifier: Apache-2.0

use std::os::unix::fs::PermissionsExt;

use futures_channel::{mpsc::UnboundedReceiver, oneshot::Sender};
use nipart::{
    ErrorKind, InterfaceType, NetworkState, NipartError, NipartstateInterface,
};
use tokio::{fs::File, io::AsyncWriteExt};

use crate::TaskWorker;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NipartConfCmd {
    /// Override saved network state
    SaveState(Box<NetworkState>),
    QueryState,
}

impl std::fmt::Display for NipartConfCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SaveState(_) => {
                write!(f, "save-state")
            }
            Self::QueryState => {
                write!(f, "query-state")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NipartConfReply {
    None,
    State(Box<NetworkState>),
}

type FromManager =
    (NipartConfCmd, Sender<Result<NipartConfReply, NipartError>>);

const INTERNAL_STATE_DIR: &str = "/etc/nipart/states/internal";
const APPLIED_STATE_PATH: &str = "/etc/nipart/states/internal/applied.yml";
const APPLIED_SECRETS_PATH: &str =
    "/etc/nipart/states/internal/applied.secrets.yml";

#[derive(Debug)]
pub(crate) struct NipartConfWorker {
    receiver: UnboundedReceiver<FromManager>,
    saved_state: NetworkState,
}

impl TaskWorker for NipartConfWorker {
    type Cmd = NipartConfCmd;
    type Reply = NipartConfReply;

    async fn new(
        receiver: UnboundedReceiver<FromManager>,
    ) -> Result<Self, NipartError> {
        Ok(Self {
            receiver,
            saved_state: read_state_from_file()?,
        })
    }

    fn receiver(&mut self) -> &mut UnboundedReceiver<FromManager> {
        &mut self.receiver
    }

    async fn process_cmd(
        &mut self,
        cmd: NipartConfCmd,
    ) -> Result<NipartConfReply, NipartError> {
        log::debug!("Processing config command: {cmd}");
        match cmd {
            NipartConfCmd::SaveState(mut state) => {
                discard_absent_iface(&mut state);
                save_state_to_file(&state).await?;
                self.saved_state = *state;
                Ok(NipartConfReply::None)
            }
            NipartConfCmd::QueryState => {
                Ok(NipartConfReply::State(Box::new(self.saved_state.clone())))
            }
        }
    }
}

fn read_state_from_file() -> Result<NetworkState, NipartError> {
    let content = if std::path::Path::new(APPLIED_STATE_PATH).exists() {
        match std::fs::read_to_string(APPLIED_STATE_PATH) {
            Ok(s) => s,
            Err(e) => {
                log::debug!(
                    "Failed to load saved state from {APPLIED_STATE_PATH}: {e}"
                );
                return Ok(NetworkState::default());
            }
        }
    } else {
        log::debug!("Saved state file {APPLIED_STATE_PATH} does not exist");
        return Ok(NetworkState::default());
    };
    let mut state = match serde_yaml::from_str::<NetworkState>(&content) {
        Ok(s) => s,
        Err(e) => {
            log::debug!(
                "Deleting corrupted saved state file {APPLIED_STATE_PATH}: {e}"
            );
            std::fs::remove_file(APPLIED_STATE_PATH).ok();
            NetworkState::default()
        }
    };

    if std::path::Path::new(APPLIED_SECRETS_PATH).exists()
        && let Ok(secrets) = std::fs::read_to_string(APPLIED_SECRETS_PATH) {
            match serde_yaml::from_str::<NetworkState>(&secrets) {
                Ok(s) => {
                    if let Err(e) = state.merge(&s) {
                        log::warn!(
                            "Failed to merge saved secrets into saved state, \
                             using empty state: {e}"
                        );
                        state = NetworkState::default();
                    }
                }
                Err(e) => {
                    log::debug!(
                        "Deleting corrupted saved secrets file \
                         {APPLIED_SECRETS_PATH}: {e}"
                    );
                    std::fs::remove_file(APPLIED_SECRETS_PATH).ok();
                }
            };
        }

    Ok(state)
}

async fn save_state_to_file(
    net_state: &NetworkState,
) -> Result<(), NipartError> {
    create_instal_state_dir()?;
    log::trace!("Saving state {net_state}");

    let mut state = net_state.clone();
    let secret_state = state.hide_secrets();

    let state_yaml_str = serde_yaml::to_string(&state).map_err(|e| {
        NipartError::new(
            ErrorKind::Bug,
            format!("Failed to generate YAML for {state}: {e}"),
        )
    })?;
    let secret_yaml_str =
        serde_yaml::to_string(&secret_state).map_err(|e| {
            NipartError::new(
                ErrorKind::Bug,
                format!("Failed to generate YAML for {secret_state}: {e}"),
            )
        })?;

    let mut fd = File::create(APPLIED_STATE_PATH).await?;
    fd.set_permissions(PermissionsExt::from_mode(0o644)).await?;
    fd.write_all(state_yaml_str.as_bytes()).await?;

    // We should remove the file first to make sure newly created
    // `APPLIED_SECRETS_PATH` is own by daemon uid.
    std::fs::remove_file(APPLIED_SECRETS_PATH).ok();
    let mut fd = File::create(APPLIED_SECRETS_PATH).await?;
    fd.set_permissions(PermissionsExt::from_mode(0o600)).await?;
    fd.write_all(secret_yaml_str.as_bytes()).await?;

    Ok(())
}

fn create_instal_state_dir() -> Result<(), NipartError> {
    let dir_path = std::path::Path::new(INTERNAL_STATE_DIR);
    if !dir_path.exists() {
        log::debug!("Creating dir {}", dir_path.display());
        std::fs::create_dir_all(dir_path).map_err(|e| {
            NipartError::new(
                ErrorKind::DaemonFailure,
                format!("Failed to create dir {}: {e}", dir_path.display()),
            )
        })?;
    }
    Ok(())
}

fn discard_absent_iface(state_to_save: &mut NetworkState) {
    let pending_changes: Vec<(String, InterfaceType)> = state_to_save
        .ifaces
        .iter()
        .filter_map(|i| {
            if i.is_absent() {
                Some((i.name().to_string(), i.iface_type().clone()))
            } else {
                None
            }
        })
        .collect();
    for (iface_name, iface_type) in pending_changes {
        state_to_save
            .ifaces
            .remove(iface_name.as_str(), Some(&iface_type));
    }
}
