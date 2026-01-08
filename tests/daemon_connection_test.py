# SPDX-License-Identifier: Apache-2.0

from nipart import NipartClient


def test_daemon_conn_ping():
    client = NipartClient()
    assert client.ping() == "pong"
