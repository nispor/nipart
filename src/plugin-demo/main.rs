// SPDX-License-Identifier: Apache-2.0

mod plugin;

use nipart::{NipartError, NipartPlugin};

use self::plugin::NipartPluginDemo;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), NipartError> {
    NipartPluginDemo::run().await
}
