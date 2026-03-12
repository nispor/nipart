// SPDX-License-Identifier: Apache-2.0

use nipart::NipartClient;

use crate::CliError;

pub(crate) struct CommandWaitOnline;

impl CommandWaitOnline {
    pub(crate) const CMD: &str = "wait-online";

    pub(crate) fn new_cmd() -> clap::Command {
        clap::Command::new(Self::CMD)
            .alias("w")
            .about("Wait daemon to reach online state")
    }

    pub(crate) async fn handle() -> Result<(), CliError> {
        let mut cli = NipartClient::new().await?;
        cli.wait_online().await?;
        println!("Network is online");
        Ok(())
    }
}
