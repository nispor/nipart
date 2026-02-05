# SPDX-License-Identifier: Apache-2.0

import pytest

import nipart

from .testlib.cmdlib import exec_cmd
from .testlib.dhcp import DHCP_SRV_IP4
from .testlib.dhcp import DHCP_SRV_IP4_PREFIX
from .testlib.env import is_fedora
from .testlib.retry import retry_till_true_or_timeout
from .testlib.statelib import load_yaml


WG_TEST_NIC = "wg0"


@pytest.fixture
def clean_up():
    yield
    nipart.apply(
        load_yaml(
            f"""---
            interfaces:
              - name: {WG_TEST_NIC}
                type: wireguard
                state: absent"""
        )
    )


def test_wireguard_iface_static_ip(clean_up):
    desired_state = load_yaml(
        f"""---
        interfaces:
          - name: {WG_TEST_NIC}
            type: wireguard
            state: up
            wireguard:
              public-key: "JKossUAjywXuJ2YVcaeD6PaHs+afPmIthDuqEVlspwA="
              private-key: "6LTHiAM4vgKEgi5vm30f/EBIEWFDmySkTc9EWCcIqEs="
              listen-port: 51820
              peers:
                - endpoint: 192.0.2.0:51820
                  public-key: 8bdQrVLqiw3ZoHCucNh1YfH0iCWuyStniRr8t7H24Fk=
                  preshared-key: TqIkTsTSxWJ1vSnhUW2oXFAtB5l9hRFWdgn2BrKX3ik=
                  persistent-keepalive: 0
                  allowed-ips:
                  - ip: 0.0.0.0
                    prefix-length: 0
                  - ip: '::'
                    prefix-length: 0
                  protocol-version: 1
        """
    )
    nipart.apply(desired_state)
