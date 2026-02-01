# SPDX-License-Identifier: Apache-2.0 OR MIT

"""
Cross-platform file-based locking module.
"""

import sys
from contextlib import contextmanager


@contextmanager
def flock(lock_file_path: str):
    """
    Context manager for file-based locking.
    """

    with open(lock_file_path, "wb") as file_handle:
        _acquire_lock(file_handle)
        yield
    # Lock is automatically released when file is closed.


if sys.platform == "win32":
    import msvcrt
    import time

    def _acquire_lock(file_handle):
        """
        Acquire lock on Windows using msvcrt.
        Uses non-blocking mode with manual retry to match POSIX blocking behavior.
        """

        file_handle.write(b"L")
        file_handle.flush()
        file_handle.seek(0)

        while True:
            try:
                msvcrt.locking(file_handle.fileno(), msvcrt.LK_NBLCK, 1)
                break
            except OSError:
                # Lock is held by another process, retry after a short delay.
                time.sleep(0.1)
else:
    import fcntl

    def _acquire_lock(file_handle):
        """
        Acquire lock on POSIX systems using fcntl.
        """

        fcntl.flock(file_handle.fileno(), fcntl.LOCK_EX)
