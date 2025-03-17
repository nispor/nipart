// SPDX-License-Identifier: Apache-2.0

use nipart::{NetworkState, NipartConnection, NipartQueryOption};

use crate::CliError;

pub(crate) struct ShowCommand;

impl ShowCommand {
    pub(crate) const NAME: &str = "show";

    pub(crate) fn gen_command() -> clap::Command {
        clap::Command::new("show")
            .alias("s")
            .about("Query network state")
            .arg(
                clap::Arg::new("RUNNING")
                    .long("running")
                    .short('r')
                    .action(clap::ArgAction::SetTrue)
                    .help("Show running state(default)"),
            )
            .arg(
                clap::Arg::new("DIFF")
                    .long("diff")
                    .short('d')
                    .action(clap::ArgAction::SetTrue)
                    .help("Show changed state after last stored state"),
            )
            .arg(
                clap::Arg::new("SAVED")
                    .long("saved")
                    .short('s')
                    .action(clap::ArgAction::SetTrue)
                    .help("Show stored state"),
            )
            .arg(
                clap::Arg::new("POST")
                    .long("post")
                    .short('p')
                    .action(clap::ArgAction::SetTrue)
                    .help("Show full state after last commit"),
            )
            .group(
                clap::ArgGroup::new("FLAGS")
                    .args(["RUNNING", "DIFF", "SAVED", "POST"]),
            )
    }

    pub(crate) async fn handle(
        matches: &clap::ArgMatches,
    ) -> Result<(), CliError> {
        let mut conn = NipartConnection::new().await?;
        let net_state = if matches.get_flag("SAVED") {
            conn.query_net_state(NipartQueryOption::saved()).await?
        } else if matches.get_flag("DIFF") {
            Self::get_diff_state(&mut conn).await?
        } else if matches.get_flag("POST") {
            conn.query_net_state(NipartQueryOption::post_last_commit())
                .await?
        } else {
            conn.query_net_state(NipartQueryOption::running()).await?
        };

        println!("{}", serde_yaml::to_string(&net_state)?);
        Ok(())
    }

    pub(crate) async fn get_diff_state(
        conn: &mut NipartConnection,
    ) -> Result<NetworkState, CliError> {
        let post_commit_state = conn
            .query_net_state(NipartQueryOption::post_last_commit())
            .await?;
        let cur_net_state =
            conn.query_net_state(NipartQueryOption::running()).await?;

        Ok(cur_net_state.gen_diff(&post_commit_state)?)
    }
}
