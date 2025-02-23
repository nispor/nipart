// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use chrono::{DateTime, Local};
use nipart::{
    NetworkCommit, NetworkCommitQueryOption, NetworkState, NipartApplyOption,
    NipartConnection, NipartUuid,
};
use serde::{Deserialize, Serialize};

use crate::CliError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommitShowType {
    Brief,
    Normal,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CommitBriefShow {
    uuid: NipartUuid,
    time: String,
    desc: String,
}

impl From<&NetworkCommit> for CommitBriefShow {
    fn from(commit: &NetworkCommit) -> Self {
        Self {
            uuid: commit.uuid,
            desc: commit.description.clone(),
            time: DateTime::<Local>::from(commit.time).to_rfc2822(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CommitDefaultShow {
    uuid: NipartUuid,
    desc: String,
    time: String,
    stat: NetworkState,
}

impl From<&NetworkCommit> for CommitDefaultShow {
    fn from(commit: &NetworkCommit) -> Self {
        Self {
            uuid: commit.uuid,
            desc: commit.description.clone(),
            time: DateTime::<Local>::from(commit.time).to_rfc2822(),
            stat: commit.desired_state.clone(),
        }
    }
}

pub(crate) struct CommitCommand;

impl CommitCommand {
    pub(crate) const NAME: &str = "commit";

    pub(crate) fn gen_command() -> clap::Command {
        clap::Command::new("commit")
            .alias("c")
            .about("Nipartd commit control")
            .subcommand(
                clap::Command::new("show")
                    .alias("s")
                    .about("Show all network commits")
                    .arg(
                        clap::Arg::new("UUID")
                            .required(false)
                            .value_parser(
                                clap::builder::NonEmptyStringValueParser::new(),
                            )
                            .index(1)
                            .help("UUID of commit show only"),
                    )
                    .arg(
                        clap::Arg::new("COUNT")
                            .short('c')
                            .long("count")
                            .value_parser(clap::value_parser!(u32))
                            .help(
                                "Show only specified count of latest commits",
                            ),
                    )
                    .arg(
                        clap::Arg::new("FULL")
                            .short('f')
                            .long("full")
                            .action(clap::ArgAction::SetTrue)
                            .help("Show all information of commit"),
                    ),
            )
            .subcommand(
                clap::Command::new("revert")
                    .about("Revert specified commit")
                    .arg(
                        clap::Arg::new("UUID")
                            .required(true)
                            .value_parser(
                                clap::builder::NonEmptyStringValueParser::new(),
                            )
                            .index(1)
                            .help(
                                "Create new commit to revert the state \
                                in specified commit",
                            ),
                    )
                    .arg(
                        clap::Arg::new("MEMORY_ONLY")
                            .long("memory-only")
                            .action(clap::ArgAction::SetTrue)
                            .required(false)
                            .help("Do not make the revert state persistent"),
                    ),
            )
            .subcommand(
                clap::Command::new("remove")
                    .alias("rm")
                    .about("Remove specified commit and revert changes of it")
                    .arg(
                        clap::Arg::new("UUIDs")
                            .required(true)
                            .action(clap::ArgAction::Set)
                            .num_args(0..)
                            .help(
                                "UUIDs of commit to remove, \
                                state in these commit will be reverted",
                            ),
                    ),
            )
            .subcommand(
                clap::Command::new("rollback")
                    .about("Rollback to specified commit")
                    .arg(
                        clap::Arg::new("UUID")
                            .required(true)
                            .value_parser(
                                clap::builder::NonEmptyStringValueParser::new(),
                            )
                            .index(1)
                            .help(
                                "Apply new network state to \
                                revert all states since specified UUID",
                            ),
                    )
                    .arg(
                        clap::Arg::new("MEMORY_ONLY")
                            .long("memory-only")
                            .action(clap::ArgAction::SetTrue)
                            .required(false)
                            .help("Do not make the rollback state persistent"),
                    ),
            )
    }

    pub(crate) async fn handle(
        matches: &clap::ArgMatches,
    ) -> Result<(), CliError> {
        let mut conn = NipartConnection::new().await?;
        if let Some(revert_matches) = matches.subcommand_matches("revert") {
            let mut opt = NipartApplyOption::default();
            if revert_matches.get_flag("MEMORY_ONLY") {
                opt.memory_only = true;
            }

            if let Some(uuid) = revert_matches.get_one::<String>("UUID") {
                revert_commit(&mut conn, uuid, opt).await?;
            } else {
                return Err("UUID of commit to revert undefined".into());
            }
        } else if let Some(remove_matches) =
            matches.subcommand_matches("remove")
        {
            if let Some(uuids_iter) = remove_matches.get_many::<String>("UUIDs")
            {
                let uuids: Vec<String> = uuids_iter.cloned().collect();
                remove_commit(&mut conn, uuids).await?;
            } else {
                return Err("UUIDs of commit to remove undefined".into());
            }
        } else if let Some(rollback_matches) =
            matches.subcommand_matches("rollback")
        {
            let mut opt = NipartApplyOption::default();
            if rollback_matches.get_flag("MEMORY_ONLY") {
                opt.memory_only = true;
            }
            if let Some(uuid) = rollback_matches.get_one::<String>("UUID") {
                rollback_commit(&mut conn, &uuid, opt).await?;
            } else {
                return Err("UUID of commit to revert undefined".into());
            }
        } else if let Some(show_matches) = matches.subcommand_matches("show") {
            let mut opt = NetworkCommitQueryOption::default();
            if let Some(count) = show_matches.get_one::<u32>("COUNT") {
                opt.count = *count;
            }
            let commits = conn.query_commits(opt).await?;
            let show_full = show_matches.get_flag("FULL");
            if let Some(uuid) = show_matches.get_one::<String>("UUID") {
                show_commits(
                    commits
                        .as_slice()
                        .iter()
                        .filter(|c| c.uuid.to_string().as_str() == uuid),
                    if show_full {
                        CommitShowType::Full
                    } else {
                        CommitShowType::Normal
                    },
                )?;
            } else {
                show_commits(
                    commits.as_slice().iter(),
                    if show_full {
                        CommitShowType::Full
                    } else {
                        CommitShowType::Brief
                    },
                )?;
            }
        } else {
            // show brief by default
            let commits = conn.query_commits(Default::default()).await?;
            show_commits(commits.as_slice().iter(), CommitShowType::Brief)?;
        }
        Ok(())
    }
}

async fn revert_commit(
    conn: &mut NipartConnection,
    uuid: &str,
    apply_opt: NipartApplyOption,
) -> Result<(), CliError> {
    let uuid = NipartUuid::from_str(uuid)?;
    let opt = NetworkCommitQueryOption::default();
    let commits = conn.query_commits(opt).await?;
    if let Some(commit) = commits.as_slice().iter().find(|c| c.uuid == uuid) {
        if let Some(c) = conn
            .apply_net_state(commit.revert_state.clone(), apply_opt)
            .await?
        {
            show_commits(vec![c].as_slice().iter(), CommitShowType::Brief)?;
        }
        Ok(())
    } else {
        Err(format!("Commit with UUID {uuid} not found").into())
    }
}

fn show_commits<'a>(
    commits: impl Iterator<Item = &'a NetworkCommit>,
    show_type: CommitShowType,
) -> Result<(), CliError> {
    match show_type {
        CommitShowType::Brief => {
            let briefs: Vec<CommitBriefShow> =
                commits.map(CommitBriefShow::from).collect();
            for brief in briefs {
                println!("{}", serde_yaml::to_string(&brief)?);
            }
        }
        CommitShowType::Normal => {
            let ret: Vec<CommitDefaultShow> =
                commits.map(CommitDefaultShow::from).collect();
            println!("{}", serde_yaml::to_string(&ret)?);
        }
        CommitShowType::Full => {
            let commits: Vec<&'a NetworkCommit> = commits.collect();
            println!("{}", serde_yaml::to_string(&commits)?);
        }
    }
    Ok(())
}

async fn remove_commit(
    conn: &mut NipartConnection,
    uuids_str: Vec<String>,
) -> Result<(), CliError> {
    let mut uuids: Vec<NipartUuid> = Vec::new();
    for uuid_str in uuids_str {
        uuids.push(NipartUuid::from_str(&uuid_str)?);
    }

    conn.remove_commits(uuids.as_slice()).await?;
    Ok(())
}

async fn rollback_commit(
    conn: &mut NipartConnection,
    uuid_str: &str,
    opt: NipartApplyOption,
) -> Result<(), CliError> {
    let uuid = NipartUuid::from_str(uuid_str)?;
    let commits = conn.query_commits(Default::default()).await?;
    let mut commits_to_revert = Vec::new();
    let mut found = false;
    for commit in commits {
        if found {
            commits_to_revert.push(commit);
        } else if commit.uuid == uuid {
            found = true;
        }
    }

    let mut revert_state = NetworkState::default();
    for commit in commits_to_revert.as_slice() {
        revert_state.update_state(&commit.revert_state);
    }

    revert_state.description = format!("Rollback to commit {uuid}");
    if let Some(c) = conn.apply_net_state(revert_state, opt).await? {
        show_commits(vec![c].as_slice().iter(), CommitShowType::Brief)?;
    }
    Ok(())
}
