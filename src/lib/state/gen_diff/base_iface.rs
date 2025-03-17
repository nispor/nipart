// SPDX-License-Identifier: Apache-2.0

use crate::BaseInterface;

impl BaseInterface {
    pub(crate) fn include_diff_context(
        &mut self,
        _desired: &Self,
        _current: &Self,
    ) {
        /*
        if self.identifier == Some(InterfaceIdentifier::MacAddress)
            && self.mac_address.is_none()
        {
            self.mac_address.clone_from(&current.mac_address)
        }
        */
    }
}
