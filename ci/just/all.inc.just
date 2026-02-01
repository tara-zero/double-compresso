# This is included into every justfile.

set shell := ["sh", "-ceu"]
set positional-arguments := true

slash := if os() == "windows" { "\\" } else { "/" }
pathsep := if os() == "windows" { ";" } else { ":" }

rust_windows_target := if arch() == "x86_64" { f"{{arch()}}-pc-windows-gnu" } else { f"{{arch()}}-pc-windows-gnullvm" }

alias help := list
alias tasks := list

[default]
[doc("[ ] List available recipes")]
list:
    @just --list
    @echo
    @echo "[/] - can only be run outside the devcontainer, may use devcontainer internally"
    @echo "[D] - can be run anywhere, will use devcontainer if outside"
    @echo "[ ] - can be run anywhere, does not require devcontainer"
