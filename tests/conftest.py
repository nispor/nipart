# SPDX-License-Identifier: Apache-2.0

import pathlib
import subprocess
import sys
import time

import pytest

from .testlib.cmdlib import exec_cmd
from .testlib.retry import retry_till_true_or_timeout

project_dir = pathlib.Path(__file__).parent.parent.resolve()
sys.path.insert(0, f"{project_dir}/src/python")

from nipart import NipartClient

DAEMON_LOG = "/tmp/nipart_test_daemon.log"
CLI_PATH = f"{project_dir}/target/debug/npt"


@pytest.fixture(scope="session", autouse=True)
def test_env_setup(run_daemon):
    yield


@pytest.fixture(scope="session")
def run_daemon():
    bin_path = pathlib.Path(f"{project_dir}/target/debug/nipartd").resolve()
    process = subprocess.Popen(
        bin_path, stdout=sys.stdout, stderr=open(DAEMON_LOG, "w")
    )
    # Wait daemon to start up
    time.sleep(1)
    retry_till_true_or_timeout(30, check_daemon_connection)
    yield
    if process:
        process.terminate()


def check_daemon_connection():
    try:
        client = NipartClient()
        return client.ping() == "pong"
    except:
        return False


REPORT_HEADER = """OS: {osname}
Kernel: {kernel_ver}
"""


def _get_osname():
    with open("/etc/os-release") as os_release:
        for line in os_release.readlines():
            if line.startswith("PRETTY_NAME="):
                return line.split("=", maxsplit=1)[1].strip().strip('"')
    return ""


def _get_kernel_ver():
    return exec_cmd("uname -r".split())[1]


def pytest_report_header(config):
    return REPORT_HEADER.format(
        osname=_get_osname(),
        kernel_ver=_get_kernel_ver(),
    )
