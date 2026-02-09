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


def test_wireguard_iface_minimal_config(clean_up):
    desired_state = load_yaml(
        f"""---
        interfaces:
          - name: {WG_TEST_NIC}
            type: wireguard
            state: up
            wireguard:
              private-key: "6LTHiAM4vgKEgi5vm30f/EBIEWFDmySkTc9EWCcIqEs="
              peers:
                - endpoint: 192.0.2.1:51821
                  public-key: "JKossUAjywXuJ2YVcaeD6PaHs+afPmIthDuqEVlspwA="
        """
    )
    nipart.apply(desired_state)


def test_wireguard_iface_with_fwmark(clean_up):
    desired_state = load_yaml(
        f"""---
        interfaces:
          - name: {WG_TEST_NIC}
            type: wireguard
            state: up
            wireguard:
              private-key: "6LTHiAM4vgKEgi5vm30f/EBIEWFDmySkTc9EWCcIqEs="
              fwmark: 42
              peers:
                - endpoint: 192.0.2.2:51822
                  public-key: "JKossUAjywXuJ2YVcaeD6PaHs+afPmIthDuqEVlspwA="
                  allowed-ips:
                  - ip: 192.168.100.0
                    prefix-length: 24
        """
    )
    nipart.apply(desired_state)


def test_wireguard_iface_multiple_peers(clean_up):
    desired_state = load_yaml(
        f"""---
        interfaces:
          - name: {WG_TEST_NIC}
            type: wireguard
            state: up
            wireguard:
              private-key: "6LTHiAM4vgKEgi5vm30f/EBIEWFDmySkTc9EWCcIqEs="
              listen-port: 51820
              peers:
                - endpoint: 192.0.2.10:51820
                  public-key: "JKossUAjywXuJ2YVcaeD6PaHs+afPmIthDuqEVlspwA="
                  allowed-ips:
                  - ip: 10.10.0.0
                    prefix-length: 24
                - endpoint: 192.0.2.11:51820
                  public-key: "8bdQrVLqiw3ZoHCucNh1YfH0iCWuyStniRr8t7H24Fk="
                  allowed-ips:
                  - ip: 10.11.0.0
                    prefix-length: 24
                  persistent-keepalive: 25
        """
    )
    nipart.apply(desired_state)


def test_wireguard_iface_remove_interface(clean_up):
    # First create the interface
    desired_state = load_yaml(
        f"""---
        interfaces:
          - name: {WG_TEST_NIC}
            type: wireguard
            state: up
            wireguard:
              private-key: "6LTHiAM4vgKEgi5vm30f/EBIEWFDmySkTc9EWCcIqEs="
              peers:
                - endpoint: 192.0.2.3:51823
                  public-key: "JKossUAjywXuJ2YVcaeD6PaHs+afPmIthDuqEVlspwA="
        """
    )
    nipart.apply(desired_state)
    
    # Then remove it
    remove_state = load_yaml(
        f"""---
        interfaces:
          - name: {WG_TEST_NIC}
            type: wireguard
            state: absent
        """
    )
    nipart.apply(remove_state)


def test_wireguard_iface_without_endpoint_fails():
    desired_state = load_yaml(
        f"""---
        interfaces:
          - name: {WG_TEST_NIC}
            type: wireguard
            state: up
            wireguard:
              private-key: "6LTHiAM4vgKEgi5vm30f/EBIEWFDmySkTc9EWCcIqEs="
              peers:
                - public-key: "JKossUAjywXuJ2YVcaeD6PaHs+afPmIthDuqEVlspwA="
        """
    )
    with pytest.raises(Exception):  # Expecting an error due to missing endpoint
        nipart.apply(desired_state)


def test_wireguard_iface_without_private_key_fails():
    desired_state = load_yaml(
        f"""---
        interfaces:
          - name: {WG_TEST_NIC}
            type: wireguard
            state: up
            wireguard:
              public-key: "JKossUAjywXuJ2YVcaeD6PaHs+afPmIthDuqEVlspwA="
              peers:
                - endpoint: 192.0.2.4:51824
                  public-key: "8bdQrVLqiw3ZoHCucNh1YfH0iCWuyStniRr8t7H24Fk="
        """
    )
    with pytest.raises(Exception):  # Expecting an error due to missing private key
        nipart.apply(desired_state)
