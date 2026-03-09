// SPDX-License-Identifier: Apache-2.0

mod client;
mod error;
mod ipc;
mod logging;
mod no_daemon;
mod plugin;
mod schema;
mod uuid;

pub use nipart_derive::{JsonDisplay, JsonDisplayHideSecrets};

pub use self::{
    client::{NipartClient, NipartClientCmd},
    error::{ErrorKind, NipartError},
    ipc::{NipartCanIpc, NipartIpcConnection},
    logging::{NipartLogEntry, NipartLogLevel},
    no_daemon::NipartNoDaemon,
    plugin::{
        NipartIpcListener, NipartPlugin, NipartPluginClient, NipartPluginCmd,
        NipartPluginInfo,
    },
    schema::*,
    uuid::NipartUuid,
};
