#!/usr/bin/env bash
set -euo pipefail

VERSION="0.1.13"
APP="AetherForge-BEACN-Control-v${VERSION}"
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
VERIFY="$HOME/Downloads/${APP}-VERIFY.txt"
PROBE="$HOME/Downloads/${APP}-PROBE.txt"
BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
DOC_DIR="$HOME/.local/share/doc/aetherforge-beacn-control"
ROLLBACK_DIR="$HOME/.local/share/aetherforge-beacn-control/rollback/pre-v0.1.13"

exec > >(tee "$VERIFY") 2>&1

echo "AETHERFORGE_BEACN_VERSION=$VERSION"
echo "AETHERFORGE_BEACN_GATE_MODE=FAIL_FAST"
echo "AETHERFORGE_BEACN_UNDOCUMENTED_USB_WRITES=DISABLED"
echo "AETHERFORGE_BEACN_DARK_MODE=DRAGONGLASS"
echo "AETHERFORGE_BEACN_WINDOWS_UI_REFERENCE=BEACN_APP_1.4_CLEAN_ROOM"
echo "AETHERFORGE_BEACN_PASS_THROUGH_MODE=ENFORCED"
echo "AETHERFORGE_BEACN_DIRECT_USB_CONTROL=BLOCKED_SYSTEM_AUDIO_PROTECTION"

command -v cargo >/dev/null
action_wpctl="$(command -v wpctl || true)"
if [[ -z "$action_wpctl" ]]; then
    echo "AETHERFORGE_BEACN_WPCTL=FAIL"
    exit 1
fi
echo "AETHERFORGE_BEACN_WPCTL=PASS:$action_wpctl"

cd "$ROOT"

echo "==> UI API compatibility contract"
./scripts/ui-api-contract.sh src/main.rs
echo "AETHERFORGE_BEACN_UI_API_CONTRACT=PASS"

echo "==> v0.1.13 adaptive BEACN interface contract"
./scripts/v0.1.13-interface-contract.sh src/main.rs src/layout.rs
echo "AETHERFORGE_BEACN_ADAPTIVE_UI_CONTRACT=PASS"

echo "==> v0.1.13 Windows BEACN layout / DragonGlass parity contract"
./scripts/v0.1.13-windows-parity-ui-contract.sh src/main.rs
echo "AETHERFORGE_BEACN_WINDOWS_PARITY_UI_CONTRACT=PASS"

echo "==> v0.1.13 Live Profiles / safe software-DSP contract"
./scripts/v0.1.13-live-profile-dsp-contract.sh
echo "AETHERFORGE_BEACN_LIVE_PROFILE_DSP_CONTRACT=PASS"

echo "==> v0.1.13 system-audio protection / pass-through contract"
./scripts/v0.1.13-system-audio-protection-contract.sh
echo "AETHERFORGE_BEACN_SYSTEM_AUDIO_PROTECTION=PASS"

echo "==> v0.1.13 staged hardware DSP contract"
./scripts/v0.1.13-hardware-dsp-contract.sh
echo "AETHERFORGE_BEACN_HARDWARE_DSP_CONTRACT=PASS"
echo "AETHERFORGE_BEACN_HARDWARE_PROTOCOL=BEACN_LIB_V0.4.3_PINNED_STAGED"
echo "AETHERFORGE_BEACN_DOCUMENTED_DSP_WRITES=STAGED_NOT_ACTIVE"
echo "AETHERFORGE_BEACN_SOFTWARE_DSP_PROFILE_MODEL=ACTIVE"
echo "AETHERFORGE_BEACN_AETHERSTREAM_MUTATING_DSP=CAPABILITY_GATED_NO_GUESSED_WIRE_PROTOCOL"

echo "==> v0.1.13 runtime audio-node contract"
./scripts/v0.1.13-runtime-audio-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_RUNTIME_AUDIO_CONTRACT=PASS"

echo "==> v0.1.13 BEACN output-profile recovery contract"
./scripts/v0.1.13-output-profile-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_OUTPUT_PROFILE_CONTRACT=PASS"

echo "==> v0.1.13 profile-repair invocation contract"
./scripts/v0.1.13-profile-repair-invocation-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_PROFILE_REPAIR_INVOCATION_CONTRACT=PASS"

echo "==> v0.1.13 graphical-session recovery contract"
./scripts/v0.1.13-graphical-session-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_GRAPHICAL_SESSION_CONTRACT=PASS"

echo "==> v0.1.13 egui 0.36 style API regression contract"
./scripts/v0.1.13-egui-style-api-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_EGUI_STYLE_API_CONTRACT=PASS"

