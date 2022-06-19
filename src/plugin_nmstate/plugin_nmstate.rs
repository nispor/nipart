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

use async_trait::async_trait;
use nmstate::NetworkState;

use nipart::{
    ErrorKind, NipartError, NipartPlugin, NipartPluginCapacity,
    NipartQueryOption,
};

#[derive(Debug)]
struct NipartPluginNmstate {}

#[async_trait]
impl NipartPlugin for NipartPluginNmstate {
    fn name() -> &'static str {
        "nmstate"
    }

    fn capacities() -> Vec<NipartPluginCapacity> {
        vec![
            NipartPluginCapacity::QueryKernel,
            NipartPluginCapacity::ApplyKernel,
        ]
    }

    async fn query_kernel(
        _opt: &NipartQueryOption,
    ) -> Result<NetworkState, NipartError> {
        // Nmstate is using nispor which has tokio block_on which conflict
        // with tokio::main(), so we have to start thread for nmstate query.
        std::thread::spawn(|| {
            let mut net_state = NetworkState::new();
            net_state.set_kernel_only(true);
            match net_state.retrieve() {
                Ok(_) => Ok(net_state),
                Err(e) => {
                    Err(NipartError::new(ErrorKind::PluginError, e.to_string()))
                }
            }
        })
        .join()
        .expect("thread paniced")
    }
}

#[tokio::main()]
async fn main() {
    NipartPluginNmstate::run().await
}
