#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

required=(
  Cargo.toml
  crates/forgehx-core/src/lib.rs
  crates/forgehx-device/src/lib.rs
  crates/forgehx-device/src/hid.rs
  crates/forgehx-device/src/power.rs
  crates/forgehx-keyboard/src/lib.rs
  crates/forgehx-keyboard/src/registry.rs
  crates/forgehx-mouse/src/lib.rs
  crates/forgehx-mouse/src/registry.rs
  crates/forgehx-mouse/src/protocol/haste_v1.rs
  crates/forgehx-mouse/src/protocol/saga_pro.rs
  crates/forgehx-mic/src/lib.rs
  crates/forgehx-device/src/usb.rs
  crates/forgehx-device/src/classify.rs
  crates/forgehx-device/src/group.rs
  crates/forgehx-device/src/drivers.rs
  crates/forgehx-device/src/doctor.rs
  crates/forgehx-audio/src/lib.rs
  crates/forgehx-dsp/src/lib.rs
  crates/forgehx-dsp/src/engine.rs
  crates/forgehx-dsp/src/speaker_lock.rs
  crates/forgehx-dsp/src/transient.rs
  crates/forgehx-dsp/src/voicepilot.rs
  crates/forgehx-dsp/src/runtime.rs
  crates/forgehx-dsp/src/direct_pipewire.rs
  crates/forgehx-firmware/src/lib.rs
  crates/forgehx-firmware/src/package.rs
  crates/forgehx-firmware/src/registry.rs
  crates/forgehx-firmware/src/staging.rs
  crates/forgehx-firmware/src/transaction.rs
  crates/forgehx-backends/src/lib.rs
  crates/forgehx-backends/src/backend.rs
  crates/forgehx-backends/src/selector.rs
  crates/forgehx-backends/src/match_device.rs
  crates/forgehx-backends/src/openrgb.rs
  crates/forgehx-backends/src/ratbag.rs
  crates/forgehx-backends/src/services.rs
  crates/forgehx-ipc/src/lib.rs
  crates/forgehx-daemon/src/lib.rs
  crates/forgehx-daemon/src/main.rs
  crates/forgehx-cli/src/main.rs
  crates/forgehx-gui/src/main.rs
  crates/forgehx-gui/src/app.rs
  crates/forgehx-gui/src/device_page.rs
  crates/forgehx-gui/src/keyboard.rs
  crates/forgehx-gui/src/equalizer.rs
  crates/forgehx-gui/src/lighting.rs
  crates/forgehx-gui/src/mouse.rs
  crates/forgehx-gui/src/microphone.rs
  crates/forgehx-gui/src/firmware.rs
  crates/forgehx-gui/src/hardware.rs
  crates/forgehx-gui/src/settings.rs
  crates/forgehx-tray/Cargo.toml
  crates/forgehx-tray/src/main.rs
  packaging/udev/70-forgehx.rules
  forgehx.install
  scripts/refresh-haste-hid-access.sh
  packaging/systemd/forgehx-daemon.service
  packaging/systemd/forgehx-openrgb.service
  packaging/systemd/forgehx-tray.service
  packaging/systemd/forgehx-gui.service
  packaging/bin/forgehx-launch
  packaging/desktop/io.forgehx.ForgeHX.desktop
  PKGBUILD
  scripts/test-mic-input-stack.sh
  scripts/test-mic-daemon-stack.sh
  scripts/test-mic-cli-stack.sh
  scripts/test-mic-gui-stack.sh
  scripts/test-hyperx-mouse-matrix.sh
  scripts/test-hyperx-keyboard-matrix.sh
  scripts/test-hyperx-keyboard-gui.sh
  scripts/test-10.0.13-compile-regressions.sh
  scripts/test-10.0.12-warning-clean.sh
  scripts/test-10.0.12-direct-mic-path.py
  scripts/test-10.0.13-pipewire-registry-audio-discovery.py
  scripts/test-10.0.6-haste-discovery-lifetime.py
  scripts/test-10.0.6-haste-hid-access.py
  scripts/test-v116-click-suppression.py
  scripts/test-v121-target-hardware.py
  scripts/test-v121-full-target-support.py
  scripts/test-v122-continuous-voice-learning.py
  scripts/test-10.0.6-wired-mic-monitor.py
  scripts/test-v122-tray-background.py
  scripts/test-10.0.6-tray-process.py
  scripts/test-10.0.10-chat-routing.py
  scripts/test-10.0.10-communication-routing.py
  scripts/test-10.0.10-communication-source-eligibility.py
  scripts/test-10.0.10-stable-mic-identity.py
  scripts/test-10.0.10-persistent-processed-mic.py
  scripts/test-10.0.11-virtual-source-node-resolution.py
  scripts/test-v123-live-device-settings.py
  scripts/test-dsp-state-ownership.sh
  scripts/test_v110_hyperx_only_gui.py
  scripts/test_v110_hardware_settings.py
  scripts/test-v110-voicepilot-contract.py
  scripts/test-v110-voice-isolation.py
  scripts/test-v110-realtime-dsp.py
  scripts/test-v110-mic-gui.py
  scripts/test-v115-background-refresh-controls.py
  scripts/test-v115-live-active-controls.py
  scripts/test-v115-pipewire-target-fallback.py
  scripts/test-v115-user-authority.py
  scripts/test-v115-permanent-dsp.py
  scripts/test-10.0.13-base-release.sh
  scripts/test-10.0.14-base-release.sh
  scripts/test-10.0.16-base-release.sh
  scripts/test-10.0.14-compile-regressions.sh
  scripts/test-10.0.16-compile-regressions.sh
  scripts/test-10.0.14-direct-target-id.py
  scripts/test-10.0.16-private-interface.py
  scripts/test-10.0.16-recovery-release.sh
  README.md
  docs/HYPERX-MIC-FIRMWARE.md
  docs/TARGET-HARDWARE-10.0.10.md
)
for file in "${required[@]}"; do
  [[ -f "$file" ]] || { echo "missing required file: $file" >&2; exit 1; }
