// SPDX-License-Identifier: Apache-2.0

use nipart::{NipartApplyOption, NipartConnection};

use crate::{show::ShowCommand, state::state_from_file, CliError};

pub(crate) struct ApplyCommand;

impl ApplyCommand {
    pub(crate) const NAME: &str = "apply";

    pub(crate) fn gen_command() -> clap::Command {
        clap::Command::new("apply")
            .alias("set")
            .alias("a")
            .about("Apply network config")
            .arg(
                clap::Arg::new("STATE_FILE")
                    .required(false)
                    .index(1)
                    .help("Network state file"),
            )
            .arg(
                clap::Arg::new("MEMORY_ONLY")
                    .long("memory-only")
                    .action(clap::ArgAction::SetTrue)
                    .required(false)
                    .help("Do not make the state persistent"),
            )
            .arg(
                clap::Arg::new("DIFF")
                    .long("diff")
                    .action(clap::ArgAction::SetTrue)
                    .required(false)
                    .help("Apply changed state since last commit"),
            )
    }

    pub(crate) async fn handle(
        matches: &clap::ArgMatches,
    ) -> Result<(), CliError> {
        let mut conn = NipartConnection::new().await?;
        let mut opt = NipartApplyOption::default();
        let state =
            if let Some(file_path) = matches.get_one::<String>("STATE_FILE") {
                state_from_file(file_path)?
            } else if matches.get_flag("DIFF") {
                let mut state = ShowCommand::get_diff_state(&mut conn).await?;
                state.description = "nipc apply --diff".to_string();
                opt.is_diff = true;
                state
            } else {
                state_from_file("-")?
            };
        if matches.get_flag("MEMORY_ONLY") {
            opt.memory_only = true;
        }
        conn.apply_net_state(state.clone(), opt).await?;
        println!("{}", serde_yaml::to_string(&state)?);
        Ok(())
    }
}
