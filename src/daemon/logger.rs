// SPDX-License-Identifier: Apache-2.0

use nipart::NipartIpcConnection;

pub(crate) async fn log_trace(
    mut conn: Option<&mut NipartIpcConnection>,
    msg: String,
) {
    log::trace!("{msg}");
    if let Some(conn) = conn.as_mut() {
        conn.log_trace(msg).await;
    }
}

pub(crate) async fn log_info(
    mut conn: Option<&mut NipartIpcConnection>,
    msg: String,
) {
    log::info!("{msg}");
    if let Some(conn) = conn.as_mut() {
        conn.log_info(msg).await;
    }
}

pub(crate) async fn log_debug(
    mut conn: Option<&mut NipartIpcConnection>,
    msg: String,
) {
    log::debug!("{msg}");
    if let Some(conn) = conn.as_mut() {
        conn.log_debug(msg).await;
    }
}

pub(crate) async fn log_warn(
    mut conn: Option<&mut NipartIpcConnection>,
    msg: String,
) {
    log::warn!("{msg}");
    if let Some(conn) = conn.as_mut() {
        conn.log_warn(msg).await;
    }
}

pub(crate) async fn log_error(
    mut conn: Option<&mut NipartIpcConnection>,
    msg: String,
) {
    log::error!("{msg}");
    if let Some(conn) = conn.as_mut() {
        conn.log_error(msg).await;
    }
}
