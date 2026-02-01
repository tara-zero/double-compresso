# Global TODOs

* Fix lints when https://github.com/gradle/gradle/issues/35010 is fixed.
* Make `cyw43` initialization fallible to support both W and non-W Pi Pico:
  * https://github.com/embassy-rs/embassy/blob/bfa41e83183a9831bc3587ef2a0a84afded12dc9/cyw43/src/spi.rs#L164
* Speedup `cyw43` initialization by moving sleep into the loop:
  * https://github.com/embassy-rs/embassy/blob/bfa41e83183a9831bc3587ef2a0a84afded12dc9/cyw43/src/spi.rs#L162
* Write tool to reboot Pi Pico into BOOTSEL mode via `probe-rs`. Plus deploy firmware using the `picotool`
USB protocol:
  * https://github.com/Knight-Ops/usb-rp2040/tree/main
  * https://github.com/piersfinlayson/picoboot
  * https://github.com/NotQuiteApex/picoboot-rs/tree/main
  * rp4350 UART boot protocol?
* Detect Pi Pico type W vs. non-W:
  * https://datasheets.raspberrypi.com/picow/connecting-to-the-internet-with-pico-w.pdf
  * https://github.com/earlephilhower/arduino-pico/pull/1204/changes
  * https://github.com/earlephilhower/arduino-pico/issues/849#issuecomment-1436059239
* Design OTA firmware update protocol:
  * https://github.com/vovagorodok/ArduinoBleOTA/blob/0d714e64e891919462814622cc537a8b16282dff/tools/uploader.py
* Apply shell aliases to `just devsh ...`.
* Add `~/.inputrc` into the base image to disable audible bell.
* Change shell command history size inside the base image.
