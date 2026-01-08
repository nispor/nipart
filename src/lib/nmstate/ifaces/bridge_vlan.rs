// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, authors of original
// file(rust/src/lib/ifaces/bridge_vlan.rs) are:
//  * Gris Ge <fge@redhat.com>
//  * Fernando Fernandez Mancera <ffmancera@riseup.net>

use std::collections::{HashMap, hash_map::Entry};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer, ser::SerializeTuple,
};

use crate::{ErrorKind, JsonDisplay, NipartError};

#[derive(
    Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonDisplay,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
/// Bridge VLAN filtering configuration
pub struct BridgeVlanConfig {
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    /// Enable native VLAN.
    /// Deserialize and serialize from/to `enable-native`.
    pub enable_native: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Bridge VLAN filtering mode
    pub mode: Option<BridgeVlanMode>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u16_or_string"
    )]
    /// VLAN Tag for native VLAN.
    pub tag: Option<u16>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "bridge_trunk_tags_serialize"
    )]
    /// Trunk tags.
    /// Deserialize and serialize from/to `trunk-tags`.
    pub trunk_tags: Option<Vec<BridgeVlanTrunkTag>>,
}

impl BridgeVlanConfig {
    pub fn is_empty(&self) -> bool {
        self == &Default::default()
    }

    pub(crate) fn compress_port_vlan_ranges(&mut self) {
        if let Some(trunk_tags) = &self.trunk_tags {
            let mut new_trunk_tags: Vec<u16> = Vec::new();
            for trunk_tag in trunk_tags {
                match trunk_tag {
                    BridgeVlanTrunkTag::Id(vid) => new_trunk_tags.push(*vid),
                    BridgeVlanTrunkTag::IdRange(range) => {
                        for i in range.min..range.max + 1 {
                            new_trunk_tags.push(i);
                        }
                    }
                };
            }
            self.trunk_tags = Some(compress_vlan_ids(new_trunk_tags));
        }
    }

    pub(crate) fn sanitize(&mut self) -> Result<(), NipartError> {
        if self.mode == Some(BridgeVlanMode::Trunk)
            && self.tag.is_some()
            && self.tag != Some(0)
            && self.enable_native != Some(true)
        {
            return Err(NipartError::new(
                ErrorKind::InvalidArgument,
                "Bridge VLAN filtering `tag` cannot be use in trunk mode \
                 without `enable-native`"
                    .to_string(),
            ));
        }

        if self.mode == Some(BridgeVlanMode::Access)
            && self.enable_native == Some(true)
        {
            return Err(NipartError::new(
                ErrorKind::InvalidArgument,
                "Bridge VLAN filtering `enable-native: true` cannot be set in \
                 access mode"
                    .to_string(),
            ));
        }

        if self.mode == Some(BridgeVlanMode::Access) {
            if let Some(tags) = self.trunk_tags.as_ref() {
                if !tags.is_empty() {
                    return Err(NipartError::new(
                        ErrorKind::InvalidArgument,
                        "Bridge VLAN filtering access mode cannot have \
                         trunk-tags defined"
                            .to_string(),
                    ));
                }
            }
        }

        if self.mode == Some(BridgeVlanMode::Trunk) && self.trunk_tags.is_none()
        {
            return Err(NipartError::new(
                ErrorKind::InvalidArgument,
                "Bridge VLAN filtering trunk mode cannot have empty trunk-tags"
                    .to_string(),
            ));
        }
        if let Some(tags) = self.trunk_tags.as_ref() {
            if self.mode.is_none() {
                self.mode = Some(BridgeVlanMode::Trunk);
            }
            validate_overlap_trunk_tags(tags)?;
        }

        Ok(())
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    JsonDisplay,
)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum BridgeVlanMode {
    /// Trunk mode
    Trunk,
    /// Access mode
    #[default]
    Access,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum BridgeVlanTrunkTag {
    /// Single VLAN trunk ID
    Id(u16),
    /// VLAN trunk ID range
    IdRange(BridgeVlanRange),
}

impl std::fmt::Display for BridgeVlanTrunkTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Id(d) => write!(f, "id={d}"),
            Self::IdRange(range) => {
                write!(f, "id-range=[{},{}]", range.min, range.max)
            }
        }
    }
}

