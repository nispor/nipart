// SPDX-License-Identifier: Apache-2.0

use nipart::{ErrorKind, NipartError};

pub(crate) fn np_error_to_nipart(
    np_error: nispor::NisporError,
) -> NipartError {
    NipartError::new(
        ErrorKind::Bug,
        format!("{}: {}", np_error.kind, np_error.msg),
    )
}
