// SPDX-License-Identifier: Apache-2.0

use std::io::Read;

use nipart::NetworkState;

use super::CliError;

pub(crate) fn state_from_file(
    file_path: &str,
) -> Result<NetworkState, CliError> {
    if file_path == "-" {
        state_from_fd(&mut std::io::stdin())
    } else {
        state_from_fd(&mut std::fs::File::open(file_path)?)
    }
}

fn state_from_fd<R>(fd: &mut R) -> Result<NetworkState, CliError>
where
    R: Read,
{
    let mut content = String::new();
    // Replace non-breaking space '\u{A0}'  to normal space
    fd.read_to_string(&mut content)?;
    let content = content.replace('\u{A0}', " ");

    Ok(serde_yaml::from_str::<NetworkState>(&content)?)
}
