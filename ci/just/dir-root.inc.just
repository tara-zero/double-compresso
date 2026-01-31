# This is included into source root justfiles.

export PYTHONPATH := justfile_directory() + "/ci" + if env("PYTHONPATH", "") == "" { "" } else { ":" + env("PYTHONPATH") }
src_root_dir := "."
kotlin_dir := "./kotlin"
rust_dir := "./rust"

[doc("[ ] Create the manifest for the base CI/devcontainer images and push it into registry")]
[group("Continuous integration")]
manifest-base-push:
    {{ cicli }} manifest-base-push

[doc("[ ] Create the manifest for the base CI/devcontainer images")]
[group("Continuous integration")]
manifest-base-create:
    {{ cicli }} manifest-base-create

[doc("[ ] Build base CI/devcontainer image and push it into registry")]
[group("Continuous integration")]
image-base-push:
    {{ cicli }} image-base-push

[doc("[ ] Build base CI/devcontainer image")]
[group("Continuous integration")]
image-base-build:
    {{ cicli }} image-base-build

[doc("[D] Run all amd64-only recipes that are normally run in CI")]
[group("Continuous integration")]
ci-all-amd64: \
        ci-all \
        apk-build \
        apk-build-release

[doc("[D] Run all cross-architecture recipes that are normally run in CI")]
[group("Continuous integration")]
ci-all: \
        fw-pico2w-build \
        fw-pico2w-build-release \
        cli-build-linux-release \
        cli-build-windows-release \
        gui-build-linux-release \
        gui-build-windows-release

[doc("[D] Remove all generated files, build artifacts, etc.")]
clean: kotlin-clean rust-clean
