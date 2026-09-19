#!/usr/bin/env bash
set -euo pipefail

VERSION="0.1.21"
APP="AetherForge-BEACN-Control-v${VERSION}"
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
VERIFY="$HOME/Downloads/${APP}-VERIFY.txt"
PROBE="$HOME/Downloads/${APP}-PROBE.txt"
BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
DOC_DIR="$HOME/.local/share/doc/aetherforge-beacn-control"
ROLLBACK_DIR="$HOME/.local/share/aetherforge-beacn-control/rollback/pre-v0.1.21"

exec > >(tee "$VERIFY") 2>&1

echo "AETHERFORGE_BEACN_VERSION=$VERSION"
echo "AETHERFORGE_BEACN_GATE_MODE=FAIL_FAST"
echo "AETHERFORGE_BEACN_UNDOCUMENTED_USB_WRITES=DISABLED"
echo "AETHERFORGE_BEACN_DARK_MODE=DRAGONGLASS"
echo "AETHERFORGE_BEACN_WINDOWS_UI_REFERENCE=BEACN_APP_1.4_CLEAN_ROOM"
echo "AETHERFORGE_BEACN_PRIVATE_DSP=APP_ISOLATED_PIPE_SOURCE"
echo "AETHERFORGE_BEACN_SYSTEM_DSP_INTEGRATION=FORBIDDEN"
echo "AETHERFORGE_BEACN_MIC_MEMORY=READ_ONLY_STARTUP_IMPORT"
echo "AETHERFORGE_BEACN_HARDWARE_WRITES=BLOCKED_SYSTEM_AUDIO_PROTECTION"

command -v cargo >/dev/null
action_wpctl="$(command -v wpctl || true)"
if [[ -z "$action_wpctl" ]]; then
    echo "AETHERFORGE_BEACN_WPCTL=FAIL"
    exit 1
fi
echo "AETHERFORGE_BEACN_WPCTL=PASS:$action_wpctl"

