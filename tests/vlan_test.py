# SPDX-License-Identifier: Apache-2.0

from operator import itemgetter

import pytest

import nipart

from .testlib.cmdlib import exec_cmd
from .testlib.statelib import load_yaml
from .testlib.statelib import show_only


TEST_BASE_NIC = "dummy1"
TEST_BASE_NIC2 = "dummy2"
TEST_VLAN_NIC = "dummy1.100"


@pytest.fixture
def vlan_over_dummy():
    nipart.apply(
        load_yaml(
            f"""---
            interfaces:
              - name: {TEST_VLAN_NIC}
                type: vlan
                state: up
                vlan:
                  id: 100
                  base-iface: {TEST_BASE_NIC}
              - name: {TEST_BASE_NIC}
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
              - name: {TEST_VLAN_NIC}
                type: vlan
                state: absent
              - name: {TEST_BASE_NIC}
                type: dummy
                state: absent
            """
        )
    )


def test_create_and_remove_vlan(vlan_over_dummy):
    vlan_iface = show_only(TEST_VLAN_NIC)
    assert vlan_iface["vlan"]["id"] == 100
    assert vlan_iface["vlan"]["base-iface"] == TEST_BASE_NIC


@pytest.fixture
def dummy2():
    nipart.apply(
        load_yaml(
            f"""---
            interfaces:
              - name: {TEST_BASE_NIC2}
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
              - name: {TEST_BASE_NIC2}
                type: dummy
                state: absent
            """
        )
    )


def test_vlan_change_property(vlan_over_dummy, dummy2):
    for prop_name, prop_value in [
        ("id", 101),
        ("base-iface", TEST_BASE_NIC2),
        ("protocol", "802.1ad"),
        ("protocol", "802.1q"),
        ("protocol", "802.1ad"),
        ("registration-protocol", "gvrp"),
        ("registration-protocol", "mvrp"),
        ("registration-protocol", "none"),
        ("registration-protocol", "mvrp"),
        ("registration-protocol", "gvrp"),
        ("reorder-headers", "true"),
        ("reorder-headers", "false"),
        ("reorder-headers", True),
        ("reorder-headers", False),
        ("loose-binding", "true"),
        ("loose-binding", "false"),
        ("loose-binding", True),
        ("loose-binding", False),
        ("bridge-binding", "true"),
        ("bridge-binding", "false"),
        ("bridge-binding", True),
        ("bridge-binding", False),
        ("ingress-qos-map", [{"from": 3, "to": 5}, {"from": 2, "to": 4}]),
        ("ingress-qos-map", [{"from": 3, "to": 7}, {"from": 2, "to": 6}]),
        ("egress-qos-map", [{"from": 1, "to": 2}, {"from": 3, "to": 6}]),
        ("egress-qos-map", [{"from": 1, "to": 3}, {"from": 3, "to": 7}]),
        ("ingress-qos-map", []),
        ("egress-qos-map", []),
    ]:
        print(f"Changing VLAN prop {prop_name} to {prop_value}")
        state = load_yaml(
            f"""---
                interfaces:
                  - name: {TEST_VLAN_NIC}
                    type: vlan
                    state: up
                """
        )
        state["interfaces"][0]["vlan"] = {prop_name: prop_value}
        nipart.apply(state)
        vlan_iface = show_only(TEST_VLAN_NIC)
        if prop_value == "true":
            prop_value = True
        if prop_value == "false":
            prop_value = False
        if prop_name in ["ingress-qos-map", "egress-qos-map"]:
            prop_value.sort(key=itemgetter("from"))
            if not prop_value:
                prop_value = None
        assert vlan_iface["state"] == "up"
        assert vlan_iface["vlan"].get(prop_name) == prop_value
