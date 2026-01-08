// SPDX-License-Identifier: Apache-2.0

use crate::{
    BridgeVlanConfig, BridgeVlanMode, BridgeVlanRange, BridgeVlanTrunkTag,
};

pub(crate) fn parse_bridge_vlan_conf(
    np_vlan_entries: &[nispor::BridgeVlanEntry],
    default_pvid: Option<u16>,
) -> Option<BridgeVlanConfig> {
    let mut ret = BridgeVlanConfig::default();
    let mut is_native = false;
    let mut trunk_tags = Vec::new();
    let is_access_port = is_access_port(np_vlan_entries);

    for np_vlan_entry in np_vlan_entries {
        let mut def_pvid = 1;
        if let Some(pvid) = default_pvid {
            def_pvid = pvid;
        }
        let (vlan_min, vlan_max) = get_vlan_tag_range(np_vlan_entry);
        if vlan_min == def_pvid && vlan_max == def_pvid {
            continue;
        }
        if is_access_port {
            ret.tag = Some(vlan_min);
        } else if np_vlan_entry.is_pvid && np_vlan_entry.is_egress_untagged {
            ret.tag = Some(vlan_max);
            is_native = true;
        } else if vlan_min == vlan_max {
            trunk_tags.push(BridgeVlanTrunkTag::Id(vlan_min));
        } else {
            trunk_tags.push(BridgeVlanTrunkTag::IdRange(BridgeVlanRange {
                max: vlan_max,
                min: vlan_min,
            }));
        }
    }
    if trunk_tags.is_empty() {
        ret.mode = Some(BridgeVlanMode::Access);
    } else {
        ret.mode = Some(BridgeVlanMode::Trunk);
        ret.enable_native = Some(is_native);
    }
    if ret.mode == Some(BridgeVlanMode::Access)
        && trunk_tags.is_empty()
        && ret.tag.is_none()
    {
        None
    } else {
        ret.trunk_tags = Some(trunk_tags);

        Some(ret)
    }
}

fn is_access_port(np_vlan_entries: &[nispor::BridgeVlanEntry]) -> bool {
    np_vlan_entries.len() == 1
        && np_vlan_entries[0].is_pvid
        && np_vlan_entries[0].is_egress_untagged
}

fn get_vlan_tag_range(np_vlan_entry: &nispor::BridgeVlanEntry) -> (u16, u16) {
    np_vlan_entry.vid_range.unwrap_or((
        np_vlan_entry.vid.unwrap_or(1),
        np_vlan_entry.vid.unwrap_or(1),
    ))
}

fn _gen_bridge_vlan_conf(
    vlan_conf: &BridgeVlanConfig,
) -> Vec<nispor::BridgeVlanEntry> {
    let mut ret = Vec::new();
    match vlan_conf.mode {
        Some(BridgeVlanMode::Trunk) => {
            if let Some(trunk_tags) = &vlan_conf.trunk_tags {
                for trunk_tag in trunk_tags.as_slice() {
                    ret.push(trunk_tag_to_np_vlan_range(trunk_tag));
                }
            }
            if let Some(t) = vlan_conf.tag {
                ret.push(access_tag_to_np_vlan_range(t))
            }
        }
        Some(BridgeVlanMode::Access) => {
            if let Some(t) = vlan_conf.tag {
                ret.push(access_tag_to_np_vlan_range(t))
            };
        }
        _ => (),
    }

    ret
}

pub(crate) fn gen_bridge_vlan_conf(
    des_vlan_conf: &BridgeVlanConfig,
    cur_vlan_conf: Option<&BridgeVlanConfig>,
) -> Vec<nispor::BridgeVlanEntry> {
    let cur_entries = if let Some(cur_vlan_conf) = cur_vlan_conf.as_ref() {
        _gen_bridge_vlan_conf(cur_vlan_conf)
    } else {
        Vec::new()
    };
    let des_entries = _gen_bridge_vlan_conf(des_vlan_conf);

    let mut ret = Vec::new();

    // remove old entries
    for mut cur_entry in cur_entries {
        if !des_entries.contains(&cur_entry) {
            cur_entry.remove = true;
            ret.push(cur_entry)
        }
    }
    ret.extend(des_entries);
    ret
}

fn trunk_tag_to_np_vlan_range(
    trunk_tag: &BridgeVlanTrunkTag,
) -> nispor::BridgeVlanEntry {
    let mut ret = nispor::BridgeVlanEntry::default();
    match trunk_tag {
        BridgeVlanTrunkTag::Id(vid) => {
            ret.vid = Some(*vid);
        }
        BridgeVlanTrunkTag::IdRange(range) => {
            ret.vid_range = Some((range.min, range.max));
        }
    }
    ret.is_pvid = false;
    ret.is_egress_untagged = false;
    ret
}

fn access_tag_to_np_vlan_range(tag: u16) -> nispor::BridgeVlanEntry {
    let mut ret = nispor::BridgeVlanEntry::default();
    ret.vid = Some(tag);
    ret.is_pvid = true;
    ret.is_egress_untagged = true;
    ret
}
