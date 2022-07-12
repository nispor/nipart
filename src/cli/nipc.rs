//    Copyright 2021-2022 Red Hat, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod error;

use nipart::{
    ipc_connect, ipc_exec, NipartApplyOption, NipartIpcMessage,
    NipartQueryOption, NipartState,
};

use crate::error::CliError;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = clap::App::new("nip")
        .about("CLI to Nipart daemon")
        .arg_required_else_help(true)
        .subcommand(
            clap::SubCommand::with_name("show")
                .about("show network state")
                .alias("s")
                .alias("sh")
                .alias("sho"),
        )
        .subcommand(
            clap::SubCommand::with_name("apply")
                .about("apply network state")
                .alias("a")
                .alias("ap")
                .alias("app")
                .alias("appl")
                .arg(
                    clap::Arg::new("FILE")
                        .required(true)
                        .index(1)
                        .help("Network state file to apply"),
                ),
        )
        .get_matches();

    let mut log_builder = env_logger::Builder::new();
    log_builder.filter(Some("nipart"), log::LevelFilter::Info);
    log_builder.filter(None, log::LevelFilter::Debug);
    log_builder.init();

    if matches.subcommand_matches("show").is_some() {
        handle_show().await;
    } else if let Some(args) = matches.subcommand_matches("apply") {
        if let Some(file_path) = args.value_of("FILE") {
            handle_apply(file_path).await?;
        }
    }
    Ok(())
}

async fn handle_show() {
    let mut connection = ipc_connect().await.unwrap();
    match ipc_exec(
        &mut connection,
        &NipartIpcMessage::QueryState(NipartQueryOption::default()),
    )
    .await
    {
        Ok(NipartIpcMessage::QueryStateReply(net_state)) => {
            println!("{}", serde_yaml::to_string(&net_state).unwrap())
        }
        Ok(i) => log::error!("Unknown reply: {:?}", i),
        Err(e) => log::error!("{}", e),
    }
}

async fn handle_apply(file_path: &str) -> Result<(), CliError> {
    let mut connection = ipc_connect().await.unwrap();
    let state = read_state_from_file(file_path)?;
    match ipc_exec(
        &mut connection,
        &NipartIpcMessage::ApplyState(
            state.clone(),
            NipartApplyOption::default(),
        ),
    )
    .await
    {
        Ok(NipartIpcMessage::ApplyStateReply) => {
            log::info!(
                "State applied\n{}",
                serde_yaml::to_string(&state).unwrap()
            );
            Ok(())
        }
        Ok(i) => Err(format!("Unknown reply: {:?}", i).into()),
        Err(e) => Err(e.into()),
    }
}

fn read_state_from_file(file_path: &str) -> Result<NipartState, CliError> {
    let fd = std::fs::File::open(file_path)?;
    Ok(serde_yaml::from_reader(fd)?)
}