if command -v pactl >/dev/null; then
    echo "AETHERFORGE_BEACN_PACTL=PASS:$(command -v pactl)"
else
    echo "AETHERFORGE_BEACN_PACTL=OPTIONAL_MISSING_PROFILE_RECOVERY_FALLBACK_WIREPLUMBER"
fi

if command -v systemctl >/dev/null; then
    echo "AETHERFORGE_BEACN_AUDIO_RECOVERY_SYSTEMCTL=PASS:$(command -v systemctl)"
else
    echo "AETHERFORGE_BEACN_AUDIO_RECOVERY_SYSTEMCTL=OPTIONAL_MISSING"
fi

echo "==> v0.1.13 strict-Clippy test-style regression contract"
./scripts/v0.1.13-clippy-test-style-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_CLIPPY_TEST_STYLE_CONTRACT=PASS"

for helper in pw-record pw-play timeout; do
    if command -v "$helper" >/dev/null; then
        echo "AETHERFORGE_BEACN_RECORDER_HELPER_${helper//-/_}=PASS:$(command -v "$helper")"
    else
        echo "AETHERFORGE_BEACN_RECORDER_HELPER_${helper//-/_}=OPTIONAL_MISSING"
    fi
done

echo "==> cargo fmt --all (single normalization pass)"
cargo fmt --all
echo "==> cargo fmt --all -- --check"
cargo fmt --all -- --check
echo "AETHERFORGE_BEACN_FMT=PASS"

echo "==> cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings"
cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings
echo "AETHERFORGE_BEACN_CLIPPY=PASS"

echo "==> cargo test --workspace --all-targets"
cargo test --workspace --all-targets
test_count="$(cargo test --test core -- --list 2>/dev/null | grep -c ": test$" || true)"
if [[ "$test_count" != "20" ]]; then
    echo "AETHERFORGE_BEACN_CORE_TEST_COUNT=FAIL:expected=20:actual=$test_count"
    exit 1
fi
echo "AETHERFORGE_BEACN_CORE_TEST_COUNT=PASS:20"
echo "AETHERFORGE_BEACN_TEST=PASS"

echo "==> cargo build --release"
cargo build --release
echo "AETHERFORGE_BEACN_BUILD=PASS"

mkdir -p "$ROLLBACK_DIR"
if [[ -f "$BIN_DIR/aetherforge-beacn-control" ]]; then
    cp -a "$BIN_DIR/aetherforge-beacn-control" "$ROLLBACK_DIR/aetherforge-beacn-control"
    echo "AETHERFORGE_BEACN_ROLLBACK_BINARY=PASS:$ROLLBACK_DIR/aetherforge-beacn-control"
else
    echo "AETHERFORGE_BEACN_ROLLBACK_BINARY=NOT_NEEDED"
fi
if [[ -f "$APP_DIR/aetherforge-beacn-control.desktop" ]]; then
    cp -a "$APP_DIR/aetherforge-beacn-control.desktop" "$ROLLBACK_DIR/aetherforge-beacn-control.desktop"
    echo "AETHERFORGE_BEACN_ROLLBACK_DESKTOP=PASS:$ROLLBACK_DIR/aetherforge-beacn-control.desktop"
else
    echo "AETHERFORGE_BEACN_ROLLBACK_DESKTOP=NOT_NEEDED"
fi

install -Dm755 target/release/aetherforge-beacn-control "$BIN_DIR/aetherforge-beacn-control"
install -Dm644 packaging/aetherforge-beacn-control.desktop "$APP_DIR/aetherforge-beacn-control.desktop"
install -Dm644 THIRD_PARTY_NOTICES.md "$DOC_DIR/THIRD_PARTY_NOTICES.md"

echo "==> graphical-session recovery probe (WAYLAND_DISPLAY/DISPLAY intentionally unset)"
env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET -u DISPLAY \
    "$BIN_DIR/aetherforge-beacn-control" --graphical-session-probe
echo "AETHERFORGE_BEACN_GRAPHICAL_SESSION_RECOVERY=PASS"

if command -v pactl >/dev/null; then
    echo "==> safe BEACN output-profile repair (no WirePlumber restart, no default-device change)"
    "$BIN_DIR/aetherforge-beacn-control" --repair-output-profile
else
    echo "AETHERFORGE_BEACN_OUTPUT_PROFILE_REPAIR=SKIP:PACTL_MISSING"
fi

"$BIN_DIR/aetherforge-beacn-control" --probe "$PROBE"
echo "AETHERFORGE_BEACN_PROBE=PASS:$PROBE"
echo "AETHERFORGE_BEACN_INSTALL=PASS"
echo "VERIFY_FILE=$VERIFY"
echo "PROBE_FILE=$PROBE"
