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
    "statistic",
)

SCRIPT_DIR = os.path.dirname(os.path.realpath(__file__))
NMSTATE_RUST_CODE_DIR = os.path.realpath(
    f"{SCRIPT_DIR}/../../../../nmstate/rust/src/lib"
)

def replace_crate_with_crate_state():
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + ["s/crate::/crate::state::/g", "{}", ";"],
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
            "s/use crate::state::state::merge_json_value/"
            "use super::json::merge_json_value/",
            f"{SCRIPT_DIR}/iface.rs",
        ],
        check=True,
    )
    subprocess.run(
        f"find {SCRIPT_DIR} -type f -name *.rs -exec sed -i -e".split()
        + [
            "s/state::get_json_value_difference/"
            "json::get_json_value_difference/g",
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


def main():
    for file in os.listdir(NMSTATE_RUST_CODE_DIR):
        if file not in DENY_LIST:
            src_path = f"{NMSTATE_RUST_CODE_DIR}/{file}"
            dst_path = f"{SCRIPT_DIR}/{file}"
            print("HAHA ", src_path)
            if os.path.isdir(src_path):
                shutil.copytree(src_path, dst_path, dirs_exist_ok=True)
            else:
                shutil.copy(src_path, dst_path)

    remove_query_apply_net_state()
    replace_crate_with_crate_state()
    exposing_merged_xxx()
    replace_nmstate_error_with_nipart_error()
    fix_merge_json_value()
    remove_feature_compile_line()


main()
