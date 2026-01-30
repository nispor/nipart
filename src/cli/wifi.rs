// SPDX-License-Identifier: Apache-2.0

use std::io::{IsTerminal, Write, stdin, stdout};

use nipart::{NetworkState, NipartClient, NipartNoDaemon};
use nix::sys::termios::{LocalFlags, SetArg, tcgetattr, tcsetattr};

use crate::CliError;

pub(crate) struct CommandWifi;

impl CommandWifi {
    pub(crate) const CMD: &str = "wifi";

    pub(crate) fn new_cmd() -> clap::Command {
        clap::Command::new("wifi")
            .about("WIFI actions")
            .subcommand_required(true)
            .subcommand(
                clap::Command::new("scan")
                    .about("WIFI active scan")
                    .alias("s")
                    .arg(
                        clap::Arg::new("IFACE")
                            .required(false)
                            .index(1)
                            .help("Scan on specified interface only"),
                    ),
            )
            .subcommand(
                clap::Command::new("connect")
                    .alias("c")
                    .about("Connect WIFI")
                    .arg(
                        clap::Arg::new("SSID")
                            .required(true)
                            .index(1)
                            .help("SSID to connect"),
                    )
                    .arg(
                        clap::Arg::new("NO_PASS")
                            .long("no-pass")
                            .action(clap::ArgAction::SetTrue)
                            .help(
                                "Do not ask for password(SSID does not \
                                 require password to connect)",
                            ),
                    ),
            )
    }

    pub(crate) async fn handle(
        matches: &clap::ArgMatches,
    ) -> Result<(), CliError> {
        if let Some(matches) = matches.subcommand_matches("scan") {
            let iface_name =
                matches.get_one::<String>("IFACE").map(|s| s.as_str());
            let mut wifi_cfgs = NipartNoDaemon::wifi_scan(iface_name).await?;
            wifi_cfgs.sort_unstable_by_key(|wifi_cfg| wifi_cfg.signal_percent);
            wifi_cfgs.reverse();
            println!("{}", serde_yaml::to_string(&wifi_cfgs)?);
        } else if let Some(matches) = matches.subcommand_matches("connect") {
            // It is safe to unwrap because of clap `required: true`
            let ssid = matches.get_one::<String>("SSID").unwrap();
            let state_str = if matches.get_flag("NO_PASS") {
                format!(
                    r#"---
                    interfaces:
                    - name: {ssid}
                      type: wifi-cfg
                      state: up
                      ipv4:
                        enabled: true
                        dhcp: true
                      wifi:
                        ssid: {ssid}
                    "#
                )
            } else {
                let pass = getpass()?;
                format!(
                    r#"---
                    interfaces:
                    - name: {ssid}
                      type: wifi-cfg
                      state: up
                      ipv4:
                        enabled: true
                        dhcp: true
                      wifi:
                        ssid: {ssid}
                        password: {pass}
                    "#
                )
            };

            let desired_state: NetworkState = serde_yaml::from_str(&state_str)?;
            let mut desired_state_to_show = desired_state.clone();
            desired_state_to_show.hide_secrets();
            log::info!(
                "Applying desire state:\n{}",
                serde_yaml::to_string(&desired_state_to_show)?
            );
            let mut cli = NipartClient::new().await?;
            cli.apply_network_state(desired_state, Default::default())
                .await?;
        }
        Ok(())
    }
}

// No idea why `libc::getpass()` or `nix::getpass()` does not exists, we have to
// it manually here.
fn getpass() -> Result<String, CliError> {
    let fd = stdin();
    let mut password = String::new();
    if fd.is_terminal() {
        let mut term = tcgetattr(&fd).map_err(|errno| {
            CliError::from(format!(
                "Failed to get terminal info from STDIN: {errno}"
            ))
        })?;
        let term_bak = term.clone();
        // Hide input
        term.local_flags.remove(LocalFlags::ECHO);
        // Show newline(user press enter)
        term.local_flags.insert(LocalFlags::ECHONL);

        tcsetattr(&fd, SetArg::TCSANOW, &term).map_err(|errno| {
            CliError::from(format!(
                "Failed to set STDIN terminal info for hiding password: \
                 {errno}"
            ))
        })?;

        print!("Please input password: ");
        stdout().flush().ok();
        let result = fd.read_line(&mut password);
        result?;
        // Restore the STDIN
        if let Err(errno) = tcsetattr(&fd, SetArg::TCSANOW, &term_bak) {
            log::warn!("Failed to restore STDIN terminal info: {errno}");
        };
    } else {
        fd.read_line(&mut password)?;
    }

    // Remove the tailing new line
    Ok(password.trim_end().to_string())
}
