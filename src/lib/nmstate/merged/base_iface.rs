// SPDX-License-Identifier: Apache-2.0

use crate::{BaseInterface, InterfaceState, InterfaceType, NipartError};

impl BaseInterface {
    // `NipartstateInterface::merge()` already done JSON level merging, this
    // function is doing special merging after that.
    pub fn post_merge(&mut self, old: &Self) -> Result<(), NipartError> {
        // Do not allow unknown interface type overriding existing
        // Do not allow ethernet interface type overriding veth
        if !(self.iface_type.is_unknown()
            || (old.iface_type == InterfaceType::Ethernet
                && self.iface_type == InterfaceType::Veth))
        {
            self.iface_type = old.iface_type.clone();
        }
        // Do not allow new unknown interface state to override old
        if self.state == InterfaceState::Unknown {
            self.state = old.state;
        }

        match (self.ipv4.as_mut(), old.ipv4.as_ref()) {
            (None, Some(old_ipv4)) => self.ipv4 = Some(old_ipv4.clone()),
            (Some(self_ipv4), Some(old_ipv4)) => self_ipv4.post_merge(old_ipv4),
            _ => (),
        }

        match (self.ipv6.as_mut(), old.ipv6.as_ref()) {
            (None, Some(old_ipv6)) => self.ipv6 = Some(old_ipv6.clone()),
            (Some(self_ipv6), Some(old_ipv6)) => self_ipv6.post_merge(old_ipv6),
            _ => (),
        }
        Ok(())
    }
}
