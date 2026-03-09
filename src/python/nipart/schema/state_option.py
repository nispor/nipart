# SPDX-License-Identifier: Apache-2.0

import enum

from ..version import LATEST_SCHEMA_VERSION


class NipartStateKind(enum.StrEnum):
    RUNNING = "running-network-state"
    SAVED = "saved-network-state"
    DEFAULT = RUNNING


class NipartQueryOption:
    def __init__(
        self, version=LATEST_SCHEMA_VERSION, kind=NipartStateKind.DEFAULT
    ):
        self.version = version
        self.kind = kind

    def to_dict(self):
        return {"version": self.version, "kind": self.kind}

    def running():
        return NipartQueryOption(kind=NipartStateKind.RUNNING)

    def saved():
        return NipartQueryOption(kind=NipartStateKind.SAVED)


class NipartApplyOption:
    def __init__(self, version=LATEST_SCHEMA_VERSION, verify_change=True):
        self.version = version
        self.no_verify = not verify_change

    def to_dict(self):
        return {"version": self.version, "no-verify": self.no_verify}
