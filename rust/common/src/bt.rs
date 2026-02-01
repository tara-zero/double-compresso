// SPDX-License-Identifier: Apache-2.0 OR MIT

// TODO: Check that exactly one feature of ("btleplug", "trouble") is enabled.

/// Firmware version information and OTA update service.
/// Use this UUID to search for compatible devices.
pub const GATT_SERVICE_FW: [u8; 16] = uuid(0x6c5d6994_9333_46b5_8c8e_8cefab9effff);

/// Firmware version information and supported protocols.
/// A string consisting of comma-separated values: `$OTA,$FW,$PROTO`.
/// The values are:
/// - `$OTA`: OTA update and version info protocol version, unsigned integer.
/// - `$FW`: Firmware version, semver string.
/// - `$PROTO`: Control and data protocol version, unsigned integer.
pub const GATT_CHAR_FW_VER: [u8; 16] = uuid(0x6c5d6994_9333_46b5_8c8e_8cefab9e0000);

/// Current OTA update and version info protocol version.
///
/// Must be incremented before marking a stable release when OTA and version info protocol changes in an
/// incompatible way.
///
/// Avoid changing this value after the first stable release.
pub const CUR_OTA_VER: u8 = 0;

/// Current control and data protocol version.
///
/// Must be incremented before marking a stable release when control and data protocol changes in an
/// incompatible way.
pub const CUR_PROTO_VER: u8 = 0;

const fn uuid(uuid: u128) -> [u8; 16] {
    uuid.to_le_bytes()
}
