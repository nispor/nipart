// SPDX-License-Identifier: Apache-2.0

use nipart::{
    ErrorKind, NetworkState, NipartClientCmd, NipartError, NipartIpcConnection,
};

use crate::{
    commander::NipartCommander, lock::NipartLockManager, log_debug, log_info,
};

pub(crate) async fn process_api_connection(
    mut conn: NipartIpcConnection,
    mut commander: NipartCommander,
) -> Result<(), NipartError> {
    let (peer_uid, peer_pid) = get_peer_info(&conn)?;

    log_debug(
        Some(&mut conn),
        format!("Got connection from PID {peer_pid} UID {peer_uid}"),
    )
    .await;

    loop {
        let cmd = match conn.recv::<NipartClientCmd>().await {
            Ok(c) => {
                if let Err(e) = permission_check(&c, peer_uid) {
                    conn.send::<Result<(), NipartError>>(Err(e)).await?;
                    continue;
                } else {
                    c
                }
            }
            Err(e) => {
                if e.kind == ErrorKind::IpcClosed {
                    break Ok(());
                }
                conn.send::<Result<(), NipartError>>(Err(e)).await?;
                continue;
            }
        };
        match cmd {
            NipartClientCmd::Ping => conn.send(Ok("pong".to_string())).await?,
            NipartClientCmd::QueryNetworkState(opt) => {
                let result =
                    commander.query_network_state(Some(&mut conn), *opt).await;
                conn.send(result).await?;
            }
            NipartClientCmd::ApplyNetworkState(opt) => {
                log_info(
                    Some(&mut conn),
                    format!(
                        "Client process {peer_pid} acquiring lock before \
                         apply state"
                    ),
                )
                .await;
                if let Some(cur_locker) = NipartLockManager::cur_locker_pid() {
                    log_info(
                        Some(&mut conn),
                        format!(
                            "Waiting on-going transaction by PID {cur_locker}"
                        ),
                    )
                    .await;
                }

                let lock = NipartLockManager::lock(peer_pid).await;
                log_info(
                    Some(&mut conn),
                    format!("Client process {peer_pid} acquired lock"),
                )
                .await;
                let (desired_state, opt) = *opt;
                let result = commander
                    .apply_network_state(Some(&mut conn), desired_state, opt)
                    .await;
                log_info(
                    Some(&mut conn),
                    format!("Client process {peer_pid} released lock"),
                )
                .await;
                drop(lock);
                conn.send(result).await?;
            }
            _ => {
                conn.send::<Result<NetworkState, NipartError>>(Err(
                    NipartError::new(
                        ErrorKind::NoSupport,
                        format!("Unsupported request {cmd:?}"),
                    ),
                ))
                .await?;
            }
        }
    }
}

// Once https://github.com/rust-lang/rust/issues/76915 goes stable and shipped
// to most distributions, we should use `std::os::unix::net::SocketCred`
//
// Return (uid, pid)
fn get_peer_info(
    conn: &NipartIpcConnection,
) -> Result<(u32, i32), NipartError> {
    let credential = nix::sys::socket::getsockopt(
        conn,
        nix::sys::socket::sockopt::PeerCredentials,
    )
    .map_err(|e| {
        NipartError::new(
            ErrorKind::Bug,
            format!("Failed to getsockopt SO_PEERCRED failed: {e}"),
        )
    })?;

    Ok((credential.uid(), credential.pid()))
}

fn permission_check(
    command: &NipartClientCmd,
    peer_uid: u32,
) -> Result<(), NipartError> {
    if peer_uid == 0 {
        Ok(())
    } else {
        match command {
            NipartClientCmd::Ping => Ok(()),
            NipartClientCmd::QueryNetworkState(s) => {
                if s.include_secrets {
                    Err(NipartError::new(
                        ErrorKind::PermissionDeny,
                        "Query with secrets included requires root permission"
                            .into(),
                    ))
                } else {
                    Ok(())
                }
            }
            _ => Err(NipartError::new(
                ErrorKind::PermissionDeny,
                "Command {command} need to root permission".into(),
            )),
        }
    }
}
