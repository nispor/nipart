// SPDX-License-Identifier: Apache-2.0

mod commit;
mod dhcp;
mod error;
mod event;
mod ipc;
mod lock;
mod logging;
mod monitor;
mod nipart_uuid;
mod plugin;
pub(crate) mod plugin_common;
mod plugin_external;
mod plugin_ipc;
mod plugin_native;
pub(crate) mod state;
mod state_options;

pub use self::commit::{NetworkCommit, NetworkCommitQueryOption};
pub use self::dhcp::{
    NipartDhcpConfig, NipartDhcpConfigV4, NipartDhcpConfigV6, NipartDhcpLease,
    NipartDhcpLeaseV4, NipartDhcpLeaseV6,
};
pub use self::error::{ErrorKind, NipartError};
pub use self::event::{NipartEvent, NipartEventAddress, NipartUserEvent};
pub use self::ipc::{DEFAULT_TIMEOUT, NipartConnection};
pub use self::lock::{NipartLockEntry, NipartLockOption};
pub use self::logging::{NipartLogEntry, NipartLogLevel};
pub use self::monitor::{
    NipartAddressMonitorKind, NipartAddressMonitorRule, NipartLinkMonitorKind,
    NipartLinkMonitorRule, NipartMonitorEvent, NipartMonitorRule,
};
pub use self::nipart_uuid::NipartUuid;
pub use self::plugin::{
    NipartPluginEvent, NipartPluginInfo, NipartPostStartData, NipartRole,
};
pub use self::plugin_external::{NipartExternalPlugin, NipartPluginRunner};
pub use self::plugin_ipc::NipartConnectionListener;
pub use self::plugin_native::NipartNativePlugin;
// TODO: Use explicit export
pub use self::state::*;
pub use self::state_options::{
    NipartApplyOption, NipartQueryOption, NipartStateKind,
};
