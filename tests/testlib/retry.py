# SPDX-License-Identifier: Apache-2.0

import time


def retry_till_true_or_timeout(timeout, func, *args, **kwargs):
    max_timeout = timeout
    ret = func(*args, **kwargs)
    while timeout > 0:
        if ret:
            break
        print(f"{func.__name__} returned False: retrying " f"{timeout}/{max_timeout}")
        time.sleep(1)
        timeout -= 1
        ret = func(*args, **kwargs)
    return ret
