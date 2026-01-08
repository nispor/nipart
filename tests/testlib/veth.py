# SPDX-License-Identifier: Apache-2.0

from contextlib import contextmanager

from nipart import NipartClient

from .cmdlib import exec_cmd
from .apply import nipart_apply


@contextmanager
def veth_interface(ifname, peer):
    nipart_apply(
        f"""---
        interfaces:
        - name: {ifname}
          type: veth
          veth:
            peer: {peer}
        """
    )
    try:
        yield
    finally:
        nipart_apply(
            f"""---
            interfaces:
            - name: {ifname}
              type: veth
              state: absent
            - name: {peer}
              type: veth
              state: absent
            """
        )
