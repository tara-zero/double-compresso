"""
WSL (Windows Subsystem for Linux) utility functions.
"""


def is_wsl_path(full_path: str) -> bool:
    """
    Check if the given path is a WSL path.
    """

    return (
        full_path.startswith("\\\\wsl$\\")
        or full_path.startswith("//wsl$/")
        or full_path.startswith("\\\\wsl.localhost\\")
        or full_path.startswith("//wsl.localhost//")
    )
