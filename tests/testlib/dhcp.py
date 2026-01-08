# SPDX-License-Identifier: Apache-2.0

import os
import signal

import pytest

from .cmdlib import exec_cmd

DHCP_SRV_IP4_PREFIX = "192.0.2"
DHCP_SRV_IP6_PREFIX = "2001:db8:a"
DHCP_SRV_IP4 = f"{DHCP_SRV_IP4_PREFIX}.1"
DHCP_SRV_IP6 = f"{DHCP_SRV_IP6_PREFIX}::1"

IPV4_CLASSLESS_ROUTE_DST_NET1 = "198.51.100.0/24"
IPV4_CLASSLESS_ROUTE_NEXT_HOP1 = "192.0.2.1"
IPV6_CLASSLESS_ROUTE_PREFIX = "2001:db8:f"

DNSMASQ_CONF_PATH = "/tmp/nipart_test_dnsmasq.conf"
DNSMASQ_PID_PATH = "/tmp/nipart_test_dnsmasq.pid"

DHCP_SRV_NIC = "dhcp_srv"


def start_dhcp_server(net_ns):
    exec_cmd(
        f"ip netns exec {net_ns} "
        f"ip addr add {DHCP_SRV_IP4}/24 dev {DHCP_SRV_NIC}".split()
    )
    exec_cmd(
        f"ip netns exec {net_ns} "
        f"ip addr add {DHCP_SRV_IP6}/64 dev {DHCP_SRV_NIC}".split()
    )
    dnsmasq_conf = """
    leasefile-ro
    interface={iface}
    dhcp-range={ipv4_prefix}.200,{ipv4_prefix}.250,255.255.255.0,48h
    enable-ra
    dhcp-range={ipv6_prefix}::100,{ipv6_prefix}::fff,ra-names,slaac,64,480h
    dhcp-range={ipv6_classless_route}::100,{ipv6_classless_route}::fff,static
    dhcp-option=option:classless-static-route,{classless_rt},{classless_rt_dst}
    dhcp-option=option:dns-server,{v4_dns_server}
    """.format(
        **{
            "iface": DHCP_SRV_NIC,
            "ipv4_prefix": DHCP_SRV_IP4_PREFIX,
            "ipv6_prefix": DHCP_SRV_IP6_PREFIX,
            "classless_rt": IPV4_CLASSLESS_ROUTE_DST_NET1,
            "classless_rt_dst": IPV4_CLASSLESS_ROUTE_NEXT_HOP1,
            "v4_dns_server": DHCP_SRV_IP4,
            "ipv6_classless_route": IPV6_CLASSLESS_ROUTE_PREFIX,
        }
    )
    with open(DNSMASQ_CONF_PATH, "w") as fd:
        fd.write(dnsmasq_conf)

    exec_cmd(
        f"sudo ip netns exec {net_ns} dnsmasq "
        f"--interface={DHCP_SRV_NIC} --log-dhcp --pid-file={DNSMASQ_PID_PATH} "
        f"--conf-file={DNSMASQ_CONF_PATH} ".split()
    )


def stop_dhcp_server():
    with open(DNSMASQ_PID_PATH, "r") as fd:
        os.kill(int(fd.read()), signal.SIGTERM)
