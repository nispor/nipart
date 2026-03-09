# SPDX-License-Identifier: Apache-2.0

from .client import NipartClient
from .schema.state_option import NipartApplyOption


def apply(desired_state, *, verify_change=True):
    cli = NipartClient()
    opt = NipartApplyOption(verify_change=verify_change)
    cli.apply_network_state(desired_state, opt)
