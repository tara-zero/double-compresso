# SPDX-License-Identifier: Apache-2.0 OR MIT

"""
CI CLI entry point.
"""
# TODO: Check with mypy and pylint.
# TODO: Ask to update devcontainer image when devcontainer.json changes.

import argparse
import datetime
import json
import logging
import os
import platform
import shutil
import subprocess
import sys
from base64 import b32encode
from copy import deepcopy
from os import path

try:
    # Requires Python >=3.11
    import tomllib
except ImportError:
    import tomli as tomllib

from cicli.flock import flock
from cicli.log import configure_logger
from cicli.wsl import is_wsl_path, share_to_local

_L = logging.getLogger(__name__)

_SRC_ROOT = path.dirname(path.dirname(path.dirname(path.abspath(__file__))))
_SRC_BASE_NAME = path.basename(_SRC_ROOT) if path.basename(_SRC_ROOT) else "src"
_WORKSPACE_SRC_ROOT = f"/workspaces/{_SRC_BASE_NAME}"

_SRC_ROOT_DEVCONTAINER_LOCK = path.join(_SRC_ROOT, ".cicli-devcontainer.lock")
_SRC_ROOT_CI = path.join(_SRC_ROOT, "ci")

_SRC_ROOT_CI_CONTAINER_BASE = path.join(_SRC_ROOT, "ci", "container", "base")
_SRC_ROOT_RUST_TOOLCHAIN_TOML = path.join(_SRC_ROOT, "rust", "rust-toolchain.toml")
_SRC_ROOT_ANDROID_TOOLCHAIN_TOML = path.join(
    _SRC_ROOT, "kotlin", "gradle", "toolchain.versions.toml"
)

_BASE_IMAGE_DEPS = (
    _SRC_ROOT_CI_CONTAINER_BASE,
    _SRC_ROOT_RUST_TOOLCHAIN_TOML,
    _SRC_ROOT_ANDROID_TOOLCHAIN_TOML,
)

_IMAGE_BASE_TAG = "ghcr.io/tara-zero/double-compresso-ci-base"
_IMAGE_ANNOTATIONS = (
    ("org.opencontainers.image.title", "DoubleCompresso CI Base Image"),
    (
        "org.opencontainers.image.description",
        "Dev Container/Base CI image for DoubleCompresso",
    ),
    (
        "org.opencontainers.image.source",
        "https://github.com/tara-zero/double-compresso",
    ),
)

_CONTAINER_NAME = "double-compresso-cicli-devcontainer-base32-" + b32encode(
    _SRC_ROOT.encode()
).decode().replace("=", "_")


def _main():
    """
    CI CLI main function.
    """

    arg_parser = argparse.ArgumentParser(
        description="CI CLI tool",
    )
    arg_parser.add_argument(
        "-l",
        "--log-level",
        action="store",
        default="info",
        type=str,
        choices=("debug", "info", "warning", "error", "critical"),
        required=False,
        help="Log level (default: %(default)s)",
    )
    arg_parser.add_argument(
        "-i",
        "--inside-devcontainer",
        action="store_true",
        default=False,
        required=False,
        help="Use when inside a devcontainer",
    )

    subparsers = arg_parser.add_subparsers(title="Subcommand")

    for name, subcommand, subcommand_args, inside_devcont in _SUBCOMMANDS:
        description = _get_subcommand_description(subcommand, inside_devcont)
        subparser = subparsers.add_parser(name, help=description)
        subcommand_args(subparser)
        subparser.set_defaults(subcommand_name=name)
        subparser.set_defaults(subcommand=subcommand)
        subparser.set_defaults(subcommand_inside_devcont=inside_devcont)

    args = arg_parser.parse_args()

    # TODO: Change to `arg_parser.add_subparsers(title="Subcommand", required=True)` after updating to newer Python.
    assert hasattr(args, "subcommand_name"), "Subcommand is not specified"

    configure_logger(args.log_level)

    if args.subcommand_inside_devcont is False:
        assert not args.inside_devcontainer, (
            f"Subcommand `{args.subcommand_name}` cannot be used inside a devcontainer"
        )
    elif args.subcommand_inside_devcont is True:
        assert args.inside_devcontainer, (
            f"Subcommand `{args.subcommand_name}` cannot be used outside a devcontainer"
        )

    args.subcommand(args)


def _get_subcommand_description(subcommand, inside_devcont):
    doc = subcommand.__doc__
    if doc is None:
        description = None
    else:
        description = doc.strip().split("\n", 1)[0].strip().rstrip(".")
        if inside_devcont is False:
            description += " [outside devcontainer]"
        elif inside_devcont is True:
            description += " [inside devcontainer]"
    return description


