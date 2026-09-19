#!/usr/bin/env bash
set -euo pipefail

VERSION="0.1.23"
APP_ID="aetherforge-beacn-control"
APP_NAME="AetherForge BEACN Control"
BIN="$HOME/.local/bin/$APP_ID"
LAUNCHER="$HOME/.local/bin/${APP_ID}-launch"
APP_DIR="$HOME/.local/share/applications"
DESKTOP="$APP_DIR/${APP_ID}.desktop"
DATA_ROOT="$HOME/.local/share/$APP_ID"
DOC_DIR="$HOME/.local/share/doc/$APP_ID"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/$APP_ID"
STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/$APP_ID"
VERIFY="$HOME/Downloads/AetherForge-BEACN-Control-v${VERSION}-VERIFY.txt"
PROBE="$HOME/Downloads/AetherForge-BEACN-Control-v${VERSION}-PROBE.txt"
LOG="$HOME/Downloads/AetherForge-BEACN-Control-v${VERSION}-FINAL-INSTALL.txt"
RULE_DST="/etc/udev/rules.d/50-aetherforge-beacn.rules"

exec > >(tee "$LOG") 2>&1

echo "AETHERFORGE_BEACN_FINALIZE_VERSION=$VERSION"
echo "AETHERFORGE_BEACN_FINALIZE_MODE=FULL_USER_INSTALL"

have_host_pass() {
    [[ -f "$VERIFY" ]] \
        && grep -Fxq 'AETHERFORGE_BEACN_CLIPPY=PASS' "$VERIFY" \
        && grep -Fxq 'AETHERFORGE_BEACN_TEST=PASS' "$VERIFY" \
        && grep -Fxq 'AETHERFORGE_BEACN_BUILD=PASS' "$VERIFY" \
        && grep -Fxq 'AETHERFORGE_BEACN_INSTALL=PASS' "$VERIFY"
}

bootstrap_if_needed() {
    if [[ -x "$BIN" ]] && have_host_pass; then
        echo "AETHERFORGE_BEACN_QUALIFIED_BINARY=PASS:$BIN"
        return 0
    fi

    local hit="$HOME/Downloads/HIT-IT-AETHERFORGE-BEACN-v${VERSION}.sh"
    local src="$HOME/Downloads/AetherForge-BEACN-Control-v${VERSION}/INSTALL-AND-VERIFY.sh"
    if [[ -x "$hit" ]]; then
        echo "==> Qualified install is missing/incomplete; running v${VERSION} HIT-IT gate"
        "$hit"
    elif [[ -x "$src" ]]; then
        echo "==> Qualified install is missing/incomplete; running source host gate"
        "$src"
    else
        echo "AETHERFORGE_BEACN_FINALIZE=FAIL:NO_QUALIFIED_INSTALL_OR_INSTALLER"
        echo "Expected either $BIN + $VERIFY, $hit, or $src"
        exit 1
    fi

    [[ -x "$BIN" ]] || { echo "AETHERFORGE_BEACN_FINALIZE=FAIL:BINARY_NOT_INSTALLED"; exit 1; }
    have_host_pass || { echo "AETHERFORGE_BEACN_FINALIZE=FAIL:HOST_GATE_NOT_GREEN"; exit 1; }
    echo "AETHERFORGE_BEACN_QUALIFIED_BINARY=PASS:$BIN"
}

