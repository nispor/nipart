# SPDX-License-Identifier: Apache-2.0

import json

from .schema.state_option import NipartApplyOption
from .schema.state_option import NipartQueryOption


class NipartCmdPing:
    IPC_KIND = "ping"

    def to_json(self):
        return json.dumps(
            {
                "kind": NipartCmdPing.IPC_KIND,
                "data": NipartCmdPing.IPC_KIND,
            }
        )


class NipartCmdQueryNetworkState:
    IPC_KIND = "query-network-state"

    def __init__(self, opt: NipartQueryOption):
        self.opt = opt

    def to_json(self):
        return json.dumps(
            {
                "kind": NipartCmdQueryNetworkState.IPC_KIND,
                "data": {
                    NipartCmdQueryNetworkState.IPC_KIND: self.opt.to_dict()
                },
            }
        )


class NipartCmdApplyNetworkState:
    IPC_KIND = "apply-network-state"

    def __init__(self, desired_state, opt: NipartApplyOption):
        self.desired_state = desired_state
        self.opt = opt

    def to_json(self):
        return json.dumps(
            {
                "kind": NipartCmdApplyNetworkState.IPC_KIND,
                "data": {
                    NipartCmdApplyNetworkState.IPC_KIND: (
                        self.desired_state,
                        self.opt.to_dict(),
                    )
                },
            }
        )
