// SPDX-License-Identifier: Apache-2.0

use crate::{
    Interface, InterfaceState, InterfaceType, MergedInterfaces,
    NipartstateInterface,
};

impl MergedInterfaces {
    /// For WIFI bind to any interface, we should mark all suitable wifi-phy up
    pub(crate) fn post_merge_sanitize_wifi(&mut self) {
        if self.has_any_bind_wifi() {
            for merged_iface in
                self.kernel_ifaces.values_mut().filter(|merged_iface| {
                    merged_iface.merged.iface_type() == &InterfaceType::WifiPhy
                        && merged_iface.desired.as_ref().map(|i| {
                            i.is_absent() || i.is_down() || i.is_ignore()
                        }) != Some(true)
                        && merged_iface.current.is_some()
                })
            {
                merged_iface.mark_as_changed();
                if let Some(iface) = merged_iface.for_apply.as_mut() {
                    iface.base_iface_mut().state = InterfaceState::Up;
                }
            }
        }
    }

    pub(crate) fn has_any_bind_wifi(&self) -> bool {
        self.user_ifaces.values().any(|merged_iface| {
            if let Some(Interface::WifiCfg(iface)) =
                merged_iface.for_apply.as_ref()
                && iface.is_up()
                && iface.wifi.as_ref().map(|w| w.base_iface.is_none())
                    == Some(true)
            {
                true
            } else {
                false
            }
        })
    }
}
