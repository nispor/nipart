// SPDX-License-Identifier: Apache-2.0

use crate::{ovsdb::db::OvsDbConnection, MergedNetworkState, NipartError};

pub(crate) fn ovsdb_apply(
    merged_state: &MergedNetworkState,
) -> Result<(), NipartError> {
    if merged_state.ovsdb.is_changed {
        let mut cli = OvsDbConnection::new()?;
        cli.apply_global_conf(&merged_state.ovsdb)
    } else {
        log::debug!("No OVSDB changes");
        Ok(())
    }
}
