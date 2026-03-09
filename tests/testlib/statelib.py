# SPDX-License-Identifier: Apache-2.0

from collections.abc import Mapping
from collections.abc import Sequence

from nipart import NipartClient
from nipart import NipartStateKind
from nipart import NipartQueryOption
import yaml

RETRY_COUNT = 100


def show_only(iface_name, kind=NipartStateKind.RUNNING):
    client = NipartClient()
    state = client.query_network_state(NipartQueryOption(kind=kind))
    for iface in state["interfaces"]:
        if iface["name"] == iface_name:
            return iface


def show_saved_only(iface_name):
    client = NipartClient()
    state = client.query_network_state(NipartQueryOption.saved())
    for iface in state["interfaces"]:
        if iface["name"] == iface_name:
            return iface


def load_yaml(content):
    return yaml.load(content, Loader=yaml.SafeLoader)


def state_match(desire, current):
    if isinstance(desire, Mapping):
        return isinstance(current, Mapping) and all(
            state_match(val, current.get(key)) for key, val in desire.items()
        )
    elif isinstance(desire, Sequence) and not isinstance(desire, str):
        return (
            isinstance(current, Sequence)
            and not isinstance(current, str)
            and len(current) == len(desire)
            and all(state_match(d, c) for d, c in zip(desire, current))
        )
    else:
        return desire == current
