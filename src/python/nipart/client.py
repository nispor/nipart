# SPDX-License-Identifier: Apache-2.0

import json
import struct
import socket

from .cmd import NipartCmdApplyNetworkState
from .cmd import NipartCmdPing
from .cmd import NipartCmdQueryNetworkState
from .error import NipartError
from .log import NipartLogEntry
from .nmstate.state_option import NipartstateApplyOption
from .nmstate.state_option import NipartstateQueryOption

U32_MAX = 0xFFFFFFFF


class NipartIpcConnection:
    def __init__(self, path):
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.connect(path)

    def send(self, json_str):
        data_raw = json_str.encode("utf-8")
        length = len(data_raw) & U32_MAX
        length_raw = length.to_bytes(4, byteorder="big")
        self.socket.sendall(length_raw)
        self.socket.sendall(data_raw)

    def recv(self):
        # TODO(Gris Ge): handle timeout here
        while True:
            length_raw = self.socket.recv(4)
            if not length_raw:
                raise NipartError("BUG", "Got empty reply from daemon")
            length = int.from_bytes(length_raw, byteorder="big")
            reply = json.loads(self.socket.recv(length).decode("utf-8"))
            match reply["kind"]:
                case NipartError.IPC_KIND:
                    raise NipartError.from_dict(reply["data"])
                case NipartLogEntry.IPC_KIND:
                    log_entry = NipartLogEntry.from_dict(reply["data"])
                    log_entry.emit()
                case _:
                    return reply["data"]

    def exec(self, cmd):
        self.send(cmd.to_json())
        return self.recv()


DAEMON_SOCKET_PATH = "/var/run/nipart/sockets/daemon"


class NipartClient:
    def __init__(self):
        self._conn = NipartIpcConnection(DAEMON_SOCKET_PATH)

    def ping(self):
        return self._conn.exec(NipartCmdPing())

    def query_network_state(self, opt=None):
        if not opt:
            opt = NipartstateQueryOption()
        return self._conn.exec(NipartCmdQueryNetworkState(opt))

    def apply_network_state(self, desired_state, opt=None):
        if not opt:
            opt = NipartstateApplyOption()
        return self._conn.exec(NipartCmdApplyNetworkState(desired_state, opt))
