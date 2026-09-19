#!/usr/bin/env bash
set -euo pipefail

VERSION="0.1.19"
APP="AetherForge-BEACN-Control-v${VERSION}"
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
VERIFY="$HOME/Downloads/${APP}-VERIFY.txt"
PROBE="$HOME/Downloads/${APP}-PROBE.txt"
BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
DOC_DIR="$HOME/.local/share/doc/aetherforge-beacn-control"
ROLLBACK_DIR="$HOME/.local/share/aetherforge-beacn-control/rollback/pre-v0.1.19"

exec > >(tee "$VERIFY") 2>&1

echo "AETHERFORGE_BEACN_VERSION=$VERSION"
echo "AETHERFORGE_BEACN_GATE_MODE=FAIL_FAST"
echo "AETHERFORGE_BEACN_UNDOCUMENTED_USB_WRITES=DISABLED"
echo "AETHERFORGE_BEACN_DARK_MODE=DRAGONGLASS"
echo "AETHERFORGE_BEACN_WINDOWS_UI_REFERENCE=BEACN_APP_1.4_CLEAN_ROOM"
echo "AETHERFORGE_BEACN_PRIVATE_DSP=APP_ISOLATED_PIPE_SOURCE"
echo "AETHERFORGE_BEACN_SYSTEM_DSP_INTEGRATION=FORBIDDEN"
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

echo "==> v0.1.19 adaptive BEACN interface contract"
./scripts/v0.1.19-interface-contract.sh src/main.rs src/layout.rs
echo "AETHERFORGE_BEACN_ADAPTIVE_UI_CONTRACT=PASS"

echo "==> v0.1.19 Windows BEACN layout / DragonGlass parity contract"
./scripts/v0.1.19-windows-parity-ui-contract.sh src/main.rs
echo "AETHERFORGE_BEACN_WINDOWS_PARITY_UI_CONTRACT=PASS"

echo "==> v0.1.19 canonical render-target contract"
./scripts/v0.1.19-render-target-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_RENDER_TARGET_CONTRACT=PASS"

echo "==> v0.1.19 Live Profiles / private software-DSP contract"
./scripts/v0.1.19-live-profile-dsp-contract.sh
echo "AETHERFORGE_BEACN_LIVE_PROFILE_DSP_CONTRACT=PASS"

echo "==> v0.1.19 Windows-style live DSP workflow contract"
./scripts/v0.1.19-windows-live-dsp-contract.sh
echo "AETHERFORGE_BEACN_WINDOWS_LIVE_DSP_CONTRACT=PASS"

echo "==> v0.1.19 private-DSP isolation contract"
./scripts/v0.1.19-private-dsp-isolation-contract.sh
echo "AETHERFORGE_BEACN_PRIVATE_DSP_ISOLATION_CONTRACT=PASS"

echo "==> v0.1.19 system-audio protection / isolation contract"
./scripts/v0.1.19-system-audio-protection-contract.sh
echo "AETHERFORGE_BEACN_SYSTEM_AUDIO_PROTECTION=PASS"

echo "==> v0.1.19 model-only hardware / private-DSP contract"
./scripts/v0.1.19-hardware-dsp-contract.sh
echo "AETHERFORGE_BEACN_HARDWARE_DSP_CONTRACT=PASS"

echo "==> v0.1.19 10-band EQ model parity regression contract"
./scripts/v0.1.19-eq-model-parity-contract.sh
echo "AETHERFORGE_BEACN_EQ_MODEL_PARITY_CONTRACT=PASS"
echo "AETHERFORGE_BEACN_HARDWARE_PROTOCOL=DIRECT_USB_TRANSPORT_REMOVED"
echo "AETHERFORGE_BEACN_DOCUMENTED_DSP_WRITES=DISABLED_PRIVATE_DSP_ONLY"
echo "AETHERFORGE_BEACN_SOFTWARE_DSP_PROFILE_MODEL=ACTIVE"
echo "AETHERFORGE_BEACN_SYSTEM_DSP_INTEGRATION=FORBIDDEN"

echo "==> v0.1.19 runtime audio-node contract"
./scripts/v0.1.19-runtime-audio-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_RUNTIME_AUDIO_CONTRACT=PASS"

echo "==> v0.1.19 BEACN output-profile recovery contract"
./scripts/v0.1.19-output-profile-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_OUTPUT_PROFILE_CONTRACT=PASS"

echo "==> v0.1.19 profile-repair invocation contract"
./scripts/v0.1.19-profile-repair-invocation-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_PROFILE_REPAIR_INVOCATION_CONTRACT=PASS"

