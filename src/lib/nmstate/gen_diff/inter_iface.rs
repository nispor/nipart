// SPDX-License-Identifier: Apache-2.0

use super::super::value::{copy_undefined_value, gen_diff_json_value};
use crate::{
    Interface, Interfaces, MergedInterfaces, NipartError, NipartstateInterface,
};

impl MergedInterfaces {
    pub fn gen_diff(&self) -> Result<Interfaces, NipartError> {
        let mut ret = Interfaces::default();
        for merged_iface in self
            .kernel_ifaces
            .values()
            .chain(self.user_ifaces.values())
            .filter(|i| i.is_desired() && i.desired != i.current)
        {
            let des_iface = if let Some(i) = merged_iface.for_apply.as_ref() {
                i
            } else {
                continue;
            };
            let cur_iface = if let Some(i) = merged_iface.current.as_ref() {
                i.clone()
            } else {
                if let Some(origin_des_iface) = &merged_iface.desired {
                    ret.push(origin_des_iface.clone());
                } else {
                    // Should never happen, but just in case.
                    ret.push(des_iface.clone());
                }
                continue;
            };
            let desired_value = serde_json::to_value(des_iface)?;
            let current_value = serde_json::to_value(&cur_iface)?;
            if let Some(diff_value) =
                gen_diff_json_value(&desired_value, &current_value)
            {
                let mut new_iface = des_iface.clone_name_type_only();
                new_iface.base_iface_mut().state = des_iface.base_iface().state;
                new_iface.include_diff_context(des_iface, &cur_iface);
                let mut new_iface_value = serde_json::to_value(&new_iface)?;
                copy_undefined_value(&mut new_iface_value, &diff_value);
                let new_iface =
                    serde_json::from_value::<Interface>(new_iface_value)?;
                ret.push(new_iface);
            }
        }
        Ok(ret)
    }
}

impl Interfaces {
    pub(crate) fn sanitize_for_diff(&mut self, current: &mut Self) {
        for iface in self.iter_mut() {
            if let Some(cur_iface) =
                current.get_mut(iface.name(), Some(iface.iface_type()))
            {
                iface.sanitize_before_verify(cur_iface);
            }
        }
    }
}
