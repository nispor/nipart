// SPDX-License-Identifier: Apache-2.0

use crate::{MergedRoutes, Routes};

impl MergedRoutes {
    pub fn gen_diff(&self) -> Routes {
        Routes {
            config: if self.changed_routes.is_empty() {
                None
            } else {
                Some(self.changed_routes.clone())
            },
            ..Default::default()
        }
    }
}
