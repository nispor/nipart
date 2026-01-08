// SPDX-License-Identifier: Apache-2.0

use crate::{CliError, state::state_from_file};

pub(crate) struct CommandMerge;

impl CommandMerge {
    pub(crate) const CMD: &str = "merge";

    pub(crate) fn new_cmd() -> clap::Command {
        clap::Command::new("merge")
            .alias("m")
            .about("Merged old network state with new")
            .arg(
                clap::Arg::new("OLD_STATE_FILE")
                    .required(true)
                    .index(1)
                    .help("Old Network state file"),
            )
            .arg(
                clap::Arg::new("NEW_STATE_FILE")
                    .required(true)
                    .index(2)
                    .help("New Network state file"),
            )
    }

    pub(crate) async fn handle(
        matches: &clap::ArgMatches,
    ) -> Result<(), CliError> {
        let mut state = state_from_file(
            matches.get_one::<String>("OLD_STATE_FILE").unwrap(),
        )?;
        let new_state = state_from_file(
            matches.get_one::<String>("NEW_STATE_FILE").unwrap(),
        )?;

        state.merge(&new_state)?;

        println!("{}", serde_yaml::to_string(&state)?);

        Ok(())
    }
}
