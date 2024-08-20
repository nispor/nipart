#!/usr/bin/python3
# SPDX-License-Identifier: Apache-2.0

import os
import subprocess
import shutil

DENY_LIST = (
    "nm",
    "nispor",
    "Cargo.toml",
    "unit_tests",
    "gen_conf.rs",
    "policy",
)

SCRIPT_DIR = os.path.dirname(os.path.realpath(__file__))
NMSTATE_RUST_CODE_DIR = os.path.realpath(
    f"{SCRIPT_DIR}/../../../../nmstate/rust/src/lib"
)


def fix_relative_import():
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + ["s/crate::deserializer/crate::state::deserializer/g", "{}", ";"],
        check=True,
    )
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + ["s/crate::serializer/crate::state::serializer/g", "{}", ";"],
        check=True,
    )
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + ["s/ip::is_ip/state::ip::is_ip/g", "{}", ";"],
        check=True,
    )
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + ["s/ip::{is_ip/state::ip::{is_ip/g", "{}", ";"],
        check=True,
    )
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + ["s/revert::state/state::revert::state/g", "{}", ";"],
        check=True,
    )
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + ["s/ deserializer::/ state::deserializer::/g", "{}", ";"],
        check=True,
    )


def exposing_merged_xxx():
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + ["s/pub(crate) struct Merged/pub struct Merged/g", "{}", ";"],
        check=True,
    )
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + [r"s/pub(crate) use \(.\+::Merged.\+\)$/pub use \1/g", "{}", ";"],
        check=True,
    )


def replace_nmstate_error_with_nipart_error():
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + ["s/NmstateError/NipartError/g", "{}", ";"],
        check=True,
    )


def remove_query_apply_net_state():
    os.unlink(f"{SCRIPT_DIR}/query_apply/net_state.rs")
    subprocess.run(
        f"sed -i -e".split()
        + ["/mod net_state/d", f"{SCRIPT_DIR}/query_apply/mod.rs"],
        check=True,
    )


def fix_merge_json_value():
    os.rename(f"{SCRIPT_DIR}/state.rs", f"{SCRIPT_DIR}/json.rs")
    subprocess.run(
        f"sed -i -e".split()
        + [
            "s/use crate::state::merge_json_value/"
            "use super::json::merge_json_value/",
            f"{SCRIPT_DIR}/iface.rs",
        ],
        check=True,
    )
    subprocess.run(
        f"sed -i -e".split()
        + [
            "s/state::{gen_diff_json_value/state::json::{gen_diff_json_value/",
            f"{SCRIPT_DIR}/query_apply/inter_ifaces.rs",
        ],
        check=True,
    )
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + [
            "s/state::get_json_value_difference/"
            "state::json::get_json_value_difference/g",
            "{}",
            ";",
        ],
        check=True,
    )


def remove_feature_compile_line():
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + [
            r"/^#\[cfg(feature/d",
            "{}",
            ";",
        ],
        check=True,
    )
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + [
            r"/^#\[cfg(not(feature/d",
            "{}",
            ";",
        ],
        check=True,
    )


def change_merged_network_state_props_to_public():
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + [
            "s/pub(crate) interfaces: MergedInterfaces/"
            "pub interfaces: MergedInterfaces/g",
            "{}",
            ";",
        ],
        check=True,
    )
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + [
            "s/pub(crate) dns: MergedDnsState/" "pub dns: MergedDnsState/g",
            "{}",
            ";",
        ],
        check=True,
    )
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + [
            "s/pub(crate) routes: MergedRoutes/" "pub routes: MergedRoutes/g",
            "{}",
            ";",
        ],
        check=True,
    )
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + [
            "s/pub(crate) rules: MergedRouteRules/"
            "pub rules: MergedRouteRules/g",
            "{}",
            ";",
        ],
        check=True,
    )
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + [
            "s/pub(crate) kernel_ifaces: HashMap<String, Interface>/"
            "pub kernel_ifaces: HashMap<String, Interface>/g",
            "{}",
            ";",
        ],
        check=True,
    )
    subprocess.run(
        f"sed -i -e".split()
        + [
            r"s/pub(crate) fn new($/pub fn new(/",
            f"{SCRIPT_DIR}/net_state.rs",
        ],
        check=True,
    )


def change_supported_list_to_public():
    subprocess.run(
        f"sed -i -e".split()
        + [
            "s/pub(crate) const SUPPORTED_LIST/pub const SUPPORTED_LIST/",
            f"{SCRIPT_DIR}/query_apply/iface.rs",
        ],
        check=True,
    )