install_udev_rule() {
    local tmp
    tmp="$(mktemp)"
    cat > "$tmp" <<'RULE'
# AetherForge BEACN Control — vendor parameter query permission.
# Application policy remains getter-only for mic-memory import; hardware setters are blocked.
SUBSYSTEM=="usb", ATTR{idVendor}=="33ae", ATTR{idProduct}=="8001", TAG+="uaccess"
SUBSYSTEM=="usb", ATTR{idVendor}=="33ae", ATTR{idProduct}=="0001", TAG+="uaccess"
RULE

    if [[ -f "$RULE_DST" ]] && cmp -s "$tmp" "$RULE_DST"; then
        echo "AETHERFORGE_BEACN_UDEV_RULE=PASS:ALREADY_INSTALLED"
        rm -f "$tmp"
        return 0
    fi

    if [[ "$EUID" == "0" ]]; then
        install -Dm644 "$tmp" "$RULE_DST"
    elif command -v sudo >/dev/null 2>&1; then
        echo "==> One sudo prompt may appear to finalize BEACN USB uaccess"
        sudo install -Dm644 "$tmp" "$RULE_DST"
    else
        rm -f "$tmp"
        echo "AETHERFORGE_BEACN_UDEV_RULE=FAIL:SUDO_UNAVAILABLE"
        exit 1
    fi
    rm -f "$tmp"

    if command -v udevadm >/dev/null 2>&1; then
        if [[ "$EUID" == "0" ]]; then
            udevadm control --reload-rules || true
            udevadm trigger --subsystem-match=usb --attr-match=idVendor=33ae || true
        else
            sudo udevadm control --reload-rules || true
            sudo udevadm trigger --subsystem-match=usb --attr-match=idVendor=33ae || true
        fi
    fi
    echo "AETHERFORGE_BEACN_UDEV_RULE=PASS:INSTALLED"
}

install_runtime_layout() {
    mkdir -p "$HOME/.local/bin" "$APP_DIR" "$DATA_ROOT" "$DOC_DIR" "$CONFIG_DIR/on-device" "$STATE_DIR"

    cat > "$LAUNCHER" <<EOF_LAUNCH
#!/usr/bin/env bash
set -euo pipefail
BIN="$BIN"
STATE_HOME="\${XDG_STATE_HOME:-\${HOME:?HOME is required}/.local/state}"
STATE_DIR="\$STATE_HOME/$APP_ID"
LOG="\$STATE_DIR/launch.log"
mkdir -p "\$STATE_DIR"
printf '\n=== %s $APP_NAME v$VERSION launch ===\n' "\$(date --iso-8601=seconds 2>/dev/null || date)" >> "\$LOG"
exec "\$BIN" "\$@" >> "\$LOG" 2>&1
EOF_LAUNCH
    chmod 755 "$LAUNCHER"

    cat > "$DESKTOP" <<EOF_DESKTOP
[Desktop Entry]
Type=Application
Version=1.0
Name=$APP_NAME
Comment=AetherForge-native BEACN Mic control
Exec=$LAUNCHER
TryExec=$LAUNCHER
Icon=audio-input-microphone
Terminal=false
Categories=AudioVideo;Audio;Settings;
StartupNotify=true
Keywords=BEACN;Microphone;Mic;Audio;DSP;AetherForge;
EOF_DESKTOP
    chmod 644 "$DESKTOP"

    cat > "$DATA_ROOT/uninstall.sh" <<EOF_UNINSTALL
#!/usr/bin/env bash
set -euo pipefail
APP_ID="$APP_ID"
rm -f "\$HOME/.local/bin/\$APP_ID" \
      "\$HOME/.local/bin/\${APP_ID}-launch" \
      "\$HOME/.local/share/applications/\${APP_ID}.desktop"
if [[ "\${1:-}" == "--purge" ]]; then
    rm -rf "\${XDG_CONFIG_HOME:-\$HOME/.config}/\$APP_ID" \
           "\${XDG_STATE_HOME:-\$HOME/.local/state}/\$APP_ID" \
           "\$HOME/.local/share/\$APP_ID" \
           "\$HOME/.local/share/doc/\$APP_ID"
else
    echo "Profiles/config/state preserved. Run again with --purge to remove them."
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "\$HOME/.local/share/applications" >/dev/null 2>&1 || true
fi
echo "AETHERFORGE_BEACN_UNINSTALL=PASS"
EOF_UNINSTALL
    chmod 755 "$DATA_ROOT/uninstall.sh"

    cat > "$DOC_DIR/INSTALLATION.txt" <<EOF_DOC
$APP_NAME v$VERSION

Installed binary: $BIN
Launcher: $LAUNCHER
Desktop entry: $DESKTOP
Configuration: $CONFIG_DIR
State/logs: $STATE_DIR
Host verification: $VERIFY
Runtime probe: $PROBE
Uninstaller: $DATA_ROOT/uninstall.sh

On Device policy: BEACN hardware DSP authoritative; mic-memory import is read-only.
Private DSP policy: local profiles/overlays only; not made the system default source.
EOF_DOC

    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
    fi
    if command -v kbuildsycoca6 >/dev/null 2>&1; then
        kbuildsycoca6 >/dev/null 2>&1 || true
    elif command -v kbuildsycoca5 >/dev/null 2>&1; then
        kbuildsycoca5 >/dev/null 2>&1 || true
    fi

    echo "AETHERFORGE_BEACN_DESKTOP_ENTRY=PASS:$DESKTOP"
    echo "AETHERFORGE_BEACN_LAUNCHER=PASS:$LAUNCHER"
    echo "AETHERFORGE_BEACN_CONFIG_HOME=PASS:$CONFIG_DIR"
    echo "AETHERFORGE_BEACN_STATE_HOME=PASS:$STATE_DIR"
    echo "AETHERFORGE_BEACN_UNINSTALLER=PASS:$DATA_ROOT/uninstall.sh"
}

