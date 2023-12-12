// SPDX-License-Identifier: Apache-2.0

mod error;

use nipart::{
    NipartConnection, NipartError, NipartEvent, NipartEventAction,
    NipartEventAddress, NipartEventData,
};

use crate::error::CliError;

// timeout on 5 seconds
const IPC_TIMEOUT: u64 = 5000;

#[tokio::main]
async fn main() -> Result<(), CliError> {
    let matches = clap::Command::new("nipc")
        .about("CLI to Nipart daemon")
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("plugin-info")
                .alias("pi")
                .about("Query plugin info"),
        )
        .subcommand(
            clap::Command::new("debug")
                .about(
                    "For developer debug purpose only, \
                    send arbitrary event to daemon",
                )
                .alias("d")
                .arg(
                    clap::Arg::new("EVENT")
                        .index(1)
                        .help("YAML file path for event to sent"),
                ),
        )
        .get_matches();

    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(Some("nipart"), log::LevelFilter::Info);
    log_builder.filter(None, log::LevelFilter::Debug);
    log_builder.init();

    if let Some(_) = matches.subcommand_matches("plugin-info") {
        handle_plugin_info().await?;
    } else if let Some(matches) = matches.subcommand_matches("debug") {
        handle_debug(matches).await?;
    }
    Ok(())
}

async fn handle_plugin_info() -> Result<(), CliError> {
    let mut conn = NipartConnection::new().await?;
    let replies = conn.query_plugin_info().await?;
    println!("{}", serde_yaml::to_string(&replies)?);
    Ok(())
}

async fn handle_debug(matches: &clap::ArgMatches) -> Result<(), CliError> {
    let mut conn = NipartConnection::new().await?;
    let event_file_path = matches.get_one::<String>("EVENT").unwrap();
    let event = read_event_from_file(event_file_path.as_str())?;
    conn.send(&event).await?;
    let replies = conn.recv_reply(event.uuid, IPC_TIMEOUT, 0).await?;
    println!("{}", serde_yaml::to_string(&replies)?);
    Ok(())
}

fn read_event_from_file(file_path: &str) -> Result<NipartEvent, CliError> {
    let fd = std::fs::File::open(file_path)?;
    Ok(serde_yaml::from_reader(fd)?)
}