done

grep -qx 'Exec=forgehx-launch' packaging/desktop/io.forgehx.ForgeHX.desktop
grep -qx 'ExecStart=/usr/bin/forgehx-tray' packaging/systemd/forgehx-tray.service
grep -qx 'ExecStart=/usr/bin/forgehx-gui' packaging/systemd/forgehx-gui.service
grep -qx 'ExecStart=/usr/bin/forgehx-daemon' packaging/systemd/forgehx-daemon.service
grep -qx 'ExecStart=/usr/bin/openrgb --server --server-host 127.0.0.1 --server-port 6742' packaging/systemd/forgehx-openrgb.service
grep -q 'Wants=forgehx-openrgb.service' packaging/systemd/forgehx-daemon.service
grep -q 'ATTRS{idVendor}=="0951"' packaging/udev/70-forgehx.rules
grep -q 'ATTRS{idVendor}=="03f0"' packaging/udev/70-forgehx.rules
grep -q 'ATTRS{idVendor}=="03f0", ATTRS{idProduct}=="028e"' packaging/udev/70-forgehx.rules
grep -q 'ATTRS{idVendor}=="03f0", ATTRS{idProduct}=="048e"' packaging/udev/70-forgehx.rules
grep -q 'ATTRS{idProduct}=="028e", ENV{ID_USB_INTERFACE_NUM}=="02", MODE:="0666"' packaging/udev/70-forgehx.rules
grep -q 'ATTRS{idProduct}=="048e", ENV{ID_USB_INTERFACE_NUM}=="02", MODE:="0666"' packaging/udev/70-forgehx.rules
grep -Fqx 'hidapi = { version = "2", default-features = false, features = ["linux-native"] }' Cargo.toml
grep -q '^version = "10.0.16"$' Cargo.toml
grep -q '^pkgver=10.0.16$' PKGBUILD
grep -q '^pkgrel=1$' PKGBUILD
grep -q 'MicFirmwareGet' crates/forgehx-core/src/lib.rs
grep -q 'KeyboardCapabilities' crates/forgehx-core/src/lib.rs
grep -q 'KeyboardModelInfo' crates/forgehx-core/src/lib.rs
grep -q 'forgehx keyboard capabilities' README.md
grep -q 'Firmware inventory only' crates/forgehx-gui/src/firmware.rs
grep -q "'systemd'" PKGBUILD
grep -q "'openrgb'" PKGBUILD
grep -q "'libratbag'" PKGBUILD
grep -q "'pipewire-audio'" PKGBUILD
grep -q "'pipewire-pulse'" PKGBUILD
grep -q "'libpipewire'" PKGBUILD
grep -q "'libpulse'" PKGBUILD
grep -q '^sonora = "0.2"$' Cargo.toml
grep -q '^sonora.workspace = true$' crates/forgehx-dsp/Cargo.toml
if grep -qE "'noise-suppression-for-voice'|'lsp-plugins-lv2'" PKGBUILD; then
  echo 'legacy external mic DSP dependency found in PKGBUILD' >&2
  exit 1
