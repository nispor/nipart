// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use super::value::copy_undefined_value;
use crate::{BaseInterface, InterfaceState, InterfaceType, NipartError};

/// Trait implemented by all type of interfaces.
pub trait NipartstateInterface:
    std::fmt::Debug + for<'a> Deserialize<'a> + Serialize + Default + Clone
{
    fn base_iface(&self) -> &BaseInterface;

    fn base_iface_mut(&mut self) -> &mut BaseInterface;

    /// Whether interface is physical interface or create by kernel or
    /// userspace at runtime.
    fn is_virtual(&self) -> bool;

    /// Whether specified interface only exist in user space configuration
    /// without any kernel interface index.
    fn is_userspace(&self) -> bool {
        self.iface_type().is_userspace()
    }

    /// Whether can hold ports
    fn is_controller(&self) -> bool {
        self.iface_type().is_controller()
    }

    fn name(&self) -> &str {
        self.base_iface().name.as_str()
    }

    fn iface_type(&self) -> &InterfaceType {
        &self.base_iface().iface_type
    }

    fn iface_state(&self) -> InterfaceState {
        self.base_iface().state
    }

    /// Invoke [BaseInterface::hide_secrets()] and interface specifics
    /// `hide_secrets_iface_specific()`.
    /// Will invoke `hide_secrets_iface_specific()` at the end.
    /// Please do not override this but implement
    /// `hide_secrets_iface_specific()` instead.
    fn hide_secrets(&mut self) {
        self.base_iface_mut().hide_secrets();
        self.hide_secrets_iface_specific();
    }

    fn hide_secrets_iface_specific(&mut self) {}

    fn is_ignore(&self) -> bool {
        self.base_iface().state.is_ignore()
    }

    fn is_up(&self) -> bool {
        self.base_iface().state.is_up()
    }

    fn is_down(&self) -> bool {
        self.base_iface().state.is_down()
    }

    fn is_absent(&self) -> bool {
        self.base_iface().state.is_absent()
    }

    /// Use properties defined in new_state to override Self without
    /// understanding the property meaning and limitation.
    /// Will invoke `merge_iface_specific()` at the end.
    /// Please do not override this function but implement
    /// `merge_iface_specific()` instead.
    fn merge(&self, new_state: &Self) -> Result<Self, NipartError>
    where
        for<'de> Self: Deserialize<'de>,
    {
        let mut new_value = serde_json::to_value(new_state)?;
        let old_value = serde_json::to_value(self)?;
        copy_undefined_value(&mut new_value, &old_value);

        let old_state = self.clone();

        let mut ret: Self = serde_json::from_value(new_value)?;
        ret.base_iface_mut().post_merge(old_state.base_iface())?;
        ret.post_merge_iface_specific(&old_state)?;

        Ok(ret)
    }

    /// Please implemented this function if special merge action required
    /// for certain interface type. Do not need to worry about the merge of
    /// [BaseInterface].
    fn post_merge_iface_specific(
        &mut self,
        _old_state: &Self,
    ) -> Result<(), NipartError> {
        Ok(())
    }

    /// Invoke sanitize on the [BaseInterface] and `sanitize_iface_specfic()`.
    /// Sanitation process is performed for apply action when merging desired
    /// state with current state:
    ///  * Validate user inputs.
    ///  * Clean up properties which is for query only.
    ///  * Change desired state smartly. (e.g. Remove IP for disabled IP stack)
    fn sanitize(&mut self, current: Option<&Self>) -> Result<(), NipartError> {
        self.base_iface_mut()
            .sanitize(current.as_ref().map(|c| c.base_iface()))?;
        self.sanitize_iface_specfic(current)
    }

    /// Invoke sanitize current for verify on the [BaseInterface] and
    /// `sanitize_before_verify_iface_specfic()`
    fn sanitize_before_verify(&mut self, current: &mut Self) {
        self.base_iface_mut()
            .sanitize_before_verify(current.base_iface_mut());
        self.sanitize_before_verify_iface_specfic(current);
    }

    /// Please implement this function if special sanitize action required
    /// for certain interface type.
    /// Do not include action for [BaseInterface].
    fn sanitize_iface_specfic(
        &mut self,
        _current: Option<&Self>,
    ) -> Result<(), NipartError> {
        Ok(())
    }

    /// Please implement this function if special sanitize action required
    /// for certain interface type before verification.
    /// Do not include action for [BaseInterface].
    fn sanitize_before_verify_iface_specfic(&mut self, _current: &mut Self) {}

    /// When generating difference between desired and current, certain value
    /// should be included as context in the output. For example, when
    /// VLAN ID changed, including base-iface as context seems reasonable.
    /// Default implementation does nothing.
    /// This function will invoke `include_diff_context()` against
    /// `BaseInterface`. For any interface specific task, please implement
    /// `include_diff_context_iface_specific()` instead.
    fn include_diff_context(&mut self, desired: &Self, current: &Self) {
        self.base_iface_mut()
            .include_diff_context(desired.base_iface(), current.base_iface());
        self.include_diff_context_iface_specific(desired, current)
    }

    fn include_diff_context_iface_specific(
        &mut self,
        _desired: &Self,
        _current: &Self,
    ) {
    }

    fn from_base(base_iface: BaseInterface) -> Self {
        let mut new = Self::default();
        *new.base_iface_mut() = base_iface;
        new
    }

    /// When generating revert state for desired state, certain value
    /// should be included as context in the output. For example, when
    /// reverting a IP disable action, we should include its original static
    /// IP addresses.
    /// This function will invoke `include_revert_context()` against
    /// `BaseInterface`. For any interface specific task, please implement
    /// `include_revert_context_iface_specific()` instead.
    fn include_revert_context(&mut self, desired: &Self, pre_apply: &Self) {
        self.base_iface_mut().include_revert_context(
            desired.base_iface(),
            pre_apply.base_iface(),
        );
        self.include_revert_context_iface_specific(desired, pre_apply);
    }

    fn include_revert_context_iface_specific(
        &mut self,
        _desired: &Self,
        _pre_apply: &Self,
    ) {
    }

    /// Return a list of port names. None means not desired or cannot hold
    /// ports
    fn ports(&self) -> Option<Vec<&str>> {
        None
    }

    /// Return parent interface name, None means not desired or no parent
    fn parent(&self) -> Option<&str> {
        None
    }

    /// Whether desired changes need to delete the interface first.
    /// Default implementation is false
    fn need_delete_before_change(&self, _current: &Self) -> bool {
        false
    }
}
