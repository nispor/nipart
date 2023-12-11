// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NipartQueryStateOption {
    pub kernel_only: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NipartQueryConfigOption {
    pub include_memory: bool,
    pub include_ondisk: bool,
}