fi
grep -q 'pub const IPC_PROTOCOL_VERSION: u32 = 11;' crates/forgehx-core/src/lib.rs
grep -q 'MicVoiceEnrollStart' crates/forgehx-core/src/lib.rs
grep -q 'pub struct SpeakerLockConfig' crates/forgehx-core/src/lib.rs
grep -q 'pub struct PlaybackRejectionConfig' crates/forgehx-core/src/lib.rs
grep -q 'pub const IPC_MIN_PROTOCOL_VERSION: u32 = 2;' crates/forgehx-core/src/lib.rs
grep -q 'BackendEnsure' crates/forgehx-core/src/lib.rs
grep -q 'BackendRestart' crates/forgehx-core/src/lib.rs
grep -q 'send_after_negotiation' crates/forgehx-ipc/src/lib.rs
grep -q 'Negotiate before sending the real command' crates/forgehx-ipc/src/lib.rs
grep -q 'BackendServiceManager' crates/forgehx-backends/src/services.rs
grep -q 'OpenRGB SDK is already reachable' crates/forgehx-backends/src/services.rs
grep -q 'WAS_ACTIVE=0' scripts/install-local.sh
grep -q 'server v{}.*negotiated v{}' crates/forgehx-daemon/src/lib.rs
grep -q 'const CLIENT_PROTOCOL_MAX: u32 = 5;' crates/forgehx-backends/src/openrgb.rs
grep -q 'server_version.min(CLIENT_PROTOCOL_MAX)' crates/forgehx-backends/src/openrgb.rs
grep -q 'pub const PROFILE_SCHEMA_VERSION: u32 = 2;' crates/forgehx-core/src/lib.rs
grep -q 'BackendKind::OpenRgb' crates/forgehx-daemon/src/lib.rs
grep -q 'BackendKind::Ratbag' crates/forgehx-daemon/src/lib.rs
grep -q 'hyperx-pulsefire-haste-wireless-v1' crates/forgehx-device/src/drivers.rs
grep -q 'HASTE_WIRELESS_PID: u16 = 0x028e' crates/forgehx-mouse/src/protocol/haste_v1.rs
grep -q 'HASTE_WIRED_PID: u16 = 0x048e' crates/forgehx-mouse/src/protocol/haste_v1.rs
grep -q 'SAGA_PRO_WIRED_PID: u16 = 0x04bf' crates/forgehx-mouse/src/protocol/saga_pro.rs
grep -q 'SAGA_PRO_WIRELESS_PID: u16 = 0x06bf' crates/forgehx-mouse/src/protocol/saga_pro.rs
grep -q 'PulsefireHasteWireless::from_device' crates/forgehx-daemon/src/lib.rs
grep -q 'target.object' crates/forgehx-audio/src/lib.rs
grep -q 'object.serial' crates/forgehx-audio/src/lib.rs
grep -q 'object.serial' crates/forgehx-dsp/src/lib.rs
grep -q 'pub const PROCESSED_SOURCE_PREFIX: &str = "forgehx_processed_mic";' crates/forgehx-dsp/src/lib.rs
grep -q 'run_direct_pipewire' crates/forgehx-dsp/src/runtime.rs
grep -q 'wait_for_source_node_id(raw_source)' crates/forgehx-dsp/src/direct_pipewire.rs
grep -q 'Some(raw_source_node_id)' crates/forgehx-dsp/src/direct_pipewire.rs
grep -q 'Audio/Source' crates/forgehx-dsp/src/direct_pipewire.rs
grep -q 'NODE_DESCRIPTION => "ForgeHX Mic"' crates/forgehx-dsp/src/direct_pipewire.rs
if grep -RniE 'pw-loopback|ForgeHX DSP Internal Injection|node\.hidden' crates/forgehx-dsp/src --include='*.rs'; then
  echo 'obsolete virtual microphone bridge marker found in ForgeHX DSP production source' >&2
  exit 1
fi
if grep -n 'Audio/Sink' crates/forgehx-dsp/src/direct_pipewire.rs; then
  echo 'direct microphone runtime must not publish an internal Audio/Sink' >&2
  exit 1
fi
if grep -RniE 'librnnoise_ladspa|noise_suppressor_mono|lsp-plug\.in/plugins/lv2' crates/forgehx-dsp/src --include='*.rs'; then
  echo 'legacy external microphone DSP implementation found' >&2
  exit 1
