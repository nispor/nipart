# SPDX-License-Identifier: Apache-2.0

import pytest

import nipart

from .testlib.cmdlib import exec_cmd
from .testlib.dhcp import DHCP_SRV_IP4
from .testlib.dhcp import DHCP_SRV_IP4_PREFIX
from .testlib.env import is_fedora
from .testlib.retry import retry_till_true_or_timeout
from .testlib.statelib import load_yaml
from .testlib.wifi import TEST_WIFI_PSK
from .testlib.wifi import TEST_WIFI_SSID
from .testlib.wifi import WIFI_TEST_NIC
from .testlib.wifi import wifi_env


@pytest.fixture
def clean_up():
    yield
    nipart.apply(
        load_yaml(
            f"""---
            interfaces:
              - name: {WIFI_TEST_NIC}
                type: wifi-phy
                state: absent"""
        )
    )


def ping_peer():
    try:
        exec_cmd(f"ping {DHCP_SRV_IP4} -c 1 -w 5".split())
    except:
        return False
    return True


@pytest.mark.skipif(not is_fedora(), reason=("Only fedora has mac80211_hwsim module "))
class TestWifi:
    def test_wifi_iface_static_ip(self, wifi_env, clean_up):
        nipart.apply(
            load_yaml(
                f"""---
                interfaces:
                  - name: {WIFI_TEST_NIC}
                    type: wifi-phy
                    state: up
                    wifi:
                      ssid: {TEST_WIFI_SSID}
                      password: {TEST_WIFI_PSK}
                    ipv4:
                      enabled: true
                      dhcp: false
                      address:
                        - ip: {DHCP_SRV_IP4_PREFIX}.99
                          prefix-length: 24"""
            )
        )
        assert retry_till_true_or_timeout(5, ping_peer)

    def test_wifi_iface_dhcpv4(self, wifi_env, clean_up):
        nipart.apply(
            load_yaml(
                f"""---
                interfaces:
                  - name: {WIFI_TEST_NIC}
                    type: wifi-phy
                    state: up
                    wifi:
                      ssid: {TEST_WIFI_SSID}
                      password: {TEST_WIFI_PSK}
                    ipv4:
                      enabled: true
                      dhcp: true"""
            )
        )
        assert retry_till_true_or_timeout(5, ping_peer)
