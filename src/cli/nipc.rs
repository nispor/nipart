//    Copyright 2021 Red Hat, Inc.
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

use nipart::{ipc_connect, ipc_exec, NipartIpcMessage, NipartQueryOption};

#[tokio::main]
async fn main() {
    let matches = clap::App::new("nip")
        .about("CLI to Nipart daemon")
        .subcommand(
            clap::SubCommand::with_name("show")
                .about("show network state")
                .alias("s")
                .alias("sh")
                .alias("sho"),
        )
        .get_matches();

    if matches.subcommand_matches("show").is_some() {
        handle_show().await;
    } else {
        todo!()
    }
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
        Ok(i) => eprintln!("Unknown reply: {:?}", i),
        Err(e) => eprintln!("{}", e),
    }
}