echo "==> v0.1.19 graphical-session recovery contract"
./scripts/v0.1.19-graphical-session-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_GRAPHICAL_SESSION_CONTRACT=PASS"

echo "==> v0.1.19 egui 0.36 style API regression contract"
./scripts/v0.1.19-egui-style-api-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_EGUI_STYLE_API_CONTRACT=PASS"

for helper in pactl pw-record mkfifo; do
    if command -v "$helper" >/dev/null; then
        echo "AETHERFORGE_BEACN_PRIVATE_DSP_HELPER_${helper//-/_}=PASS:$(command -v "$helper")"
    else
        echo "AETHERFORGE_BEACN_PRIVATE_DSP_HELPER_${helper//-/_}=FAIL:MISSING"
        exit 1
    fi
done

if command -v systemctl >/dev/null; then
    echo "AETHERFORGE_BEACN_AUDIO_RECOVERY_SYSTEMCTL=PASS:$(command -v systemctl)"
else
    echo "AETHERFORGE_BEACN_AUDIO_RECOVERY_SYSTEMCTL=OPTIONAL_MISSING"
fi

echo "==> v0.1.19 strict-Clippy test-style regression contract"
./scripts/v0.1.19-clippy-test-style-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_CLIPPY_TEST_STYLE_CONTRACT=PASS"

for helper in pw-play timeout; do
    if command -v "$helper" >/dev/null; then
        echo "AETHERFORGE_BEACN_RECORDER_HELPER_${helper//-/_}=PASS:$(command -v "$helper")"
    else
        echo "AETHERFORGE_BEACN_RECORDER_HELPER_${helper//-/_}=OPTIONAL_MISSING"
    fi
done

echo "==> v0.1.19 private-audio strict-Clippy regression contract"
./scripts/v0.1.19-private-audio-clippy-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_PRIVATE_AUDIO_CLIPPY_CONTRACT=PASS"

echo "==> v0.1.19 render-navigation dead-code regression contract"
./scripts/v0.1.19-render-nav-clippy-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_RENDER_NAV_CLIPPY_CONTRACT=PASS"

echo "==> v0.1.19 desktop/GUI launch regression contract"
./scripts/v0.1.19-gui-launch-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_GUI_LAUNCH_CONTRACT=PASS"

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
if [[ "$test_count" != "26" ]]; then
    echo "AETHERFORGE_BEACN_CORE_TEST_COUNT=FAIL:expected=26:actual=$test_count"
    exit 1
fi
echo "AETHERFORGE_BEACN_CORE_TEST_COUNT=PASS:26"
echo "AETHERFORGE_BEACN_TEST=PASS"

echo "==> cargo build --release"
cargo build --release
echo "AETHERFORGE_BEACN_BUILD=PASS"

echo "==> private DSP pure-Rust self-test"
target/release/aetherforge-beacn-control --private-dsp-self-test
echo "AETHERFORGE_BEACN_PRIVATE_DSP_SELF_TEST=PASS"

echo "==> built GUI startup smoke test (display variables intentionally unset)"
GUI_SMOKE_OUTPUT="$(env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET -u DISPLAY target/release/aetherforge-beacn-control --gui-smoke-test 2>&1)"
printf '%s\n' "$GUI_SMOKE_OUTPUT"
grep -q '^AETHERFORGE_BEACN_GUI_SMOKE_TEST=PASS$' <<<"$GUI_SMOKE_OUTPUT" || {
    echo "AETHERFORGE_BEACN_BUILT_GUI_SMOKE_TEST=FAIL"
    exit 1
}
echo "AETHERFORGE_BEACN_BUILT_GUI_SMOKE_TEST=PASS"

mkdir -p "$ROLLBACK_DIR"
if [[ -f "$BIN_DIR/aetherforge-beacn-control" ]]; then
    cp -a "$BIN_DIR/aetherforge-beacn-control" "$ROLLBACK_DIR/aetherforge-beacn-control"
    echo "AETHERFORGE_BEACN_ROLLBACK_BINARY=PASS:$ROLLBACK_DIR/aetherforge-beacn-control"
else
    echo "AETHERFORGE_BEACN_ROLLBACK_BINARY=NOT_NEEDED"
fi
if [[ -f "$BIN_DIR/aetherforge-beacn-control-launch" ]]; then
    cp -a "$BIN_DIR/aetherforge-beacn-control-launch" "$ROLLBACK_DIR/aetherforge-beacn-control-launch"
    echo "AETHERFORGE_BEACN_ROLLBACK_LAUNCHER=PASS:$ROLLBACK_DIR/aetherforge-beacn-control-launch"
else
    echo "AETHERFORGE_BEACN_ROLLBACK_LAUNCHER=NOT_NEEDED"
