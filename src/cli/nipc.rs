// SPDX-License-Identifier: Apache-2.0

mod apply;
mod commit;
mod error;
mod gen_cmd;
mod show;
mod state;

use std::str::FromStr;

use nipart::{NipartConnection, NipartEvent, NipartLogLevel};

use crate::{
    apply::ApplyCommand, commit::CommitCommand, error::CliError,
    gen_cmd::GenCommand, show::ShowCommand,
};

#[tokio::main]
async fn main() -> Result<(), CliError> {
    let mut cli_cmd = clap::Command::new("nipc")
        .about("CLI to Nipart daemon")
        .arg_required_else_help(true)
        .arg(
            clap::Arg::new("verbose")
                .short('v')
                .action(clap::ArgAction::Count)
                .help("Increase verbose level")
                .global(true),
        )
        .subcommand(
            clap::Command::new("plugin")
                .alias("p")
                .about("plugin control")
                .arg_required_else_help(true)
                .subcommand(
                    clap::Command::new("show").alias("s").about("Show plugins"),
                ),
        )
        .subcommand(ShowCommand::gen_command())
        .subcommand(ApplyCommand::gen_command())
        .subcommand(GenCommand::gen_command())
        .subcommand(
            clap::Command::new("log")
                .alias("l")
                .arg_required_else_help(true)
                .about("Query/Change logging settings")
                .subcommand(
                    clap::Command::new("show")
                        .alias("s")
                        .about("Show logging level"),
                )
                .subcommand(
                    clap::Command::new("apply")
                        .alias("a")
                        .about("Change logging level")
                        .arg(
                            clap::Arg::new("level")
                                .index(1)
                                .value_parser(
                                    clap::builder::PossibleValuesParser::new([
                                        "off", "error", "info", "debug",
                                        "trace",
                                    ]),
                                )
                                .required(true)
                                .help("Log level"),
                        ),
                ),
        )
        .subcommand(
            clap::Command::new("daemon")
                .alias("d")
                .arg_required_else_help(true)
                .about("Nipartd daemon control")
                .subcommand(
                    clap::Command::new("stop")
                        .about("Instruct nipartd daemon to stop"),
                ),
        )
        .subcommand(CommitCommand::gen_command())
        .subcommand(
            clap::Command::new("debug")
                .about(
                    "For developer debug purpose only, \
                    send arbitrary event to daemon",
                )
                .arg(
                    clap::Arg::new("EVENT")
                        .index(1)
                        .help("YAML file path for event to sent"),
                ),
        );

    let matches = cli_cmd.get_matches_mut();

    let mut log_builder = env_logger::Builder::new();
    let log_level = match matches.get_count("verbose") {
        0 => log::LevelFilter::Info,
        1 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    log_builder.filter(Some("nipart"), log_level);
    log_builder.filter(None, log_level);
    log_builder.init();

    if let Some(m) = matches.subcommand_matches("plugin") {
        handle_plugin(m).await?;
    } else if let Some(matches) = matches.subcommand_matches(ShowCommand::NAME)
    {
        ShowCommand::handle(matches).await?;
    } else if let Some(m) = matches.subcommand_matches("log") {
        handle_log(m).await?;
    } else if let Some(matches) = matches.subcommand_matches("debug") {
        handle_debug(matches).await?;
    } else if let Some(matches) = matches.subcommand_matches("daemon") {
        handle_daemon_cmd(matches).await?;
    } else if let Some(matches) = matches.subcommand_matches(ApplyCommand::NAME)
    {
        ApplyCommand::handle(matches).await?;
    } else if let Some(matches) =
        matches.subcommand_matches(CommitCommand::NAME)
    {
        CommitCommand::handle(matches).await?;
    } else if let Some(matches) = matches.subcommand_matches(GenCommand::NAME) {
        GenCommand::handle(matches).await?;
    } else {
        eprintln!("Error: Invalid argument\n");
        cli_cmd.print_help()?;
        std::process::exit(1);
    }

    Ok(())
}

async fn handle_plugin(matches: &clap::ArgMatches) -> Result<(), CliError> {
    let mut conn = NipartConnection::new().await?;
    if matches.subcommand_matches("show").is_some() {
        let replies = conn.query_plugin_info().await?;
        println!("{}", serde_yaml::to_string(&replies)?);
        Ok(())
    } else {
        Err(format!("Invalid sub-command for `plugin` {matches:?}").into())
    }
}

async fn handle_debug(matches: &clap::ArgMatches) -> Result<(), CliError> {
    let mut conn = NipartConnection::new().await?;
    let event_file_path = matches.get_one::<String>("EVENT").unwrap();
    let event = read_event_from_file(event_file_path.as_str())?;
    conn.send(&event).await?;
    let replie = conn.recv_reply(event.uuid, conn.timeout).await?;
    println!("{}", serde_yaml::to_string(&replie)?);
    Ok(())
}

async fn handle_log(matches: &clap::ArgMatches) -> Result<(), CliError> {
    let mut conn = NipartConnection::new().await?;
    if matches.subcommand_matches("show").is_some() {
        let replies = conn.query_log_level().await?;
        println!("{}", serde_yaml::to_string(&replies)?);
    } else if let Some(m) = matches.subcommand_matches("apply") {
        let log_level_str: &String = m
            .get_one("level")
            .ok_or(CliError::from("Undefined log level"))?;
        let log_level = NipartLogLevel::from_str(log_level_str.as_str())?;
        let replies = conn.set_log_level(log_level).await?;
        println!("{}", serde_yaml::to_string(&replies)?);
    }
    Ok(())
}

async fn handle_daemon_cmd(matches: &clap::ArgMatches) -> Result<(), CliError> {
    let mut conn = NipartConnection::new().await?;
    if matches.subcommand_matches("stop").is_some() {
        conn.stop_daemon().await?;
    }
    Ok(())
}

fn read_event_from_file(file_path: &str) -> Result<NipartEvent, CliError> {
    let fd = std::fs::File::open(file_path)?;
    Ok(serde_yaml::from_reader(fd)?)
}
