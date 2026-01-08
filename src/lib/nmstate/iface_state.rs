// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original file is:
//  * Gris Ge <fge@redhat.com>
//  * Fernando Fernandez Mancera <ffmancera@riseup.net>
//  * Ales Musil <amusil@redhat.com>
//  * Quique Llorente <ellorent@redhat.com>
//  * Wen Liang <liangwen12year@gmail.com>
//  * Íñigo Huguet <ihuguet@redhat.com>

use serde::{Deserialize, Serialize};

use crate::JsonDisplay;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonDisplay,
)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
/// The state of interface
#[derive(Default)]
pub enum InterfaceState {
    /// Interface is marked as up.
    /// TODO: Please check link status for carrier state
    /// Deserialize and serialize from/to 'up'.
    #[default]
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
    /// Unknown state to nmstate. This state also been treated as
    /// [InterfaceState::Ignore] when applying.
    Unknown,
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
