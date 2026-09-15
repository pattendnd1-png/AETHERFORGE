#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

grep -q '^version = "10.0.16"$' Cargo.toml
grep -q '^pkgver=10.0.16$' PKGBUILD
grep -q '^pkgrel=1$' PKGBUILD
grep -q 'pub const IPC_PROTOCOL_VERSION: u32 = 11;' crates/forgehx-core/src/lib.rs
grep -q 'MicFirmwareInventory' crates/forgehx-core/src/lib.rs
grep -q 'MicFirmwareUpdate' crates/forgehx-core/src/lib.rs
grep -q 'MicFirmwareRecovery' crates/forgehx-core/src/lib.rs
grep -q 'pub enum FirmwareSupportLevel' crates/forgehx-core/src/lib.rs
grep -q 'pub struct FirmwareIdentity' crates/forgehx-core/src/lib.rs
grep -q 'pub struct FirmwarePackageInfo' crates/forgehx-core/src/lib.rs
grep -q 'pub enum FirmwareValidation' crates/forgehx-core/src/lib.rs
grep -q 'pub enum FirmwareTransactionState' crates/forgehx-core/src/lib.rs
grep -q 'pub struct FirmwareTransactionStatus' crates/forgehx-core/src/lib.rs

if grep -RniE 'RawFirmware|FirmwareRaw|flash_raw|write_firmware_bytes|raw_firmware_write|FirmwarePacket|firmware_report_id' crates --include='*.rs'; then
  echo 'generic/raw firmware write API marker found' >&2
  exit 1
fi

echo 'ForgeHX 10.0.16 firmware core invariants passed.'

test -f crates/forgehx-firmware/Cargo.toml
test -f crates/forgehx-firmware/src/lib.rs
test -f crates/forgehx-firmware/src/package.rs
test -f crates/forgehx-firmware/src/registry.rs
test -f crates/forgehx-firmware/src/staging.rs
grep -q '"crates/forgehx-firmware"' Cargo.toml
grep -q 'pub struct FirmwareStager' crates/forgehx-firmware/src/staging.rs
grep -q 'pub struct FirmwareRegistry' crates/forgehx-firmware/src/registry.rs
grep -q 'vendor_id: 0x03f0, product_id: 0x0d84' crates/forgehx-firmware/src/registry.rs
grep -q 'vendor_id: 0x03f0, product_id: 0x02b5' crates/forgehx-firmware/src/registry.rs
if grep -nE 'vendor_id == 0x03f0[^&|]*$|vendor_id: 0x03f0, product_id: None' crates/forgehx-firmware/src/registry.rs; then
  echo 'broad HP VID firmware match found' >&2
  exit 1
fi
grep -q 'pub fn transition' crates/forgehx-firmware/src/transaction.rs
grep -q 'FirmwareTransactionState::Flashing' crates/forgehx-firmware/src/transaction.rs
grep -q 'UpdateNotEnabled' crates/forgehx-firmware/src/transaction.rs
grep -q 'pub fn save' crates/forgehx-firmware/src/transaction.rs
grep -q 'pub fn load' crates/forgehx-firmware/src/transaction.rs
if grep -nE 'load\([^)]*\).*transition\(FirmwareTransactionState::Flashing' crates/forgehx-firmware/src/transaction.rs; then
  echo 'firmware journal must never auto-resume flashing' >&2
  exit 1
fi
grep -q 'MicFirmwareGet' crates/forgehx-core/src/lib.rs
grep -q 'MicFirmwareStage' crates/forgehx-core/src/lib.rs
grep -q 'MicFirmwareValidate' crates/forgehx-core/src/lib.rs
grep -q 'MicFirmwareBegin' crates/forgehx-core/src/lib.rs
grep -q 'MicFirmwareStatus' crates/forgehx-core/src/lib.rs
grep -q 'MicFirmwareForget' crates/forgehx-core/src/lib.rs
grep -q 'FirmwareIdentity { identity:' crates/forgehx-core/src/lib.rs
grep -q 'FirmwarePackage { package:' crates/forgehx-core/src/lib.rs
grep -q 'FirmwareTransaction { status:' crates/forgehx-core/src/lib.rs
grep -q 'forgehx-firmware' crates/forgehx-daemon/Cargo.toml
grep -q 'fn mic_firmware_get' crates/forgehx-daemon/src/lib.rs
grep -q 'fn mic_firmware_stage' crates/forgehx-daemon/src/lib.rs
grep -q 'fn mic_firmware_begin' crates/forgehx-daemon/src/lib.rs
grep -q 'Capability::MicFirmwareInventory' crates/forgehx-daemon/src/lib.rs
grep -q 'protocol_version < 6' crates/forgehx-daemon/src/lib.rs
if grep -RniE 'Command::.*(RawFirmware|FirmwareRaw|FirmwarePacket)|raw.*firmware.*packet|firmware.*report_id' crates/forgehx-core crates/forgehx-daemon --include='*.rs'; then
  echo 'raw firmware IPC/daemon surface found' >&2
  exit 1
fi

grep -q 'enum MicFirmwareCommand' crates/forgehx-cli/src/main.rs
for cmd in Get Stage Validate Begin Status Forget; do grep -q "MicFirmwareCommand::${cmd}" crates/forgehx-cli/src/main.rs; done
grep -q 'mod firmware;' crates/forgehx-gui/src/main.rs
grep -q 'Self::Firmware => "Firmware"' crates/forgehx-gui/src/device_page.rs
grep -q 'Capability::MicFirmwareInventory' crates/forgehx-gui/src/device_page.rs
grep -q 'Firmware inventory only' crates/forgehx-gui/src/firmware.rs
grep -q 'SHA-256:' crates/forgehx-gui/src/firmware.rs
grep -q 'Update Firmware' crates/forgehx-gui/src/firmware.rs
grep -q 'identity.support_level.can_update() && package.validation == FirmwareValidation::Valid' crates/forgehx-gui/src/firmware.rs

echo 'ForgeHX 10.0.16 firmware CLI/GUI invariants passed.'
