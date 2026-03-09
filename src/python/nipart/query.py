# SPDX-License-Identifier: Apache-2.0

from .client import NipartClient
from .schema.state_option import NipartQueryOption
from .schema.state_option import NipartStateKind


def show():
    client = NipartClient()
    opt = NipartQueryOption(kind=NipartStateKind.RUNNING)
    return client.query_network_state(opt)
