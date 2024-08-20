// SPDX-License-Identifier: Apache-2.0

mod commit;
mod dhcp;
mod error;
mod event;
mod ipc;
mod lock;
mod logging;
mod monitor;
mod plugin;
mod plugin_ipc;
mod plugin_trait;
mod state_options;
// TODO: Currently we are copy code from nmstate, hence suppressed warnings,
//       Need to clean up the code once detached from nmstate code base
#[allow(dead_code, unused_imports, unexpected_cfgs)]
mod state;

pub use self::commit::{NetworkCommit, NetworkCommitQueryOption};
pub use self::dhcp::{
    NipartDhcpConfig, NipartDhcpConfigV4, NipartDhcpConfigV6, NipartDhcpLease,
    NipartDhcpLeaseV4, NipartDhcpLeaseV6,
};
pub use self::error::{ErrorKind, NipartError};
pub use self::event::{NipartEvent, NipartEventAddress, NipartUserEvent};
pub use self::ipc::{NipartConnection, DEFAULT_TIMEOUT};
pub use self::lock::{NipartLockEntry, NipartLockOption};
pub use self::logging::NipartLogLevel;
pub use self::monitor::{
    NipartAddressMonitorKind, NipartAddressMonitorRule, NipartLinkMonitorKind,
    NipartLinkMonitorRule, NipartMonitorEvent, NipartMonitorRule,
};
pub use self::plugin::{NipartPluginEvent, NipartPluginInfo, NipartRole};
pub use self::plugin_ipc::NipartConnectionListener;
pub use self::plugin_trait::{NipartExternalPlugin, NipartNativePlugin};
pub use self::state_options::{NipartApplyOption, NipartQueryOption};

// TODO Please remove this * once we detached from nmstate code base
pub use self::state::*;
