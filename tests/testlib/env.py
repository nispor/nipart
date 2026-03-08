# SPDX-License-Identifier: Apache-2.0

import os
import pathlib
from .cmdlib import exec_cmd


def npt_path():
    project_dir = pathlib.Path(__file__).parent.parent.parent.resolve()
    return f"{project_dir}/target/debug/npt"


def has_kernel_module(name):
    try:
        exec_cmd(f"modprobe {name} -n".split())
        return True
    except Exception:
        return False
