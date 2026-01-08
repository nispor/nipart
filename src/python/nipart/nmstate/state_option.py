# SPDX-License-Identifier: Apache-2.0

import enum

from ..version import LATEST_SCHEMA_VERSION


class NipartstateStateKind(enum.StrEnum):
    RUNNING = "running-network-state"
    SAVED = "saved-network-state"
    DEFAULT = RUNNING


class NipartstateQueryOption:
    def __init__(
        self, version=LATEST_SCHEMA_VERSION, kind=NipartstateStateKind.DEFAULT
    ):
        self.version = version
        self.kind = kind

    def to_dict(self):
        return {"version": self.version, "kind": self.kind}

    def running():
        return NipartstateQueryOption(kind=NipartstateStateKind.RUNNING)

    def saved():
        return NipartstateQueryOption(kind=NipartstateStateKind.SAVED)


class NipartstateApplyOption:
    def __init__(self, version=LATEST_SCHEMA_VERSION, verify_change=True):
        self.version = version
        self.no_verify = not verify_change

    def to_dict(self):
        return {"version": self.version, "no-verify": self.no_verify}
