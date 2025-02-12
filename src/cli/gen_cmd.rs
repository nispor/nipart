// SPDX-License-Identifier: Apache-2.0

use crate::{state::state_from_file, CliError};

pub(crate) struct GenCommand;

impl GenCommand {
    pub(crate) const NAME: &str = "gen";

    pub(crate) fn gen_command() -> clap::Command {
        clap::Command::new("gen")
            .alias("g")
            .about("Generate states")
            .subcommand(
                clap::Command::new("diff")
                    .alias("d")
                    .about("Generate difference between network states")
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
                    ),
            )
            .subcommand(
                clap::Command::new("revert")
                    .alias("r")
                    .about("Generate revert network states")
                    .arg(
                        clap::Arg::new("DESIRED_STATE")
                            .required(true)
                            .index(1)
                            .help("Desired Network state"),
                    )
                    .arg(
                        clap::Arg::new("PREAPPLY_STATE")
                            .required(true)
                            .index(2)
                            .help("Network State before applying desire state"),
                    ),
            )
    }

    pub(crate) async fn handle(
        matches: &clap::ArgMatches,
    ) -> Result<(), CliError> {
        let net_state =
            if let Some(diff_matches) = matches.subcommand_matches("diff") {
                let old_state = state_from_file(
                    diff_matches.get_one::<String>("OLD_STATE").unwrap(),
                )?;
                let new_state = state_from_file(
                    diff_matches.get_one::<String>("NEW_STATE").unwrap(),
                )?;
                new_state.gen_diff(&old_state)?
            } else if let Some(revert_matches) =
                matches.subcommand_matches("revert")
            {
                let desired_state = state_from_file(
                    revert_matches.get_one::<String>("DESIRED_STATE").unwrap(),
                )?;
                let preapply_state = state_from_file(
                    revert_matches.get_one::<String>("PREAPPLY_STATE").unwrap(),
                )?;
                desired_state.generate_revert(&preapply_state)?
            } else {
                return Err("Invalid sub-command for nipc gen".into());
            };

        println!("{}", serde_yaml::to_string(&net_state)?);
        Ok(())
    }
}