fi
if grep -RniE 'JamesDSP|jamesdsp|EasyEffects|easyeffects' crates/forgehx-dsp crates/forgehx-gui/src/microphone.rs; then
  echo 'output-oriented DSP runtime marker found in microphone stack' >&2
  exit 1
fi
./scripts/test-10.0.16-base-release.sh
python scripts/test-v110-voicepilot-contract.py
python scripts/test-v110-voice-isolation.py
python scripts/test-v116-click-suppression.py
python scripts/test-v121-target-hardware.py
python scripts/test-v121-full-target-support.py
python scripts/test-v122-continuous-voice-learning.py
python scripts/test-10.0.6-wired-mic-monitor.py
python scripts/test-v122-tray-background.py
python scripts/test-10.0.6-tray-process.py
python scripts/test-10.0.10-chat-routing.py
python scripts/test-10.0.10-communication-routing.py
python scripts/test-10.0.10-communication-source-eligibility.py
python scripts/test-10.0.10-stable-mic-identity.py
python scripts/test-10.0.10-persistent-processed-mic.py
python scripts/test-10.0.11-virtual-source-node-resolution.py
python scripts/test-v123-live-device-settings.py
python scripts/test-v124-haste-hidapi-framing.py
python scripts/test-v125-haste-direct-interface-fallback.py
python scripts/test-10.0.6-haste-discovery-lifetime.py
python scripts/test-10.0.6-haste-hid-access.py
python scripts/test-v110-realtime-dsp.py
python scripts/test-v110-mic-gui.py
python scripts/test-v115-background-refresh-controls.py
python scripts/test-v115-live-active-controls.py
python scripts/test-v115-pipewire-target-fallback.py
python scripts/test-v115-user-authority.py
python scripts/test-v115-permanent-dsp.py
./scripts/test-mic-input-stack.sh
./scripts/test-mic-daemon-stack.sh
./scripts/test-mic-cli-stack.sh
./scripts/test-mic-gui-stack.sh
./scripts/test-hyperx-mouse-matrix.sh
./scripts/test-hyperx-keyboard-matrix.sh
./scripts/test-hyperx-keyboard-gui.sh
./scripts/test-10.0.16-compile-regressions.sh
python scripts/test-10.0.12-direct-mic-path.py
python scripts/test-10.0.13-pipewire-registry-audio-discovery.py
./scripts/test-10.0.12-warning-clean.sh
./scripts/test-dsp-state-ownership.sh
python scripts/test_v110_hyperx_only_gui.py
python scripts/test_v110_hardware_settings.py
grep -q '127.0.0.1:6742' crates/forgehx-backends/src/openrgb.rs
if grep -n 'return Some(EqControl::' crates/forgehx-gui/src/equalizer.rs; then
  echo 'equalizer actions must escape egui closures through an outer action accumulator' >&2
  exit 1
fi
if grep -nE 'return Some\(MouseControl::(PollingRate|Profile)' crates/forgehx-gui/src/mouse.rs; then
  echo 'mouse actions must escape egui closures through an outer action accumulator' >&2
  exit 1
fi

if grep -n 'command.protocol_version() != IPC_PROTOCOL_VERSION' crates/forgehx-daemon/src/lib.rs; then
  echo 'exact IPC equality check would break protocol negotiation' >&2
  exit 1
fi

if grep -RniE 'send_feature_report\s*\(|bootloader[ _-]*write\s*\(|RawHidWrite|raw_hid_write\s*\(' crates --include='*.rs'; then
  echo 'unsafe firmware/raw-write API marker found in Rust source' >&2
  exit 1
fi
if grep -RniE 'Command::(Raw|Vendor|FeatureReport)' crates --include='*.rs'; then
  echo 'raw vendor write IPC marker found' >&2
  exit 1
fi

if grep -RniE 'Some\(\.[0-9]' crates --include='*.rs'; then
  echo 'Rust float literals must include an integer part (use 0.x, not .x)' >&2
  exit 1
fi

if find . -type f \( -iname '*.bin' -o -iname '*.fw' -o -iname '*.firmware' -o -iname '*.hex' \) -not -path './.git/*' | grep -q .; then
  echo 'firmware binary found in ForgeHX source/package payload' >&2
  exit 1
fi
bash scripts/check-firmware-manager.sh

echo 'ForgeHX 10.0.16 package layout checks passed.'
