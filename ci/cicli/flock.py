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
        _acquire_lock(file_handle, lock_file_path)
        yield
    # Lock is automatically released when file is closed.


if sys.platform == "win32":
    import errno
    import logging
    import msvcrt
    import time

    from cicli.wsl import is_wsl_path

    _L = logging.getLogger(__name__)

    def _acquire_lock(file_handle, file_path):
        """
        Acquire lock on Windows using msvcrt.
        Uses non-blocking mode with manual retry to match POSIX blocking behavior.
        """

        file_handle.write(b"L")
        file_handle.flush()
        file_handle.seek(0)

        while True:
            try:
                # https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/locking?view=msvc-170
                msvcrt.locking(file_handle.fileno(), msvcrt.LK_NBLCK, 1)
                break
            except OSError as exc:
                if exc.errno in (errno.EACCES, errno.EDEADLOCK):
                    # Lock is held by another process, retry after a short delay.
                    time.sleep(0.1)
                elif (exc.errno == errno.EINVAL) and is_wsl_path(file_path):
                    _L.warning(
                        f'File locking on WSL shares does not work: "{file_path}"'
                    )
                    break
else:
    import fcntl

    def _acquire_lock(file_handle, _file_path):
        """
        Acquire lock on POSIX systems using fcntl.
        """

        fcntl.flock(file_handle.fileno(), fcntl.LOCK_EX)