def _subcommand_devcmd(args):
    """
    Run command inside the devcontainer.
    """

    cmd = [args.command]
    cmd.extend(args.arguments)

    cont_cmd = _podman_or_docker()
    devcontainer = _read_devcontainer_json()
    _ensure_devcontainer_is_running(cont_cmd, devcontainer)
    _L.warning("Running inside devcontainer...")
    _devcontainer_run(
        cont_cmd,
        devcontainer,
        cmd,
        workdir=args.workdir,
        check=not args.ignore_return_code,
    )


def _subcommand_devcmd_args(subparser):
    """
    Add CLI arguments specific to the `devcmd` command.
    """

    subparser.add_argument(
        "-I",
        "--ignore-return-code",
        action="store_true",
        default=False,
        required=False,
        help="Ignore non-zero exit code",
    )
    subparser.add_argument(
        "-w",
        "--workdir",
        action="store",
        type=str,
        default=None,
        required=False,
        help="Working directory inside the container, either absolute path or relative to the project source directory",
    )
    subparser.add_argument(
        "command",
        metavar="COMMAND",
        action="store",
        help="Command to run inside the devcontainer",
    )
    subparser.add_argument(
        "arguments",
        metavar="ARG",
        nargs="*",
        action="store",
        help="Command arguments",
    )


def _podman_or_docker():
    """
    Return the podman or docker command based on the environment.
    """

    cmd = "podman"
    if shutil.which(cmd):
        return cmd
    cmd = "docker"
    if shutil.which(cmd):
        return cmd
    assert False, "Neither `podman` nor `docker` found"


def _ensure_devcontainer_is_running(cont_cmd: str, devcontainer):
    """
    Start a devcontainer in detached mode.
    """

    with flock(_SRC_ROOT_DEVCONTAINER_LOCK):
        _ensure_devcontainer_is_running_unlocked(cont_cmd, devcontainer)


def _ensure_devcontainer_is_running_unlocked(cont_cmd: str, devcontainer):
    """
    Start a devcontainer in detached mode. Without locking.
    """
    # TODO: Add command to clean/remove existing container (and maybe even image).

    image = devcontainer["image"]

    ps_json = subprocess.check_output((cont_cmd, "ps", "--all", "--format", "json"))
    ps = json.loads(ps_json)
    ps_devcontainer = None
    for container in ps:
        if _CONTAINER_NAME in container["Names"]:
            assert ps_devcontainer is None, "Multiple containers with the same name"
            ps_devcontainer = container

    if ps_devcontainer is not None and ps_devcontainer["Image"] != image:
        _L.warning("Devcontainer uses outdated image")

        if ps_devcontainer["State"] != "exited":
            cmd = (cont_cmd, "kill", _CONTAINER_NAME)
            _run(cmd)

            cmd = (cont_cmd, "wait", _CONTAINER_NAME)
            _run(cmd)

        cmd = (cont_cmd, "rm", _CONTAINER_NAME)
        _run(cmd)

        ps_devcontainer = None

    if ps_devcontainer is not None and ps_devcontainer["State"] != "running":
        _L.warning("Restarting existing devcontainer...")

        cmd = (cont_cmd, "restart", _CONTAINER_NAME)
        _run(cmd)

        _run_post_start_command(cont_cmd, devcontainer)

    if ps_devcontainer is None:
        _L.warning("Starting new devcontainer...")

        if is_wsl_path(_SRC_ROOT):
            # TODO: Test on Docker for Windows.
            src_root = share_to_local(_SRC_ROOT)
        else:
            src_root = _SRC_ROOT

        cmd = [
            cont_cmd,
            "run",
            "--name",
            _CONTAINER_NAME,
            "--detach",
            "--interactive",
            "--volume",
            f"{src_root}:{_WORKSPACE_SRC_ROOT}:bind,z",
        ]
        cmd.extend(devcontainer.get("runArgs", ()))
        cmd.extend(
            (
                image,
                "read",
                f"INFINITE_WAIT_FOR_{_CONTAINER_NAME.replace('-', '_')}",
            )
        )
        _run(cmd)

        _run_post_start_command(cont_cmd, devcontainer)


def _run_post_start_command(cont_cmd: str, devcontainer):
    """
    Run `postStartCommand` inside devcontainer.
    """

    post_start_command = devcontainer.get("postStartCommand")
    if post_start_command is not None:
        _L.warning("Running `postStartCommand`...")
        _devcontainer_run(cont_cmd, devcontainer, ("sh", "-c", post_start_command))


