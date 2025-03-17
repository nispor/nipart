// SPDX-License-Identifier: Apache-2.0

use crate::{Interfaces, MergedInterfaces, NipartError};

impl MergedInterfaces {
    pub(crate) fn generate_revert(&self) -> Result<Interfaces, NipartError> {
        let mut ret = Interfaces::default();
        for iface in self
            .kernel_ifaces
            .values()
            .chain(self.user_ifaces.values())
            .filter(|i| i.is_desired() || i.is_changed())
        {
            if let Some(new_iface) = iface.generate_revert()? {
                ret.push(new_iface);
            }
        }
        Ok(ret)
    }
}
