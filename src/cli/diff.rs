// SPDX-License-Identifier: Apache-2.0

use super::state::state_from_file;
use crate::CliError;

pub(crate) struct CommandDiff;

impl CommandDiff {
    pub(crate) const CMD: &str = "diff";

    pub(crate) fn new_cmd() -> clap::Command {
        clap::Command::new("diff")
            .about("Generate difference between two states")
            .arg(
                clap::Arg::new("OLD_STATE")
                    .required(true)
                    .index(1)
                    .help("Old state"),
            )
            .arg(
                clap::Arg::new("NEW_STATE")
                    .required(true)
                    .index(2)
                    .help("New state"),
            )
    }

    pub(crate) async fn handle(
        matches: &clap::ArgMatches,
    ) -> Result<(), CliError> {
        // It is safe to unwrap because clap `required(true)` has confirmed
        // so.
        let old_state = matches
            .get_one::<String>("OLD_STATE")
            .map(|s| s.as_str())
            .unwrap();
        // It is safe to unwrap because clap `required(true)` has confirmed
        // so.
        let new_state = matches
            .get_one::<String>("NEW_STATE")
            .map(|s| s.as_str())
            .unwrap();

        let old_state = state_from_file(old_state)?;
        let new_state = state_from_file(new_state)?;

        println!(
            "{}",
            serde_yaml::to_string(&new_state.gen_diff(&old_state)?)?
        );
        Ok(())
    }
}
