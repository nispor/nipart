# SPDX-License-Identifier: Apache-2.0

import os
import re
import signal

import nipart
import pytest

from .cmdlib import exec_cmd
from .retry import retry_till_true_or_timeout
from .dhcp import start_dhcp_server
from .dhcp import stop_dhcp_server
from .dhcp import DHCP_SRV_NIC

HWSIM0_PERM_MAC = "02:00:00:00:00:00"
HWSIM1_PERM_MAC = "02:00:00:00:01:00"
TEST_NET_NS = "wifi-test"
TEST_WIFI_SSID = "Test-WIFI"
TEST_WIFI_PSK = "12345678"
HOSTAPD_PID_PATH = "/tmp/nipart_test_hostapd.pid"
HOSTAPD_CONF_PATH = "/tmp/nipart_test_hostapd.conf"
HOSTAPD_CONF = f"""
interface={DHCP_SRV_NIC}
driver=nl80211

hw_mode=g
channel=1
ssid={TEST_WIFI_SSID}

wpa=2
wpa_key_mgmt=WPA-PSK
wpa_pairwise=CCMP
wpa_passphrase={TEST_WIFI_PSK}
"""
TIMEOUT_SECS_SIM_WIFI_NICS = 30
WIFI_TEST_NIC = "test-wlan0"


@pytest.fixture(scope="module")
def wifi_env():
    exec_cmd("modprobe -r mac80211_hwsim".split(), check=False)
    exec_cmd(f"ip netns del {TEST_NET_NS}".split(), check=False)
    exec_cmd(f"ip netns add {TEST_NET_NS}".split())

    exec_cmd("modprobe mac80211_hwsim radios=2".split())
    assert retry_till_true_or_timeout(
        TIMEOUT_SECS_SIM_WIFI_NICS, has_sim_wifi_nics
    )

    state = nipart.show()
    # The nipart.show() has started wpa_supplicant again, we need to
    # kill it so it does not hold outdated information on mac80211_hwsim
    # created temporary WIFI NIC.
    exec_cmd("killall wpa_supplicant".split(), check=False)
    wlan1 = get_nic_name_by_perm_mac(state, HWSIM0_PERM_MAC)
    exec_cmd(f"ip link set {wlan1} name {WIFI_TEST_NIC}".split())
    wlan2 = get_nic_name_by_perm_mac(state, HWSIM1_PERM_MAC)
    exec_cmd(f"ip link set {wlan2} name {DHCP_SRV_NIC}".split())
    start_hostapd()
    yield
    os.remove(HOSTAPD_CONF_PATH)
    if os.path.exists(HOSTAPD_PID_PATH):
        with open(HOSTAPD_PID_PATH) as fd:
            pid = fd.read()
        os.kill(int(pid), signal.SIGTERM)
    stop_dhcp_server()
    exec_cmd(f"ip netns del {TEST_NET_NS}".split())
    retry_till_true_or_timeout(10, unload_wifi_sim_kernel_module)


def unload_wifi_sim_kernel_module():
    try:
        exec_cmd("modprobe -r mac80211_hwsim".split())
        return True
    except Exception:
        return False


def get_nic_name_by_perm_mac(state, mac):
    for iface in state["interfaces"]:
        if iface.get("permanent-mac-address") == mac:
            return iface["name"]


def get_wifi_phy_name(nic_name):
    # TODO(Gris Ge): use nipart instead of iw here
    output = exec_cmd(f"iw dev {nic_name} info".split())[1]
    match = re.search(r"[^a-zA-Z]wiphy ([0-9]+)", output)
    assert match
    if match:
        return match.group(1)


def has_sim_wifi_nics():
    exec_cmd("udevadm settle".split())
    state = nipart.show()
    wlan1 = get_nic_name_by_perm_mac(state, HWSIM0_PERM_MAC)
    wlan2 = get_nic_name_by_perm_mac(state, HWSIM1_PERM_MAC)
    return wlan1 and wlan2


def start_hostapd():
    phy_id = get_wifi_phy_name(DHCP_SRV_NIC)
    assert phy_id
    # Move phy2 to namespace with hostpad
    exec_cmd(f"iw phy#{phy_id} set netns name {TEST_NET_NS}".split())
    exec_cmd(f"ip link set {WIFI_TEST_NIC} up".split())
    exec_cmd(
        f"ip netns exec {TEST_NET_NS} ip link set {DHCP_SRV_NIC} up".split()
    )
    with open(HOSTAPD_CONF_PATH, "w") as fd:
        fd.write(HOSTAPD_CONF)

    exec_cmd(
        f"ip netns exec {TEST_NET_NS} "
        f"hostapd -B -d {HOSTAPD_CONF_PATH} -P {HOSTAPD_PID_PATH}".split(),
    )

    assert retry_till_true_or_timeout(2, hostapd_is_up)

    start_dhcp_server(TEST_NET_NS)


def hostapd_is_up():
    output = exec_cmd(f"iw {WIFI_TEST_NIC} scan".split(), check=False)[1]
    return "Test-WIFI" in output
