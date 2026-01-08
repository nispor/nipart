## SPDX-License-Identifier: Apache-2.0

from nipart import NipartClient

from .statelib import load_yaml


def nipart_apply(yaml_srt):
    cli = NipartClient()
    cli.apply_network_state(load_yaml(yaml_srt))
