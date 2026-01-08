# SPDX-License-Identifier: Apache-2.0


import pytest

from .testlib.cmdlib import exec_cmd
from .testlib.env import npt_path


@pytest.fixture
def veth1_down():
    exec_cmd("ip link del veth1".split(), check=False)
    exec_cmd("ip link add veth1 type veth peer veth1ep".split())
    yield
    exec_cmd("ip link del veth1".split())


def test_cli_wait_carrier_up(veth1_down):
    cli = npt_path()
    # Should get timeout on waiting veth1 up because both ends are down
    assert exec_cmd(f"{cli} wait veth1 up --timeout 1".split(), check=False)[0] != 0

    exec_cmd(f"ip link set veth1ep up".split())
    # Should get timeout on waiting veth1 up because other end is down
    assert exec_cmd(f"{cli} wait veth1 up --timeout 1".split(), check=False)[0] != 0

    exec_cmd(f"ip link set veth1ep down".split())
    exec_cmd(f"ip link set veth1 up".split())
    # Should get timeout on waiting veth1 up because local end is down
    assert exec_cmd(f"{cli} wait veth1 up --timeout 1".split(), check=False)[0] != 0

    exec_cmd(f"ip link set veth1ep up".split())
    exec_cmd(f"ip link set veth1 up".split())
    assert exec_cmd(f"{cli} wait veth1 up --timeout 5".split())[0] == 0