runtime_checks() {
    command -v wpctl >/dev/null 2>&1 || { echo "AETHERFORGE_BEACN_FINAL_RUNTIME=FAIL:WPCTL_MISSING"; exit 1; }
    command -v pactl >/dev/null 2>&1 || { echo "AETHERFORGE_BEACN_FINAL_RUNTIME=FAIL:PACTL_MISSING"; exit 1; }

    "$BIN" --private-dsp-self-test
    "$BIN" --repair-output-profile
    "$BIN" --probe "$PROBE"

    grep -q '^BEACN_USB_DEVICE_COUNT=[1-9]' "$PROBE" || {
        echo "AETHERFORGE_BEACN_FINAL_RUNTIME=FAIL:NO_BEACN_USB_DEVICE"
        exit 1
    }
    grep -q '^BEACN_PIPEWIRE_HEALTH=BEACN PipeWire audio healthy$' "$PROBE" || {
        echo "AETHERFORGE_BEACN_FINAL_RUNTIME=FAIL:PIPEWIRE_UNHEALTHY"
        exit 1
    }

    echo "AETHERFORGE_BEACN_FINAL_RUNTIME=PASS"
}

launch_application() {
    if pgrep -f "${BIN//\//\\/}" >/dev/null 2>&1; then
        echo "AETHERFORGE_BEACN_APP_START=PASS:ALREADY_RUNNING"
        return 0
    fi

    nohup "$LAUNCHER" >/dev/null 2>&1 &
    local pid=$!
    sleep 2
    if kill -0 "$pid" >/dev/null 2>&1; then
        echo "AETHERFORGE_BEACN_APP_START=PASS:PID=$pid"
    else
        echo "AETHERFORGE_BEACN_APP_START=FAIL:SEE_$STATE_DIR/launch.log"
        exit 1
    fi
}

bootstrap_if_needed
install_udev_rule
install_runtime_layout
runtime_checks
launch_application

echo "AETHERFORGE_BEACN_FULL_INSTALL=PASS"
echo "AETHERFORGE_BEACN_VERSION=$VERSION"
echo "AETHERFORGE_BEACN_BINARY=$BIN"
echo "AETHERFORGE_BEACN_DESKTOP=$DESKTOP"
echo "AETHERFORGE_BEACN_LOG=$STATE_DIR/launch.log"
echo "AETHERFORGE_BEACN_FINAL_REPORT=$LOG"
echo "NOTE=Login autostart is intentionally not enabled until a silent/tray startup mode exists; this avoids opening the full control window at every login."
