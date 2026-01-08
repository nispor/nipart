# SPDX-License-Identifier: Apache-2.0

import subprocess


def exec_cmd(cmd, check=True):
    p = subprocess.run(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=check,
    )

    stdout = p.stdout
    stderr = p.stderr
    rc = p.returncode

    return (rc, stdout.decode("utf-8"), stderr.decode("utf-8"))
