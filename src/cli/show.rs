// SPDX-License-Identifier: Apache-2.0

use nipart::{
    NetworkState, NipartClient, NipartNoDaemon, NipartstateInterface,
    NipartstateQueryOption,
};

use crate::CliError;

pub(crate) struct CommandShow;

impl CommandShow {
    pub(crate) const CMD: &str = "show";

    pub(crate) fn new_cmd() -> clap::Command {
        clap::Command::new("show")
            .alias("s")
            .about("Query network state")
            .arg(
                clap::Arg::new("IFNAME")
                    .index(1)
                    .help("Show specific interface only"),
            )
            .arg(
                clap::Arg::new("NO_DAEMON")
                    .long("no-daemon")
                    .visible_alias("kernel")
                    .short('n')
                    .visible_short_alias('k')
                    .action(clap::ArgAction::SetTrue)
                    .help("Do not connect to nipart daemon"),
            )
            .arg(
                clap::Arg::new("SAVED")
                    .long("saved")
                    .short('s')
                    .action(clap::ArgAction::SetTrue)
                    .help("Show the daemon saved state only"),
            )
    }

    pub(crate) async fn handle(
        matches: &clap::ArgMatches,
    ) -> Result<(), CliError> {
        let net_state = if matches.get_flag("NO_DAEMON") {
            if matches.get_flag("SAVED") {
                return Err("--no-daemon or --kernel cannot be used with \
                            --saved argument"
                    .into());
            }
            NipartNoDaemon::query_network_state(Default::default()).await?
        } else {
            let mut cli = NipartClient::new().await?;
            let opt = if matches.get_flag("SAVED") {
                NipartstateQueryOption::saved()
            } else {
                NipartstateQueryOption::running()
            };
            cli.query_network_state(opt).await?
        };
        let net_state =
            if let Some(ifname) = matches.get_one::<String>("IFNAME") {
                filter_net_state(&net_state, ifname)
            } else {
                net_state
            };

        println!("{}", serde_yaml::to_string(&net_state)?);

        Ok(())
    }
}

fn filter_net_state(
    net_state: &NetworkState,
    iface_name: &str,
) -> NetworkState {
    let mut ret = NetworkState::new();
    for iface in net_state.ifaces.to_vec() {
        if iface.name() == iface_name {
            ret.ifaces.push(iface.clone())
        }
    }
    ret
}
