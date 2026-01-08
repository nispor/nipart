# SPDX-License-Identifier: Apache-2.0

from ..client import NipartClient
from .state_option import NipartstateQueryOption
from .state_option import NipartstateStateKind


def show():
    client = NipartClient()
    opt = NipartstateQueryOption(kind=NipartstateStateKind.RUNNING)
    return client.query_network_state(opt)
