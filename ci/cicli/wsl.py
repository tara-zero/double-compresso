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


def share_to_local(full_path: str) -> str:
    """
    Convert a WSL share path to a local path inside the WSL filesystem.
    """

    # `\\wsl$\machine\local\path` -> `/local/path`
    slash = full_path[0]
    parts = full_path.split(slash)
    assert (len(parts) > 3) and parts[3], f'Incomplete WSL share path: "{full_path}"'
    parts = parts[4:]
    full_path = slash.join(parts)
    if not full_path.startswith(slash):
        full_path = slash + full_path
    if slash == "\\":
        full_path = full_path.replace("\\", "/")
    return full_path
