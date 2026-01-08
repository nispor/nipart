# SPDX-License-Identifier: Apache-2.0

from operator import itemgetter

import pytest

import nipart

from .testlib.cmdlib import exec_cmd
from .testlib.statelib import load_yaml
from .testlib.statelib import show_only
from .testlib.statelib import state_match


TEST_PORT1 = "dummy1"
TEST_PORT2 = "dummy2"
TEST_BOND_NIC = "bond99"


@pytest.fixture
def bond_over_dummy():
    nipart.apply(
        load_yaml(
            f"""---
            interfaces:
              - name: {TEST_BOND_NIC}
                type: bond
                state: up
                bond:
                  mode: active-backup
                  ports:
                    - {TEST_PORT2}
                    - {TEST_PORT1}
                  ports-config:
                  - name: {TEST_PORT1}
                    queue-id: 1
                    priority: 1
                  - name: {TEST_PORT2}
                    queue-id: 2
                    priority: 2
              - name: {TEST_PORT1}
                type: dummy
                state: up
              - name: {TEST_PORT2}
                type: dummy
                state: up
            """
        )
    )
    yield
    nipart.apply(
        load_yaml(
            f"""---
            interfaces:
              - name: {TEST_BOND_NIC}
                type: bond
                state: absent
              - name: {TEST_PORT1}
                type: dummy
                state: absent
              - name: {TEST_PORT2}
                type: dummy
                state: absent
            """
        )
    )


def test_create_and_remove_bond(bond_over_dummy):
    bond_iface = show_only(TEST_BOND_NIC)
    assert bond_iface["link-aggregation"]["mode"] == "active-backup"
    assert bond_iface["link-aggregation"]["port"] == [TEST_PORT1, TEST_PORT2]


def test_bond_change_mode(bond_over_dummy):
    state = load_yaml(
        f"""---
        interfaces:
          - name: {TEST_BOND_NIC}
            type: bond
            state: up
            bond:
              mode: 0
        """
    )
    nipart.apply(state)
    bond_iface = show_only(TEST_BOND_NIC)
    assert bond_iface["state"] == "up"
    assert bond_iface["link-aggregation"]["mode"] == "balance-rr"
    assert bond_iface["link-aggregation"]["port"] == [TEST_PORT1, TEST_PORT2]


def test_bond_change_port_config(bond_over_dummy):
    state = load_yaml(
        f"""---
        interfaces:
          - name: {TEST_BOND_NIC}
            type: bond
            state: up
            bond:
              ports-config:
              - name: {TEST_PORT1}
                queue-id: 0
                priority: 10
              - name: {TEST_PORT2}
                queue-id: 0
                priority: 20
        """
    )
    nipart.apply(state)
    bond_iface = show_only(TEST_BOND_NIC)
    assert bond_iface["state"] == "up"
    assert state_match(
        [
            {
                "name": TEST_PORT1,
                "queue-id": 0,
                "priority": 10,
            },
            {
                "name": TEST_PORT2,
                "queue-id": 0,
                "priority": 20,
            },
        ],
        bond_iface["link-aggregation"]["ports-config"],
    )
