// SPDX-License-Identifier: Apache-2.0

use crate::{Interface, MergedInterface, NipartstateInterface};

impl MergedInterface {
    /// Removing loopback is treated as reset to default
    pub(crate) fn post_merge_sanitize_loopback(&mut self) {
        if self.merged.is_absent() {
            self.for_apply = Some(Interface::Loopback(Box::default()));
            self.for_verify = Some(Interface::Loopback(Box::default()));
        }
    }
}
