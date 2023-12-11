// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use crate::NipartQueryStateOption;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum NipartUserEvent {
    QueryNetState(NipartQueryStateOption),
}