def _devcontainer_run(
    cont_cmd: str, devcontainer, command, workdir=None, env={}, check: bool = True
):
    """
    Execute a command inside the devcontainer.
    """

    env = deepcopy(env)
    for env_name, env_value in devcontainer.get("remoteEnv", {}).items():
        if env_value.startswith("${"):
            assert env_value.endswith("}"), (
                f'Invalid environment variable specification: "{env_value}" - "}}" is missing'
            )
            assert env_value.startswith("${localEnv:"), (
                f'Invalid environment variable specification: "{env_value}" - only "${{localEnv:VAR}}" is supported'
            )
            env_value = env_value[11:-1]
            env_value = os.environ.get(env_name)
            if env_value is not None:
                env[env_name] = env_value
        else:
            env[env_name] = env_value

    cmd = [
        cont_cmd,
        "exec",
        "--interactive",
    ]
    if sys.stdout.isatty():
        cmd.append("--tty")
    if workdir is not None:
        if not workdir.startswith("/"):
            if workdir == "":
                cur_dir = path.abspath(os.getcwd())
                assert cur_dir.startswith(_SRC_ROOT), (
                    f'Current directory "{cur_dir}" is not under "{_SRC_ROOT}"'
                )
                workdir = cur_dir[len(_SRC_ROOT) :]
                if path.sep != "/":
                    workdir = workdir.replace(path.sep, "/")
                workdir = workdir.rstrip("/")
                if workdir == "":
                    workdir = "."

            if workdir == ".":
                workdir = _WORKSPACE_SRC_ROOT
            else:
                workdir = f"{_WORKSPACE_SRC_ROOT}/{workdir}"
        cmd.extend(("--workdir", workdir))
    for env_name, env_value in env.items():
        cmd.extend(("--env", f"{env_name}={env_value}"))
    cmd.append(_CONTAINER_NAME)
    cmd.extend(command)

    _run(cmd, check)


def _read_devcontainer_json():
    with open(
        path.join(_SRC_ROOT, ".devcontainer", "devcontainer.json"), "r"
    ) as json_f:
        return json.load(json_f)


def _subcommand_manifest_base_push(args):
    """
    Create the manifest for the base CI/devcontainer images and push it to the registry.
    """

    _subcommand_manifest_base_create(args)

    manifest = _get_image_manifest()
    cmd = (
        "buildah",
        "manifest",
        "push",
        manifest,
    )
    _run(cmd)

    _L.info(f'Manifest "{manifest}" pushed')


def _subcommand_manifest_base_create(_args):
    """
    Create the manifest for the base CI/devcontainer images.
    """

    manifest = _get_image_manifest()

    cmd = (
        "buildah",
        "manifest",
        "exists",
        f"{manifest}",
    )
    if _run(cmd, check=False).returncode == 0:
        _L.info(f'Manifest "{manifest}" already exists, removing')

        cmd = (
            "buildah",
            "manifest",
            "rm",
            f"{manifest}",
        )
        _run(cmd)

    cmd = [
        "buildah",
        "manifest",
        "create",
    ]
    for name, value in _IMAGE_ANNOTATIONS:
        cmd.extend(("--annotation", f"{name}={value}"))
    cmd.append(manifest)
    for arch in ("amd64", "arm64"):
        cmd.append(f"{manifest}.{arch}")
    _run(cmd)

    _L.info(f'Manifest "{manifest}" created')


def _subcommand_image_base_push(args):
    """
    Build base CI/devcontainer image and push it to the registry.
    """

    arch = _get_image_arch()
    tag = _get_image_tag(arch)
    _image_base_build(args, arch, tag)

    cmd = (
        "buildah",
        "push",
        "--all",
        tag,
    )
    _run(cmd)

    _L.info(f'Image "{tag}" pushed to registry')


def _subcommand_image_base_build(args):
    """
    Build base CI/devcontainer image.
    """

    arch = _get_image_arch()
    tag = _get_image_tag(arch)
    _image_base_build(args, arch, tag)