def change_prop_list_to_public():
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + [
            "s/pub(crate) prop_list:/pub prop_list:/g",
            "{}",
            ";",
        ],
        check=True,
    )


def change_is_controller_to_public():
    subprocess.run(
        f"sed -i -e".split()
        + [
            "s/pub(crate) fn is_controller/pub fn is_controller/",
            f"{SCRIPT_DIR}/iface.rs",
        ],
        check=True,
    )


def change_merged_interfaces_props_to_public():
    subprocess.run(
        f"sed -i -e".split()
        + [
            "s/pub(crate) fn iter/pub fn iter/",
            f"{SCRIPT_DIR}/ifaces/inter_ifaces.rs",
        ],
        check=True,
    )
    subprocess.run(
        f"sed -i -e".split()
        + [
            "s/pub(crate) kernel_ifaces:/pub kernel_ifaces:/",
            f"{SCRIPT_DIR}/ifaces/inter_ifaces.rs",
        ],
        check=True,
    )


def change_merged_interface_props_to_public():
    subprocess.run(
        f"sed -i -e".split()
        + [
            "s/pub(crate) for_apply:/pub for_apply:/",
            f"{SCRIPT_DIR}/iface.rs",
        ],
        check=True,
    )
    subprocess.run(
        f"sed -i -e".split()
        + [
            "s/pub(crate) merged:/pub merged:/",
            f"{SCRIPT_DIR}/iface.rs",
        ],
        check=True,
    )
    subprocess.run(
        f"sed -i -e".split()
        + [
            "s/pub(crate) current:/pub current:/",
            f"{SCRIPT_DIR}/iface.rs",
        ],
        check=True,
    )


def expose_merged_xxx_is_changed_func():
    subprocess.run(
        f"sed -i -e".split()
        + [
            "s/pub(crate) fn is_changed(/pub fn is_changed(/",
            f"{SCRIPT_DIR}/iface.rs",
        ],
        check=True,
    )


def change_base_iface_props_to_public():
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + ["s/pub(crate) fn is_changed(/pub fn is_changed(/g", "{}", ";"],
        check=True,
    )
    subprocess.run(
        f"sed -i -e".split()
        + [
            "s/pub(crate) up_priority:/pub up_priority:/",
            f"{SCRIPT_DIR}/ifaces/base.rs",
        ],
        check=True,
    )


def change_func_ports_to_public():
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + [
            "s/pub(crate) fn ports(/pub fn ports(/g",
            "{}",
            ";",
        ],
        check=True,
    )


def serde_merged_state():
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + [
            "s/pub struct Merged/"
            r"#[derive(Deserialize, Serialize)]\npub struct Merged/g",
            "{}",
            ";",
        ],
        check=True,
    )


def expose_ip_enabled_defined():
    subprocess.run(
        f"sed -i -e".split()
        + [
            "s/pub(crate) enabled_defined:/pub enabled_defined:/",
            f"{SCRIPT_DIR}/ip.rs",
        ],
        check=True,
    )


def expose_is_userspace():
    subprocess.run(
        f"sed -i -e".split()
        + [
            "s/pub(crate) fn is_userspace/pub fn is_userspace/",
            f"{SCRIPT_DIR}/iface.rs",
        ],
        check=True,
    )


def main():
    for file in os.listdir(NMSTATE_RUST_CODE_DIR):
        if file not in DENY_LIST:
            src_path = f"{NMSTATE_RUST_CODE_DIR}/{file}"
            dst_path = f"{SCRIPT_DIR}/{file}"
            if os.path.isdir(src_path):
                shutil.copytree(src_path, dst_path, dirs_exist_ok=True)
            else:
                shutil.copy(src_path, dst_path)

    remove_query_apply_net_state()
    fix_relative_import()
    exposing_merged_xxx()
    replace_nmstate_error_with_nipart_error()
    fix_merge_json_value()
    remove_feature_compile_line()
    change_merged_network_state_props_to_public()
    change_supported_list_to_public()
    change_prop_list_to_public()
    change_is_controller_to_public()
    change_merged_interfaces_props_to_public()
    change_merged_interface_props_to_public()
    change_func_ports_to_public()
    change_base_iface_props_to_public()
    serde_merged_state()
    expose_ip_enabled_defined()
    expose_is_userspace()
    expose_merged_xxx_is_changed_func()


main()
