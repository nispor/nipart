# SPDX-License-Identifier: Apache-2.0

from ..client import NipartClient
from .state_option import NipartstateApplyOption


def apply(desired_state, *, verify_change=True):
    cli = NipartClient()
    opt = NipartstateApplyOption(verify_change=verify_change)
    cli.apply_network_state(desired_state, opt)
