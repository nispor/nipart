// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
/// The state of interface
pub enum InterfaceState {
    /// Interface is marked as up.
    /// TODO: Please check link status for carrier state
    /// Deserialize and serialize from/to 'up'.
    Up,
    /// Interface is marked as down.
    /// For apply action, down means configuration still exist but
    /// deactivate. The virtual interface will be removed and other interface
    /// will be reverted to default state.
    /// Deserialize and serialize from/to 'down'.
    Down,
    /// Only for apply action to remove configuration and bring interface to
    /// down state.
    /// Deserialize and serialize from/to 'absent'.
    Absent,
    /// Interface is not managed by backend. For apply action, interface marked
    /// as ignore will not be changed and will not cause verification failure
    /// neither.
    /// When desired controller listed currently ignored interfaces as its
    /// port, nmstate will automatically convert these ignored interfaces from
    /// 'state: ignore' to 'state: up' only when:
    ///  1. This ignored port is not mentioned in desire state.
    ///  2. This ignored port is listed as port of a desired controller.
    ///  3. Controller interface is new or does not contain ignored interfaces
    ///     currently.
    ///
    /// Deserialize and serialize from/to 'ignore'.
    Ignore,
    /// Interface is up but not managed by backend. For apply action, this
    /// state is equal to [InterfaceState::Ignore].
    /// Deserialize and serialize from/to 'up-ignore'.
    UpIgnore,
    /// Interface is down but not managed by backend. For apply action, this
    /// state is equal to [InterfaceState::Ignore].
    /// Deserialize and serialize from/to 'down-ignore'.
    DownIgnore,
    /// Unknown state to nipart. This state also been treated as
    /// [InterfaceState::Ignore] when applying.
    Unknown,
}

impl Default for InterfaceState {
    fn default() -> Self {
        Self::Up
    }
}

impl From<&str> for InterfaceState {
    fn from(s: &str) -> Self {
        match s {
            "up" => Self::Up,
            "down" => Self::Down,
            "absent" => Self::Absent,
            "ignore" => Self::Ignore,
            "up-ignore" => Self::UpIgnore,
            "down-ignore" => Self::DownIgnore,
            "unknown" => Self::Unknown,
            _ => {
                log::warn!("Unknown InterfaceState {s}, treating as `ignore`");
                Self::Ignore
            }
        }
    }
}

impl InterfaceState {
    /// Whether interface is in [InterfaceState::Ignore] or
    /// [InterfaceState::UpIgnore] or [InterfaceState::DownIgnore] or
    /// [InterfaceState::Unknown] state.
    pub fn is_ignore(&self) -> bool {
        matches!(
            self,
            Self::Ignore | Self::UpIgnore | Self::DownIgnore | Self::Unknown
        )
    }

    /// Whether interface is up and managed
    pub fn is_up(&self) -> bool {
        self == &Self::Up
    }

    pub fn is_down(&self) -> bool {
        self == &Self::Down
    }

    pub fn is_absent(&self) -> bool {
        self == &Self::Absent
    }
}
