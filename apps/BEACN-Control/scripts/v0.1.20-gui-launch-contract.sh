#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:?root}"
MAIN="$ROOT/src/main.rs"
INSTALL="$ROOT/INSTALL-AND-VERIFY.sh"
DESKTOP="$ROOT/packaging/aetherforge-beacn-control.desktop"
fail(){ echo "CONTRACT_FAIL:$1" >&2; exit 1; }
grep -q -- '--gui-smoke-test' "$MAIN" || fail missing_gui_smoke_test_arg
grep -q 'ViewportCommand::Close' "$MAIN" || fail smoke_test_does_not_close_window
grep -q 'AETHERFORGE_BEACN_GUI_SMOKE_TEST=PASS' "$MAIN" || fail smoke_test_pass_marker_missing
grep -q 'AETHERFORGE_BEACN_GUI_SMOKE_TEST' "$INSTALL" || fail installer_does_not_gate_gui_smoke_test
grep -q 'DESKTOP_LAUNCHER=' "$INSTALL" || fail installer_does_not_generate_absolute_launcher
grep -q 'Exec=@AETHERFORGE_BEACN_LAUNCHER@' "$DESKTOP" || fail desktop_is_not_absolute_launcher_template
[[ -f "$ROOT/packaging/aetherforge-beacn-control-launch" ]] || fail launcher_wrapper_template_missing
grep -q '@AETHERFORGE_BEACN_BINARY@' "$ROOT/packaging/aetherforge-beacn-control-launch" || fail launcher_wrapper_binary_placeholder_missing
grep -q 'launch.log' "$ROOT/packaging/aetherforge-beacn-control-launch" || fail launcher_wrapper_log_missing
echo AETHERFORGE_BEACN_V0_1_20_GUI_LAUNCH=PASS
