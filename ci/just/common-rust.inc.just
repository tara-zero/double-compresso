# This is included into all justfiles where Rust recipes must be available.

[doc("[D] Run `cargo bloat` on release firmware")]
[group("Firmware for Raspberry Pi Pico 2 W")]
fw-pico2w-bloat *args:
    # TODO: https://github.com/casey/just/issues/1988
    @just _fw-pico2w-bloat "${@}"

[doc("[D] Build debug firmware")]
[group("Firmware for Raspberry Pi Pico 2 W")]
fw-pico2w-build: (_fw-pico2w-build "debug" "fw-debug")
    @echo
    @echo "Debug firmware:"
    @ls -lh \
        {{ rust_dir }}/target/thumbv8m.main-none-eabihf/fw-debug/*-fw \
        {{ rust_dir }}/target/thumbv8m.main-none-eabihf/fw-debug/*-fw.uf2
    @echo

[doc("[D] Build release firmware")]
[group("Firmware for Raspberry Pi Pico 2 W")]
fw-pico2w-build-release: (_fw-pico2w-build "info" "fw-release")
    @echo
    @echo "Release APK:"
    @ls -lh \
        {{ rust_dir }}/target/thumbv8m.main-none-eabihf/fw-release/*-fw \
        {{ rust_dir }}/target/thumbv8m.main-none-eabihf/fw-release/*-fw.uf2
    @echo

[doc("[D] Build debug CLI application for Linux")]
[group("CLI application")]
cli-build-linux: (_exe-build-linux "dev" cli_bin_name)
    @echo
    @echo "Linux Debug CLI binary:"
    @ls -lh \
        {{ rust_dir }}/target/debug/{{ cli_bin_name }}
    @echo

[doc("[D] Build release CLI application for Linux")]
[group("CLI application")]
cli-build-linux-release:  (_exe-build-linux "release" cli_bin_name)
    @echo
    @echo "Linux Release CLI binary:"
    @ls -lh \
        {{ rust_dir }}/target/release/{{ cli_bin_name }}
    @echo

[doc("[D] Build debug CLI application for Windows")]
[group("CLI application")]
cli-build-windows: (_exe-build-windows "dev" cli_bin_name)
    @echo
    @echo "Windows Debug CLI binary:"
    @ls -lh \
        {{ rust_dir }}/target/{{ rust_windows_target }}/debug/{{ cli_bin_name }}.exe
    @echo

[doc("[D] Build release CLI application for Windows")]
[group("CLI application")]
cli-build-windows-release: (_exe-build-windows "release" cli_bin_name)
    @echo
    @echo "Windows Release CLI binary:"
    @ls -lh \
        {{ rust_dir }}/target/{{ rust_windows_target }}/release/{{ cli_bin_name }}.exe
    @echo

[doc("[D] Build debug GUI application for Linux")]
[group("GUI application")]
gui-build-linux: (_exe-build-linux "dev" gui_bin_name)
    @echo
    @echo "Linux Debug GUI binary:"
    @ls -lh \
        {{ rust_dir }}/target/debug/{{ gui_bin_name }}
    @echo

[doc("[D] Build release GUI application for Linux")]
[group("GUI application")]
gui-build-linux-release: (_exe-build-linux "release" gui_bin_name)
    @echo
    @echo "Linux Release GUI binary:"
    @ls -lh \
        {{ rust_dir }}/target/release/{{ gui_bin_name }}
    @echo

[doc("[D] Build debug GUI application for Windows")]
[group("GUI application")]
gui-build-windows: (_exe-build-windows "dev" gui_bin_name)
    @echo
    @echo "Windows Debug GUI binary:"
    @ls -lh \
        {{ rust_dir }}/target/{{ rust_windows_target }}/debug/{{ gui_bin_name }}.exe
    @echo

[doc("[D] Build release GUI application for Windows")]
[group("GUI application")]
gui-build-windows-release: (_exe-build-windows "release" gui_bin_name)
    @echo
    @echo "Windows Release GUI binary:"
    @ls -lh \
        {{ rust_dir }}/target/{{ rust_windows_target }}/release/{{ gui_bin_name }}.exe
    @echo
