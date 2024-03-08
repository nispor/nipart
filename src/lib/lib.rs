// SPDX-License-Identifier: Apache-2.0

mod dhcp;
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
#[allow(dead_code, unused_imports)]
mod state;

pub use self::dhcp::{
    NipartDhcpConfig, NipartDhcpConfigV4, NipartDhcpConfigV6, NipartDhcpLease,
    NipartDhcpLeaseV4, NipartDhcpLeaseV6,
};
pub use self::error::{ErrorKind, NipartError};
pub use self::event::{
    NipartEvent, NipartEventAddress, NipartPluginEvent, NipartUserEvent,
};
pub use self::ipc::{NipartConnection, DEFAULT_TIMEOUT};
pub use self::logging::NipartLogLevel;
pub use self::plugin::{NipartPluginInfo, NipartRole};
pub use self::plugin_ipc::NipartConnectionListener;
pub use self::plugin_trait::{NipartExternalPlugin, NipartNativePlugin};
pub use self::state_options::{NipartApplyOption, NipartQueryOption};

// TODO Please remove this * once we detached from nmstate code base
pub use self::state::*;
