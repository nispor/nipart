# SPDX-License-Identifier: Apache-2.0

import os
import pathlib


def is_fedora():
    return os.path.exists("/etc/fedora-release")


def npt_path():
    project_dir = pathlib.Path(__file__).parent.parent.parent.resolve()
    return f"{project_dir}/target/debug/npt"