fi
if [[ -f "$APP_DIR/aetherforge-beacn-control.desktop" ]]; then
    cp -a "$APP_DIR/aetherforge-beacn-control.desktop" "$ROLLBACK_DIR/aetherforge-beacn-control.desktop"
    echo "AETHERFORGE_BEACN_ROLLBACK_DESKTOP=PASS:$ROLLBACK_DIR/aetherforge-beacn-control.desktop"
else
    echo "AETHERFORGE_BEACN_ROLLBACK_DESKTOP=NOT_NEEDED"
fi

install -Dm755 target/release/aetherforge-beacn-control "$BIN_DIR/aetherforge-beacn-control"
DESKTOP_BINARY="$BIN_DIR/aetherforge-beacn-control"
DESKTOP_LAUNCHER="$BIN_DIR/aetherforge-beacn-control-launch"
LAUNCHER_TMP="$(mktemp)"
escaped_binary="${DESKTOP_BINARY//&/\&}"
sed "s|@AETHERFORGE_BEACN_BINARY@|$escaped_binary|g" packaging/aetherforge-beacn-control-launch > "$LAUNCHER_TMP"
chmod 755 "$LAUNCHER_TMP"
grep -Fq "BIN=\"$DESKTOP_BINARY\"" "$LAUNCHER_TMP" || {
    echo "AETHERFORGE_BEACN_LAUNCHER_BINARY=FAIL"
    rm -f "$LAUNCHER_TMP"
    exit 1
}
install -Dm755 "$LAUNCHER_TMP" "$DESKTOP_LAUNCHER"
rm -f "$LAUNCHER_TMP"
DESKTOP_TMP="$(mktemp)"
escaped_launcher="${DESKTOP_LAUNCHER//&/\&}"
sed "s|@AETHERFORGE_BEACN_LAUNCHER@|$escaped_launcher|g" packaging/aetherforge-beacn-control.desktop > "$DESKTOP_TMP"
grep -Fxq "Exec=$DESKTOP_LAUNCHER" "$DESKTOP_TMP" || {
    echo "AETHERFORGE_BEACN_DESKTOP_EXEC=FAIL"
    rm -f "$DESKTOP_TMP"
    exit 1
}
grep -Fxq "TryExec=$DESKTOP_LAUNCHER" "$DESKTOP_TMP" || {
    echo "AETHERFORGE_BEACN_DESKTOP_TRYEXEC=FAIL"
    rm -f "$DESKTOP_TMP"
    exit 1
}
install -Dm644 "$DESKTOP_TMP" "$APP_DIR/aetherforge-beacn-control.desktop"
rm -f "$DESKTOP_TMP"
echo "AETHERFORGE_BEACN_DESKTOP_EXEC=PASS:$DESKTOP_LAUNCHER"
install -Dm644 THIRD_PARTY_NOTICES.md "$DOC_DIR/THIRD_PARTY_NOTICES.md"
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
fi
echo "AETHERFORGE_BEACN_CLEAN_DESKTOP_INSTALL=PASS"

echo "==> installed GUI startup smoke test (display variables intentionally unset)"
INSTALLED_GUI_SMOKE_OUTPUT="$(env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET -u DISPLAY "$DESKTOP_LAUNCHER" --gui-smoke-test 2>&1)"
printf '%s\n' "$INSTALLED_GUI_SMOKE_OUTPUT"
grep -q '^AETHERFORGE_BEACN_GUI_SMOKE_TEST=PASS$' <<<"$INSTALLED_GUI_SMOKE_OUTPUT" || {
    echo "AETHERFORGE_BEACN_INSTALLED_GUI_SMOKE_TEST=FAIL"
    if [[ -f "$ROLLBACK_DIR/aetherforge-beacn-control" ]]; then
        cp -a "$ROLLBACK_DIR/aetherforge-beacn-control" "$BIN_DIR/aetherforge-beacn-control"
    fi
    if [[ -f "$ROLLBACK_DIR/aetherforge-beacn-control-launch" ]]; then
        cp -a "$ROLLBACK_DIR/aetherforge-beacn-control-launch" "$BIN_DIR/aetherforge-beacn-control-launch"
    else
        rm -f "$BIN_DIR/aetherforge-beacn-control-launch"
    fi
    if [[ -f "$ROLLBACK_DIR/aetherforge-beacn-control.desktop" ]]; then
        cp -a "$ROLLBACK_DIR/aetherforge-beacn-control.desktop" "$APP_DIR/aetherforge-beacn-control.desktop"
    fi
    exit 1
}
echo "AETHERFORGE_BEACN_INSTALLED_GUI_SMOKE_TEST=PASS"

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