install_beacn_udev_rule_if_needed() {
    local have_rw=0
    local found=0
    local sys bus dev node
    for sys in /sys/bus/usb/devices/*; do
        [[ -f "$sys/idVendor" ]] || continue
        [[ "$(<"$sys/idVendor")" == "33ae" ]] || continue
        found=1
        [[ -f "$sys/busnum" && -f "$sys/devnum" ]] || continue
        bus="$(printf '%03d' "$((10#$(<"$sys/busnum")))")"
        dev="$(printf '%03d' "$((10#$(<"$sys/devnum")))")"
        node="/dev/bus/usb/$bus/$dev"
        if [[ -r "$node" && -w "$node" ]]; then
            have_rw=1
            break
        fi
    done

    if [[ "$have_rw" == "1" ]]; then
        echo "AETHERFORGE_BEACN_VENDOR_QUERY_PERMISSION=PASS:CURRENT_SESSION"
        return 0
    fi

    local rule_src="$ROOT/packaging/50-aetherforge-beacn.rules"
    local rule_dst="/etc/udev/rules.d/50-aetherforge-beacn.rules"
    [[ -f "$rule_src" ]] || { echo "AETHERFORGE_BEACN_UDEV_RULE=FAIL:MISSING_SOURCE"; return 1; }

    if [[ -f "$rule_dst" ]] && cmp -s "$rule_src" "$rule_dst"; then
        echo "AETHERFORGE_BEACN_UDEV_RULE=PASS:ALREADY_INSTALLED"
    elif [[ "$EUID" == "0" ]]; then
        install -Dm644 "$rule_src" "$rule_dst"
        echo "AETHERFORGE_BEACN_UDEV_RULE=PASS:INSTALLED_ROOT"
    elif command -v sudo >/dev/null 2>&1; then
        echo "==> BEACN vendor query permission needs the standard uaccess rule (one sudo prompt may appear)"
        sudo install -Dm644 "$rule_src" "$rule_dst"
        echo "AETHERFORGE_BEACN_UDEV_RULE=PASS:INSTALLED_SUDO"
    else
        if [[ "$found" == "1" ]]; then
            echo "AETHERFORGE_BEACN_UDEV_RULE=FAIL:NO_WRITE_PERMISSION_AND_NO_SUDO"
            return 1
        fi
        echo "AETHERFORGE_BEACN_UDEV_RULE=SKIP:NO_DEVICE_NO_SUDO"
        return 0
    fi

    if command -v udevadm >/dev/null 2>&1; then
        if [[ "$EUID" == "0" ]]; then
            udevadm control --reload-rules || true
            udevadm trigger --subsystem-match=usb --attr-match=idVendor=33ae || true
        elif command -v sudo >/dev/null 2>&1; then
            sudo udevadm control --reload-rules || true
            sudo udevadm trigger --subsystem-match=usb --attr-match=idVendor=33ae || true
        fi
        echo "AETHERFORGE_BEACN_UDEV_RELOAD=PASS_OR_REPLUG_IF_NEEDED"
    fi
}

cd "$ROOT"

echo "==> UI API compatibility contract"
./scripts/ui-api-contract.sh src/main.rs
echo "AETHERFORGE_BEACN_UI_API_CONTRACT=PASS"

echo "==> v0.1.21 On Device mic-memory startup contract"
./scripts/v0.1.21-on-device-profile-contract.sh
echo "AETHERFORGE_BEACN_ON_DEVICE_PROFILE_CONTRACT=PASS"

echo "==> BEACN vendor-query udev permission"
install_beacn_udev_rule_if_needed

echo "==> v0.1.21 adaptive BEACN interface contract"
./scripts/v0.1.21-interface-contract.sh src/main.rs src/layout.rs
echo "AETHERFORGE_BEACN_ADAPTIVE_UI_CONTRACT=PASS"

echo "==> v0.1.21 Windows BEACN layout / DragonGlass parity contract"
./scripts/v0.1.21-windows-parity-ui-contract.sh src/main.rs
echo "AETHERFORGE_BEACN_WINDOWS_PARITY_UI_CONTRACT=PASS"

echo "==> v0.1.21 canonical render-target contract"
./scripts/v0.1.21-render-target-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_RENDER_TARGET_CONTRACT=PASS"

echo "==> v0.1.21 Live Profiles / private software-DSP contract"
./scripts/v0.1.21-live-profile-dsp-contract.sh
echo "AETHERFORGE_BEACN_LIVE_PROFILE_DSP_CONTRACT=PASS"

echo "==> v0.1.21 Windows-style live DSP workflow contract"
./scripts/v0.1.21-windows-live-dsp-contract.sh
echo "AETHERFORGE_BEACN_WINDOWS_LIVE_DSP_CONTRACT=PASS"

echo "==> v0.1.21 private-DSP isolation contract"
./scripts/v0.1.21-private-dsp-isolation-contract.sh
echo "AETHERFORGE_BEACN_PRIVATE_DSP_ISOLATION_CONTRACT=PASS"

echo "==> v0.1.21 system-audio protection / isolation contract"
./scripts/v0.1.21-system-audio-protection-contract.sh
echo "AETHERFORGE_BEACN_SYSTEM_AUDIO_PROTECTION=PASS"

echo "==> v0.1.21 model-only hardware / private-DSP contract"
./scripts/v0.1.21-hardware-dsp-contract.sh
echo "AETHERFORGE_BEACN_HARDWARE_DSP_CONTRACT=PASS"

echo "==> v0.1.21 10-band EQ model parity regression contract"
./scripts/v0.1.21-eq-model-parity-contract.sh
echo "AETHERFORGE_BEACN_EQ_MODEL_PARITY_CONTRACT=PASS"
echo "AETHERFORGE_BEACN_HARDWARE_PROTOCOL=VENDOR_PARAMETER_QUERY_READ_ONLY"
echo "AETHERFORGE_BEACN_DOCUMENTED_DSP_WRITES=DISABLED_PRIVATE_DSP_ONLY"
echo "AETHERFORGE_BEACN_SOFTWARE_DSP_PROFILE_MODEL=ACTIVE"
echo "AETHERFORGE_BEACN_SYSTEM_DSP_INTEGRATION=FORBIDDEN"

echo "==> v0.1.21 runtime audio-node contract"
./scripts/v0.1.21-runtime-audio-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_RUNTIME_AUDIO_CONTRACT=PASS"

echo "==> v0.1.21 BEACN output-profile recovery contract"
./scripts/v0.1.21-output-profile-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_OUTPUT_PROFILE_CONTRACT=PASS"

echo "==> v0.1.21 profile-repair invocation contract"
./scripts/v0.1.21-profile-repair-invocation-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_PROFILE_REPAIR_INVOCATION_CONTRACT=PASS"

echo "==> v0.1.21 graphical-session recovery contract"
./scripts/v0.1.21-graphical-session-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_GRAPHICAL_SESSION_CONTRACT=PASS"

echo "==> v0.1.21 egui 0.36 style API regression contract"
./scripts/v0.1.21-egui-style-api-contract.sh "$ROOT"
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

echo "==> v0.1.21 strict-Clippy test-style regression contract"
./scripts/v0.1.21-clippy-test-style-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_CLIPPY_TEST_STYLE_CONTRACT=PASS"

for helper in pw-play timeout; do
    if command -v "$helper" >/dev/null; then
        echo "AETHERFORGE_BEACN_RECORDER_HELPER_${helper//-/_}=PASS:$(command -v "$helper")"
    else
        echo "AETHERFORGE_BEACN_RECORDER_HELPER_${helper//-/_}=OPTIONAL_MISSING"
    fi
done

echo "==> v0.1.21 private-audio strict-Clippy regression contract"
./scripts/v0.1.21-private-audio-clippy-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_PRIVATE_AUDIO_CLIPPY_CONTRACT=PASS"

echo "==> v0.1.21 render-navigation dead-code regression contract"
./scripts/v0.1.21-render-nav-clippy-contract.sh "$ROOT"
echo "AETHERFORGE_BEACN_RENDER_NAV_CLIPPY_CONTRACT=PASS"

echo "==> v0.1.21 desktop/GUI launch regression contract"
./scripts/v0.1.21-gui-launch-contract.sh "$ROOT"
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