impl<'de> Deserialize<'de> for BridgeVlanTrunkTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = serde_json::Value::deserialize(deserializer)?;
        if let Some(id) = v.get("id") {
            if let Some(id) = id.as_str() {
                Ok(Self::Id(id.parse::<u16>().map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Failed to parse BridgeVlanTrunkTag id {id} as u16: \
                         {e}"
                    ))
                })?))
            } else if let Some(id) = id.as_u64() {
                Ok(Self::Id(u16::try_from(id).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Failed to parse BridgeVlanTrunkTag id {id} as u16: \
                         {e}"
                    ))
                })?))
            } else {
                Err(serde::de::Error::custom(format!(
                    "The id of BridgeVlanTrunkTag should be unsigned 16 bits \
                     integer, but got {v}"
                )))
            }
        } else if let Some(id_range) = v.get("id-range") {
            Ok(Self::IdRange(
                BridgeVlanRange::deserialize(id_range)
                    .map_err(serde::de::Error::custom)?,
            ))
        } else {
            Err(serde::de::Error::custom(format!(
                "BridgeVlanTrunkTag only support 'id' or 'id-range', but got \
                 {v}"
            )))
        }
    }
}

fn bridge_trunk_tags_serialize<S>(
    tags: &Option<Vec<BridgeVlanTrunkTag>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(tags) = tags {
        let mut serial_list = serializer.serialize_tuple(tags.len())?;
        for tag in tags {
            match tag {
                BridgeVlanTrunkTag::Id(id) => {
                    let mut map = HashMap::new();
                    map.insert("id", id);
                    serial_list.serialize_element(&map)?;
                }
                BridgeVlanTrunkTag::IdRange(id_range) => {
                    let mut map = HashMap::new();
                    map.insert("id-range", id_range);
                    serial_list.serialize_element(&map)?;
                }
            }
        }
        serial_list.end()
    } else {
        serializer.serialize_none()
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonDisplay,
)]
#[non_exhaustive]
#[serde(deny_unknown_fields)]
pub struct BridgeVlanRange {
    /// Minimum VLAN ID(included).
    #[serde(deserialize_with = "crate::deserializer::u16_or_string")]
    pub min: u16,
    /// Maximum VLAN ID(included).
    #[serde(deserialize_with = "crate::deserializer::u16_or_string")]
    pub max: u16,
}

fn validate_overlap_trunk_tags(
    tags: &[BridgeVlanTrunkTag],
) -> Result<(), NipartError> {
    let mut found: HashMap<u16, &BridgeVlanTrunkTag> = HashMap::new();
    for tag in tags {
        match tag {
            BridgeVlanTrunkTag::Id(d) => match found.entry(*d) {
                Entry::Occupied(o) => {
                    let existing_tag = o.get();
                    return Err(NipartError::new(
                        ErrorKind::InvalidArgument,
                        format!(
                            "Bridge VLAN trunk tag {tag} is overlapping with \
                             other tag {existing_tag}"
                        ),
                    ));
                }
                Entry::Vacant(v) => {
                    v.insert(tag);
                }
            },

            BridgeVlanTrunkTag::IdRange(range) => {
                for i in range.min..range.max + 1 {
                    match found.entry(i) {
                        Entry::Occupied(o) => {
                            let existing_tag = o.get();
                            return Err(NipartError::new(
                                ErrorKind::InvalidArgument,
                                format!(
                                    "Bridge VLAN trunk tag {tag} is \
                                     overlapping with other tag {existing_tag}"
                                ),
                            ));
                        }
                        Entry::Vacant(v) => {
                            v.insert(tag);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn compress_vlan_ids(mut vids: Vec<u16>) -> Vec<BridgeVlanTrunkTag> {
    vids.sort_unstable();
    vids.dedup();

    let mut ret: Vec<BridgeVlanTrunkTag> = Vec::new();

    let mut cur_range: Option<BridgeVlanRange> = None;
    let mut ranges: Vec<BridgeVlanRange> = Vec::new();

    for vid in vids {
        if let Some(mut cur) = cur_range.take() {
            if cur.max + 1 == vid {
                cur.max = vid;
                cur_range = Some(cur);
            } else {
                ranges.push(cur);
                cur_range = Some(BridgeVlanRange { min: vid, max: vid });
            }
        } else {
            cur_range = Some(BridgeVlanRange { min: vid, max: vid });
        }
    }
    if let Some(cur) = cur_range.take() {
        ranges.push(cur);
    }

    for range in ranges {
        if range.min == range.max {
            ret.push(BridgeVlanTrunkTag::Id(range.min));
        } else {
            ret.push(BridgeVlanTrunkTag::IdRange(range));
        }
    }

    ret
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_compress_bridge_vlan_ids() {
        let vids = vec![11u16, 1, 2, 3, 4, 9, 10, 20, 25, 26];

        assert_eq!(
            compress_vlan_ids(vids),
            vec![
                BridgeVlanTrunkTag::IdRange(BridgeVlanRange { min: 1, max: 4 }),
                BridgeVlanTrunkTag::IdRange(BridgeVlanRange {
                    min: 9,
                    max: 11
                }),
                BridgeVlanTrunkTag::Id(20),
                BridgeVlanTrunkTag::IdRange(BridgeVlanRange {
                    min: 25,
                    max: 26
                })
            ],
        )
    }
}