def _image_base_build(args, arch: str, tag: str):
    """
    Build base CI/devcontainer image if it does not exist.
    """

    machine = {
        "amd64": "x86_64",
        "arm64": "aarch64",
    }[arch]
    if arch == "amd64":
        mingw_sysroot = f"/usr/{machine}-w64-mingw32/sys-root/mingw"
        mingw_rust_target = f"{machine}-pc-windows-gnu"
    else:
        mingw_sysroot = f"/opt/{machine}-w64-mingw32/{machine}-w64-mingw32"
        mingw_rust_target = f"{machine}-pc-windows-gnullvm"

    with open(_SRC_ROOT_RUST_TOOLCHAIN_TOML, "rb") as toml_f:
        rust_toolchain = tomllib.load(toml_f)
    rust_toolchain = rust_toolchain["toolchain"]

    with open(_SRC_ROOT_ANDROID_TOOLCHAIN_TOML, "rb") as toml_f:
        android_toolchain = tomllib.load(toml_f)
    android_toolchain = android_toolchain["versions"]

    cmd = (
        "buildah",
        "inspect",
        "--format",
        "{{ .ImageCreatedBy }}",
        f"{tag}",
    )
    if _run(cmd, check=False).returncode == 0:
        _L.info(f'Image "{tag}" already exists')
        return

    cmd = [
        "buildah",
        "build",
    ]
    if args.inside_devcontainer:
        cmd.extend(("--isolation", "chroot"))
    cmd.extend(
        (
            "--build-arg",
            f"mingw_sysroot={mingw_sysroot}",
            "--build-arg",
            f"mingw_rust_target={mingw_rust_target}",
            "--build-arg",
            f"rust_toolchain_version={rust_toolchain['channel']}",
            "--build-arg",
            f"rust_toolchain_profile={rust_toolchain['profile']}",
            "--build-arg",
            f"rust_toolchain_components={','.join(rust_toolchain['components'])}",
            "--build-arg",
            f"rust_toolchain_targets={' '.join(rust_toolchain['targets'])}",
            "--build-arg",
            f"android_build_tools_version={android_toolchain['android_build_tools_version']}",
            "--build-arg",
            f"android_platform_version={android_toolchain['android_platform_version']}",
            "--build-arg",
            f"android_platform_tools_version={android_toolchain['android_platform_tools_version']}",
            "--platform",
            f"linux/{arch}",
            "--tag",
            tag,
        )
    )
    for name, value in _IMAGE_ANNOTATIONS:
        cmd.extend(("--annotation", f"{name}={value}"))
    cmd.append(_SRC_ROOT_CI_CONTAINER_BASE)
    _run(cmd)

    _L.info(f'Built image "{tag}"')


def _no_args(_subparser):
    """
    Do not add any arguments to the subparser.
    """


_SUBCOMMANDS = (
    (
        "devcmd",
        _subcommand_devcmd,
        _subcommand_devcmd_args,
        False,
    ),
    ("manifest-base-push", _subcommand_manifest_base_push, _no_args, None),
    ("manifest-base-create", _subcommand_manifest_base_create, _no_args, None),
    ("image-base-push", _subcommand_image_base_push, _no_args, None),
    ("image-base-build", _subcommand_image_base_build, _no_args, None),
)


def _get_image_tag(arch: str):
    """
    Get the container image tag name.
    """

    return f"{_get_image_manifest()}.{arch}"


def _get_image_manifest():
    """
    Get the multi-architecture manifest name.
    """

    branch = _git_branch_name()
    ts = _git_commit_timestamp(_BASE_IMAGE_DEPS)
    return f"{_IMAGE_BASE_TAG}:{branch}-{ts.strftime('%Y-%m-%d')}"


def _git_branch_name():
    """
    Get the name of the current git branch.
    """

    return (
        _run(("git", "rev-parse", "--abbrev-ref", "HEAD"), capture_stdout=True)
        .stdout.decode()
        .strip()
    )


def _git_commit_timestamp(paths):
    """
    Get the timestamp of the last git commit.
    """

    cmd = [
        "git",
        "log",
        "-1",
        "--format=%ct",
        "--",
    ]
    cmd.extend(paths)
    unix_ts = int(_run(cmd, capture_stdout=True).stdout.decode().strip())
    return datetime.datetime.fromtimestamp(unix_ts, datetime.timezone.utc)


def _get_image_arch():
    """
    Get the container image architecture matching the current host.
    """

    machine = platform.machine().lower()
    arch = {
        "x86_64": "amd64",
        "aarch64": "arm64",
    }.get(machine)
    assert arch is not None, f'Unsupported architecture: "{machine}"'
    return arch


def _run(command, check: bool = True, capture_stdout: bool = False):
    """
    Execute a command and check if it succeeds.
    """

    _L.info(f"Executing command {command}")
    try:
        return subprocess.run(
            command, stdout=subprocess.PIPE if capture_stdout else None, check=check
        )
    except subprocess.CalledProcessError as e:
        _L.critical(f"Command {command} failed with exit code {e.returncode}")
        sys.exit(1)


if __name__ == "__main__":
    _main()
