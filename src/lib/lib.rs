// SPDX-License-Identifier: Apache-2.0

mod error;
mod event;
mod ipc;
mod logging;
mod plugin;
mod plugin_ipc;
mod plugin_trait;
mod state_options;
// TODO: Currently we are copy code from nmstate, hence suppressed warnings,
//       Need to clean up the code once detached from nmstate code base
#[allow(dead_code)]
mod state;

pub use self::error::{ErrorKind, NipartError};
pub use self::event::{
    NipartEvent, NipartEventAction, NipartEventAddress, NipartPluginEvent,
    NipartUserEvent,
};
pub use self::ipc::NipartConnection;
pub use self::logging::NipartLogLevel;
pub use self::plugin::{NipartPluginInfo, NipartRole};
pub use self::plugin_ipc::NipartConnectionListener;
pub use self::plugin_trait::NipartPlugin;
pub use self::state_options::{NipartApplyOption, NipartQueryOption};
// TODO Please remove this * once we detached from nmstate code base
pub use self::state::*;
