// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{ErrorKind, NipartError};

/// UUID stored as unsigned 128 bit integer
#[repr(transparent)]
#[derive(PartialEq, Eq, Default, Clone, Copy, Hash, Debug, PartialOrd, Ord)]
pub struct NipartUuid(u128);

impl AsRef<u128> for NipartUuid {
    fn as_ref(&self) -> &u128 {
        &self.0
    }
}

impl std::ops::Deref for NipartUuid {
    type Target = u128;

    fn deref(&self) -> &u128 {
        &self.0
    }
}

impl std::ops::DerefMut for NipartUuid {
    fn deref_mut(&mut self) -> &mut u128 {
        &mut self.0
    }
}

impl std::fmt::Display for NipartUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", uuid::Uuid::from_u128(self.0))
    }
}

impl NipartUuid {
    pub const VOID: Self = Self(0);

    pub fn new() -> Self {
        let uuid = uuid::Uuid::now_v7().as_u128();
        if uuid == 0 { Self::new() } else { Self(uuid) }
    }
}

impl From<u128> for NipartUuid {
    fn from(d: u128) -> Self {
        Self(d)
    }
}

// The u128 is not native supported by other non-rust language, instead of
// serialize or deserialize it to integer, we use uuid string format
// The u128 is by default set to not supported by serde, so even in rust,
// u128 is most likely not parse able from JSON/YAML.
impl Serialize for NipartUuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for NipartUuid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let uuid_str = <String>::deserialize(deserializer)?;
        Ok(Self(
            uuid::Uuid::from_str(&uuid_str)
                .map_err(serde::de::Error::custom)?
                .as_u128(),
        ))
    }
}

impl FromStr for NipartUuid {
    type Err = NipartError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            uuid::Uuid::parse_str(s)
                .map_err(|e| {
                    NipartError::new(
                        ErrorKind::InvalidArgument,
                        format!("Invalid UUID string {s}: {e}"),
                    )
                })?
                .as_u128(),
        ))
    }
}
