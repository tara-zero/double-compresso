# Development

## Dependencies

### Linux

```bash
#
# Fedora
#

# To build all kinds of binaries:
sudo dnf install git just podman
# To run firmware on real hardware:
sudo dnf install libusb1
# To run Android APK on real phone:
sudo dnf install android-tools
```

Note that `probe-rs` is required to work with firmware on real hardware and it may require additional setup,
see https://probe.rs/docs/getting-started/probe-setup/ for details.

For Android development, install Android Studio:

```bash
sudo flatpak install flathub com.google.AndroidStudio
# On systems with SELinux (like Fedora) you may need additional configuration:
# https://github.com/flathub/com.google.AndroidStudio/issues/234
```

### Windows

In PowerShell:

```bash
# Install Git:
winget install -e --id Git.Git
winget install -e --id Casey.Just

# Install Python:
python

# Install either `docker` (available only on x86_64):
winget install -e --id Docker.DockerDesktop
# or `podman`:
winget install -e --id RedHat.Podman-Desktop
```

Then run Docker Desktop or Podman Desktop and perform the necessary setup.

Then add Git Bash profile to Windows Terminal:
* click \[**V**\] button to the right of \[**+**\];
* click "Settings";
* click "Profiles" -> "Add new profile";
* change "Name" to "Git Bash";
* change "Command line" to "C:\Program Files\Git\bin\bash.exe --login";
* change "Starting directory" to "%USERPROFILE%" (uncheck "Use parent process directory");
* click "Save";

Now you can open Git Bash from the same \[**V**\] menu and customize the shell:

```bash
# Remove screen flickering on "notifications":
echo "set bell-style none" >>~/.inputrc
# Replace `none` with `audible` to enable audible bell.

# Do not automatically add `.exe`:
echo "shopt -s completion_strip_exe" >>~/.bash_profile
# Make completion case-sensitive:
echo "shopt -u nocasematch nocaseglob" >>~/.bash_profile

# Increase command history size:
echo "HISTSIZE=16384" >>~/.bash_profile
echo "HISTFILESIZE=16384" >>~/.bash_profile
# Do not record duplicates in command history:
echo "HISTCONTROL=ignoredups" >>~/.bash_profile

# Enable completion:
echo "source <(podman completion bash)" >>~/.bash_profile
echo "source <(just --completions bash)" >>~/.bash_profile
```

Finally, you can restart Git Bash, clone the repository and use `just` normally.

For Android development, install Android Studio:

```bash
winget install -e --id Google.AndroidStudio
```

## Commands

Most of the things are done inside the Development Container. Some tasks require hardware access (USB or Bluetooth),
so they are run various binaries outside the container.

For simplicity, we use the `just` tool to run typical tasks, including starting the development container if needed:

```bash
# Show available commands:
just

# Open shell inside the development container:
just devsh

# Run a shell command inside the development container:
just devsh ls -lah
```

### How to...

Update Gradle wrapper to a latest version:

```bash
just gradle wrapper --gradle-version "latest"
just gradle wrapper
```

Make `waydroid` window smaller:

```bash
waydroid prop set persist.waydroid.height 1800
waydroid prop set persist.waydroid.width 800
```

## Links

### Hardware

OLED used in prototype:

* https://www.waveshare.com/wiki/1.3inch_OLED_HAT
* https://files.waveshare.com/upload/c/c8/1.3inch-OLED-HAL-Schematic.pdf
