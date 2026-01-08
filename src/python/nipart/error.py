# SPDX-License-Identifier: Apache-2.0

import json


class NipartError(Exception):
    IPC_KIND = "error"

    def __init__(self, kind, msg):
        self.kind = kind
        self.msg = msg

    def to_json(self):
        return json.dumps(
            {
                "kind": self.kind,
                "msg": self.msg,
            }
        )

    def from_dict(data):
        match data["kind"]:
            case "invalid-argument":
                return NipartValueError(data["kind"], data["msg"])
            case _:
                return NipartError(data["kind"], data["msg"])


class NipartValueError(NipartError, ValueError):
    pass
