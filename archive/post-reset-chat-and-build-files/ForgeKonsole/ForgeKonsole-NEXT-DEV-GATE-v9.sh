#!/usr/bin/env bash
set -u

HOME_DIR="${HOME:?HOME is not set}"
DL="$HOME_DIR/Downloads"
WORK="$DL/ForgeKonsole-v1.0.04-work"
LOG="$DL/ForgeKonsole-NEXT-DEV-VERIFY-v9.txt"
TMPDIR_DEV="${XDG_CACHE_HOME:-$HOME_DIR/.cache}/aetherforge/forgekonsole-next-dev-v9"
mkdir -p "$TMPDIR_DEV"
: > "$LOG"

log() { printf '%s\n' "$*" | tee -a "$LOG"; }
fail_now() { log "FORGEKONSOLE_DEV_GATE=FAIL"; log "FORGEKONSOLE_DEV_FAILURE=$1"; exit "${2:-1}"; }

log "FORGEKONSOLE_DEV_GATE=START"
log "FORGEKONSOLE_DEV_GATE_SCRIPT_REVISION=9"
log "FORGEKONSOLE_DEV_GATE_POLICY=NO_INSTALL_NO_ACTIVATION_NO_RELOAD_NO_WINDOW_LAUNCH"
log "FORGEKONSOLE_DEV_SOURCE_BASELINE=1.0.04"
log "FORGEKONSOLE_DEV_TARGET_RELEASE=1.0.05"
log "FORGEKONSOLE_DEV_WORKTREE=$WORK"
log "FORGEKONSOLE_DEV_ROOT_CAUSE=PANE_X_WAS_ROUTED_TO_CONFIRMATION_REQUEST_PATH"
log "FORGEKONSOLE_DEV_V8_PATCH_FAILURE_ROOT_CAUSE=BRITTLE_EXACT_COLOR_REGEX_DID_NOT_MATCH_HOST_SOURCE"
log "FORGEKONSOLE_DEV_CLOSE_POLICY=PANE_X_ONE_CLICK_TERMINATE_AND_CLOSE"
log "FORGEKONSOLE_DEV_CHROME_BUTTON_POLICY=SHARED_DARK_SMOKY_PURPLE_BLUE_DRAGONGLASS"
log "FORGEKONSOLE_DEV_SYSTEM_SCAN_POLICY=HOST_OWNED_READ_ONLY_FIXED_PROBES_BOUNDED_REDACTED"
log "FORGEKONSOLE_DEV_PER_PANE_SCROLL_POLICY=INDEPENDENT_POINTER_TARGETED_PAGE_KEYS_FOLLOW_BOTTOM_HELD_SCROLL_UNSEEN_OUTPUT_DETACHED_PARITY"
log "FORGEKONSOLE_DEV_TRANSPARENCY_POLICY=0_TO_99_PERCENT"
log "FORGEKONSOLE_DEV_SETTINGS_DENSITY_POLICY=RESPONSIVE_TO_ASSIGNED_PANE_WIDTH_NO_HORIZONTAL_OVERFLOW_VERTICAL_SCROLL_FALLBACK"

[[ -f "$WORK/Cargo.toml" ]] || fail_now MISSING_V10004_WORKTREE 2

NATIVE="$WORK/apps/forgekonsole/src/native.rs"
WORKSPACE_RUNTIME="$WORK/apps/forgekonsole/src/workspace_runtime.rs"
THEME="$WORK/crates/forge-ui/src/theme.rs"
REFSHELL="$WORK/apps/forgekonsole/src/reference_shell.rs"
TEST="$WORK/apps/forgekonsole/tests/v10005_one_click_close_chrome_theme_contract.rs"

for required in "$NATIVE" "$WORKSPACE_RUNTIME" "$THEME" "$REFSHELL"; do
  [[ -f "$required" ]] || fail_now "MISSING_SOURCE:$required" 2
done

grep -Fq 'FORGEKONSOLE_V10004_DEDICATED_AETHERAI_TOOLBAR_BUTTON' "$NATIVE" \
  || fail_now "WRONG_BASELINE_MISSING_V10004_AETHERAI_TOOLBAR" 2

log "FORGEKONSOLE_DEV_STAGE=V10005_TEST_INSTALL"
cat > "$TEST" <<'RS'
const NATIVE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/native.rs"));
const WORKSPACE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/workspace_runtime.rs"));
const THEME: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../crates/forge-ui/src/theme.rs"));
const REFERENCE_SHELL: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/reference_shell.rs"));

#[test]
fn pane_x_is_one_click_close_not_a_confirmation_selector() {
    let anchor = NATIVE
        .find("\"forgekonsole-pane-close\"")
        .expect("pane close interaction anchor");
    let end = NATIVE[anchor..]
        .find("let Some(runtime)")
        .map(|offset| anchor + offset)
        .expect("pane close block end");
    let block = &NATIVE[anchor..end];

    assert!(block.contains("FORGEKONSOLE_V10005_ONE_CLICK_PANE_CLOSE"));
    assert!(block.contains("self.terminate_and_close_pane(pane);"));
    assert!(!block.contains("self.request_pane_close(pane);"));
}

#[test]
fn pane_close_runtime_does_not_double_terminate_detached_panes() {
    assert!(!WORKSPACE.contains(
        "let _ = terminal.terminate();\n                        let _ = terminal.terminate();"
    ));
}

#[test]
fn chrome_controls_share_dark_smoky_purple_blue_dragon_glass_fill() {
    assert!(THEME.contains("pub fn chrome_button_rgb(&self) -> [u8; 3]"));
    assert!(THEME.contains("self.glass_alt"));
    assert!(THEME.contains("self.violet"));
    assert!(THEME.contains("self.cyan"));

    let toolbar_start = NATIVE
        .find("fn power_toolbar_button(")
        .expect("power toolbar helper");
    let toolbar_end = NATIVE[toolbar_start..]
        .find("fn power_action_button(")
        .map(|offset| toolbar_start + offset)
        .expect("power toolbar helper end");
    let toolbar = &NATIVE[toolbar_start..toolbar_end];
    assert!(toolbar.contains("let chrome_fill = theme.chrome_button_rgb();"));
    assert!(toolbar.contains("color(chrome_fill"));
    assert!(!toolbar.contains("color(theme.cyan, 32)"));

    let pane_anchor = NATIVE
        .find("\"forgekonsole-pane-close\"")
        .expect("pane close anchor");
    let pane_end = NATIVE[pane_anchor..]
        .find("let Some(runtime)")
        .map(|offset| pane_anchor + offset)
        .expect("pane close end");
    let pane = &NATIVE[pane_anchor..pane_end];
    assert!(pane.contains("self.theme.chrome_button_rgb()"));
    assert!(pane.contains("self.theme.value_text"));

    let settings_start = NATIVE
        .find("fn power_settings_pane_header(")
        .expect("settings pane header");
    let settings_end = NATIVE[settings_start..]
        .find("fn power_settings_row(")
        .map(|offset| settings_start + offset)
        .expect("settings pane header end");
    assert!(NATIVE[settings_start..settings_end].contains("theme.chrome_button_rgb()"));

    assert!(REFERENCE_SHELL.contains("let chrome_fill = theme.chrome_button_rgb();"));
    assert!(!REFERENCE_SHELL.contains("54,\n            64,\n            142,"));
    assert!(!REFERENCE_SHELL.contains("88,\n            55,\n            184,"));
    assert!(!REFERENCE_SHELL.contains("124,\n            47,\n            160,"));
}
RS
log "FORGEKONSOLE_DEV_V10005_REGRESSION_TEST=INSTALLED"

ALREADY=0
if grep -Fq 'FORGEKONSOLE_V10005_ONE_CLICK_PANE_CLOSE' "$NATIVE" \
   && grep -Fq 'pub fn chrome_button_rgb(&self) -> [u8; 3]' "$THEME" \
   && grep -Fq 'self.theme.chrome_button_rgb()' "$NATIVE" \
   && grep -Fq 'FORGEKONSOLE_V10005_SYSTEM_CHROME_BUTTON_THEME' "$NATIVE" \
   && grep -Fq 'let chrome_fill = theme.chrome_button_rgb();' "$REFSHELL"; then
  ALREADY=1
fi

RED_RC=0
if [[ "${1:-}" != "--patch-only" && "$ALREADY" -eq 0 ]]; then
  log "FORGEKONSOLE_DEV_STAGE=V10005_TDD_RED"
  RED_LOG="$TMPDIR_DEV/v10005-red.log"
  set +e
  (cd "$WORK" && cargo test -p forgekonsole --test v10005_one_click_close_chrome_theme_contract -- --nocapture) >"$RED_LOG" 2>&1
  RED_STATUS=$?
  set -e
  cat "$RED_LOG" | tee -a "$LOG"
  if (( RED_STATUS != 0 )); then
    log "FORGEKONSOLE_DEV_V10005_TDD_RED=PASS_EXPECTED_FAILURE"
  else
    log "FORGEKONSOLE_DEV_V10005_TDD_RED=FAIL_TEST_WAS_ALREADY_GREEN"
    RED_RC=1
  fi
else
  log "FORGEKONSOLE_DEV_V10005_TDD_RED=SKIP_ALREADY_APPLIED_OR_PATCH_ONLY"
fi

log "FORGEKONSOLE_DEV_STAGE=PATCH_V10005_ONE_CLICK_CLOSE"
python3 - "$NATIVE" "$WORKSPACE_RUNTIME" <<'PY' 2>&1 | tee -a "$LOG"
from pathlib import Path
import sys
native_path = Path(sys.argv[1])
workspace_path = Path(sys.argv[2])
s = native_path.read_text()
marker = "FORGEKONSOLE_V10005_ONE_CLICK_PANE_CLOSE"
anchor = s.find('"forgekonsole-pane-close"')
if anchor < 0:
    raise SystemExit("pane close interaction anchor missing")
end = s.find("let Some(runtime)", anchor)
if end < 0:
    raise SystemExit("pane close block end missing")
block = s[anchor:end]

if marker not in block:
    old = "self.request_pane_close(pane);"
    if old not in block:
        raise SystemExit("expected pane X confirmation call not found")
    block = block.replace(
        old,
        "// FORGEKONSOLE_V10005_ONE_CLICK_PANE_CLOSE\n                self.terminate_and_close_pane(pane);",
        1,
    )
    s = s[:anchor] + block + s[end:]
    print("FORGEKONSOLE_DEV_ONE_CLICK_CLOSE_PATCH=APPLIED")
else:
    if "self.terminate_and_close_pane(pane);" not in block:
        raise SystemExit("one-click marker present without terminate-and-close call")
    print("FORGEKONSOLE_DEV_ONE_CLICK_CLOSE_PATCH=ALREADY_APPLIED")

native_path.write_text(s)

w = workspace_path.read_text()
duplicate = "let _ = terminal.terminate();\n                        let _ = terminal.terminate();"
if duplicate in w:
    w = w.replace(duplicate, "let _ = terminal.terminate();", 1)
    workspace_path.write_text(w)
    print("FORGEKONSOLE_DEV_DETACHED_DOUBLE_TERMINATE_FIX=APPLIED")
else:
    print("FORGEKONSOLE_DEV_DETACHED_DOUBLE_TERMINATE_FIX=ALREADY_CLEAN")
PY
PATCH_CLOSE_RC=${PIPESTATUS[0]}
(( PATCH_CLOSE_RC == 0 )) || fail_now PATCH_V10005_ONE_CLICK_CLOSE "$PATCH_CLOSE_RC"

log "FORGEKONSOLE_DEV_STAGE=PATCH_V10005_CHROME_THEME"
python3 - "$THEME" "$NATIVE" "$REFSHELL" <<'PY' 2>&1 | tee -a "$LOG"
from pathlib import Path
import sys

theme_path = Path(sys.argv[1])
native_path = Path(sys.argv[2])
ref_path = Path(sys.argv[3])


def replace_call(text: str, start: int, replacement: str, label: str) -> str:
    if start < 0:
        raise SystemExit(f"{label} start missing")
    end = text.find(");", start)
    if end < 0:
        raise SystemExit(f"{label} end missing")
    return text[:start] + replacement + text[end + 2:]


def function_slice(text: str, start_marker: str, end_marker: str, label: str):
    start = text.find(start_marker)
    end = text.find(end_marker, start)
    if start < 0 or end < 0:
        raise SystemExit(f"{label} boundaries missing")
    return start, end, text[start:end]

# Canonical system-derived dark smoky purple-blue chrome fill.
theme = theme_path.read_text()
method_sig = "pub fn chrome_button_rgb(&self) -> [u8; 3]"
if theme.count(method_sig) > 1:
    raise SystemExit("duplicate chrome_button_rgb methods found")
if method_sig not in theme:
    addition = r'''

impl DragonGlassTheme {
    /// Shared DragonGlass chrome-control fill, derived only from canonical
    /// DragonGlass palette fields so ForgeKonsole chrome tracks the system theme.
    pub fn chrome_button_rgb(&self) -> [u8; 3] {
        fn channel(glass: u8, violet: u8, cyan: u8) -> u8 {
            let mixed = (u16::from(glass) * 6 + u16::from(violet) + u16::from(cyan)) / 8;
            u8::try_from(mixed).unwrap_or(u8::MAX)
        }

        [
            channel(self.glass_alt[0], self.violet[0], self.cyan[0]),
            channel(self.glass_alt[1], self.violet[1], self.cyan[1]),
            channel(self.glass_alt[2], self.violet[2], self.cyan[2]),
        ]
    }
}
'''
    theme = theme.rstrip() + addition
    theme_path.write_text(theme)
    print("FORGEKONSOLE_DEV_CHROME_THEME_METHOD=APPLIED")
else:
    print("FORGEKONSOLE_DEV_CHROME_THEME_METHOD=ALREADY_APPLIED")

native = native_path.read_text()

# Pane X: replace paint calls by structure instead of one historical color regex.
anchor = native.find('"forgekonsole-pane-close"')
block_end = native.find("let Some(runtime)", anchor)
if anchor < 0 or block_end < 0:
    raise SystemExit("pane close block boundaries missing")
block = native[anchor:block_end]

circle_start = block.find("painter.circle_filled(")
canonical_circle = '''painter.circle_filled(
                close_rect.center(),
                close_rect.width() * 0.46,
                color(
                    self.theme.chrome_button_rgb(),
                    if close_response.hovered() { 225 } else { 190 },
                ),
            );'''
if "self.theme.chrome_button_rgb()" not in block:
    block = replace_call(block, circle_start, canonical_circle, "pane close circle")

x_pos = block.find('"×"')
text_start = block.rfind("painter.text(", 0, x_pos)
canonical_text = '''painter.text(
                close_rect.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                egui::FontId::proportional(13.0),
                color(self.theme.value_text, 238),
            );'''
if x_pos < 0 or text_start < 0:
    raise SystemExit("pane close glyph paint call missing")
block = replace_call(block, text_start, canonical_text, "pane close glyph")
if "self.theme.chrome_button_rgb()" not in block or "self.theme.value_text" not in block:
    raise SystemExit("pane close did not adopt shared chrome theme")
native = native[:anchor] + block + native[block_end:]
print("FORGEKONSOLE_DEV_PANE_X_CHROME_THEME=APPLIED_OR_PRESENT")

# Multi-Pane / AetherAI / Settings helper: one shared system-derived visual contract.
start, end, _ = function_slice(native, "fn power_toolbar_button(", "fn power_action_button(", "power toolbar helper")
new_fn = '''fn power_toolbar_button(
    ui: &mut egui::Ui,
    theme: &DragonGlassTheme,
    label: impl Into<egui::WidgetText>,
    selected: bool,
    min_width: f32,
    scale: f32,
) -> egui::Response {
    // FORGEKONSOLE_V10005_SYSTEM_CHROME_BUTTON_THEME
    let chrome_fill = theme.chrome_button_rgb();
    let fill = color(chrome_fill, if selected { 218 } else { 178 });
    let stroke = if selected {
        egui::Stroke::new(1.0_f32, color(theme.lavender, 176))
    } else {
        egui::Stroke::new(0.8_f32, color(theme.lavender, 82))
    };
    ui.add(
        egui::Button::new(label)
            .fill(fill)
            .stroke(stroke)
            .corner_radius(8.0 * scale)
            .min_size(egui::vec2(min_width * scale, 30.0 * scale)),
    )
}

'''
native = native[:start] + new_fn + native[end:]

# Settings X: locate close circle from the actual × glyph, not old alpha/color text.
start, end, settings = function_slice(native, "fn power_settings_pane_header(", "fn power_settings_row(", "settings pane header")
x_pos = settings.find('"×"')
text_start = settings.rfind("painter.text(", 0, x_pos)
circle_start = settings.rfind("painter.circle_filled(", 0, text_start)
canonical_settings_circle = '''painter.circle_filled(
        close_rect.center(),
        close_size * 0.44,
        color(
            theme.chrome_button_rgb(),
            if close_response.hovered() { 225 } else { 190 },
        ),
    );'''
if x_pos < 0 or text_start < 0 or circle_start < 0:
    raise SystemExit("settings close paint calls missing")
if "theme.chrome_button_rgb()" not in settings[circle_start:text_start]:
    settings = replace_call(settings, circle_start, canonical_settings_circle, "settings close circle")
if "theme.chrome_button_rgb()" not in settings:
    raise SystemExit("settings close did not adopt shared chrome fill")
native = native[:start] + settings + native[end:]
native_path.write_text(native)
print("FORGEKONSOLE_DEV_SETTINGS_X_CHROME_THEME=APPLIED_OR_PRESENT")

# Titlebar minimize/maximize/close controls use the same system-derived fill.
ref = ref_path.read_text()
marker = "let chrome_fill = theme.chrome_button_rgb();"
if marker not in ref:
    min_pos = ref.find("rects.minimize.center()")
    start = ref.rfind("painter.circle_filled(", 0, min_pos)
    end = ref.find("    paint_window_control_glyph(", min_pos)
    if min_pos < 0 or start < 0 or end < 0:
        raise SystemExit("reference window-control paint block not found")
    new_controls = '''    let chrome_fill = theme.chrome_button_rgb();
    painter.circle_filled(
        rects.minimize.center(),
        rects.minimize.width().min(rects.minimize.height()) * 0.5,
        egui::Color32::from_rgba_unmultiplied(
            chrome_fill[0],
            chrome_fill[1],
            chrome_fill[2],
            if minimize_response.hovered() { 225 } else { 190 },
        ),
    );
    painter.circle_filled(
        rects.maximize.center(),
        rects.maximize.width().min(rects.maximize.height()) * 0.5,
        egui::Color32::from_rgba_unmultiplied(
            chrome_fill[0],
            chrome_fill[1],
            chrome_fill[2],
            if maximize_response.hovered() { 225 } else { 190 },
        ),
    );
    painter.circle_filled(
        rects.close.center(),
        rects.close.width().min(rects.close.height()) * 0.5,
        egui::Color32::from_rgba_unmultiplied(
            chrome_fill[0],
            chrome_fill[1],
            chrome_fill[2],
            if close_response.hovered() { 225 } else { 190 },
        ),
    );
'''
    ref = ref[:start] + new_controls + ref[end:]
    ref_path.write_text(ref)
    print("FORGEKONSOLE_DEV_WINDOW_CHROME_THEME=APPLIED")
else:
    print("FORGEKONSOLE_DEV_WINDOW_CHROME_THEME=ALREADY_APPLIED")

print("FORGEKONSOLE_DEV_CHROME_THEME_PATCH=APPLIED_OR_PRESENT")
PY
PATCH_THEME_RC=${PIPESTATUS[0]}
(( PATCH_THEME_RC == 0 )) || fail_now PATCH_V10005_CHROME_THEME "$PATCH_THEME_RC"

log "FORGEKONSOLE_DEV_STAGE=STATIC_V10005_SCAN"
python3 - "$NATIVE" "$WORKSPACE_RUNTIME" "$THEME" "$REFSHELL" <<'PY' 2>&1 | tee -a "$LOG"
from pathlib import Path
import sys
native = Path(sys.argv[1]).read_text()
workspace = Path(sys.argv[2]).read_text()
theme = Path(sys.argv[3]).read_text()
ref = Path(sys.argv[4]).read_text()

checks = {}
anchor = native.find('"forgekonsole-pane-close"')
end = native.find("let Some(runtime)", anchor)
block = native[anchor:end] if anchor >= 0 and end >= 0 else ""
checks["pane X marker"] = "FORGEKONSOLE_V10005_ONE_CLICK_PANE_CLOSE" in block
checks["pane X direct terminate close"] = "self.terminate_and_close_pane(pane);" in block
checks["pane X no confirmation route"] = "self.request_pane_close(pane);" not in block
checks["pane X system fill"] = "self.theme.chrome_button_rgb()" in block
checks["pane X pale glyph"] = "self.theme.value_text" in block
checks["no detached double terminate"] = "let _ = terminal.terminate();\n                        let _ = terminal.terminate();" not in workspace
checks["theme chrome method"] = theme.count("pub fn chrome_button_rgb(&self) -> [u8; 3]") == 1
checks["toolbar system chrome marker"] = "FORGEKONSOLE_V10005_SYSTEM_CHROME_BUTTON_THEME" in native
settings_start = native.find("fn power_settings_pane_header(")
settings_end = native.find("fn power_settings_row(", settings_start)
settings_block = native[settings_start:settings_end] if settings_start >= 0 and settings_end >= 0 else ""
checks["settings X shared fill"] = "theme.chrome_button_rgb()" in settings_block
checks["reference controls shared fill"] = "let chrome_fill = theme.chrome_button_rgb();" in ref
checks["v10004 AI button preserved"] = "FORGEKONSOLE_V10004_DEDICATED_AETHERAI_TOOLBAR_BUTTON" in native

failed = [name for name, ok in checks.items() if not ok]
if failed:
    raise SystemExit("v1.0.05 static contract failed: " + ", ".join(failed))

# Current canonical palette computes to a dark indigo/purple-blue [37, 40, 83].
glass = [11, 8, 26]
violet = [209, 61, 255]
cyan = [25, 218, 255]
rgb = [int((glass[i] * 6 + violet[i] + cyan[i]) / 8) for i in range(3)]
print("FORGEKONSOLE_DEV_V10005_STATIC_SCAN=PASS")
print("FORGEKONSOLE_DEV_CHROME_BUTTON_RGB=" + ",".join(map(str, rgb)))
PY
STATIC_RC=${PIPESTATUS[0]}

# Recovery for a previously interrupted Gate 9 run: the bridge may already
# contain system_context while native.rs still constructs AetherAiContext
# without it. Repair only that schema skew before compiling unrelated gates.
log "FORGEKONSOLE_DEV_STAGE=RECOVER_PARTIAL_SYSTEM_SCAN_SCHEMA"
python3 - "$WORK/crates/forge-aetherai-bridge/src/lib.rs" "$NATIVE" <<'PY_PARTIAL_SYSTEM_RECOVERY' 2>&1 | tee -a "$LOG"
from pathlib import Path
import re
import sys

bridge_path = Path(sys.argv[1])
native_path = Path(sys.argv[2])
bridge = bridge_path.read_text()
native = native_path.read_text()

def function_block(text: str, signature: str):
    start = text.find(signature)
    if start < 0:
        raise SystemExit(f"missing function anchor: {signature}")
    candidates = [
        value
        for value in (
            text.find("\n    fn ", start + len(signature)),
            text.find("\n    pub fn ", start + len(signature)),
        )
        if value >= 0
    ]
    end = min(candidates) if candidates else len(text)
    return start, end, text[start:end]

if "pub system_context: Option<String>" in bridge:
    start, end, block = function_block(native, "fn aetherai_context(")
    if "AetherAiContext {" not in block:
        raise SystemExit("native aetherai_context constructor missing")
    if "system_context:" not in block:
        project = re.search(
            r"(?m)^(?P<indent>[ \t]*)project_context:[ \t]*[^\n]*,$",
            block,
        )
        if project is None:
            raise SystemExit("native project_context field missing in AetherAiContext initializer")
        indent = project.group("indent")
        block = (
            block[:project.end()]
            + f"\n{indent}system_context: None,"
            + block[project.end():]
        )
        native = native[:start] + block + native[end:]
        native_path.write_text(native)
        print("FORGEKONSOLE_DEV_PARTIAL_SYSTEM_SCAN_SCHEMA_RECOVERY=APPLIED")
    else:
        print("FORGEKONSOLE_DEV_PARTIAL_SYSTEM_SCAN_SCHEMA_RECOVERY=ALREADY_CONSISTENT")
else:
    print("FORGEKONSOLE_DEV_PARTIAL_SYSTEM_SCAN_SCHEMA_RECOVERY=NOT_NEEDED")
PY_PARTIAL_SYSTEM_RECOVERY
PARTIAL_SYSTEM_RECOVERY_RC=${PIPESTATUS[0]}
(( PARTIAL_SYSTEM_RECOVERY_RC == 0 )) || fail_now RECOVER_PARTIAL_SYSTEM_SCAN_SCHEMA "$PARTIAL_SYSTEM_RECOVERY_RC"

GREEN_RC=0
if [[ "${1:-}" != "--patch-only" ]]; then
  log "FORGEKONSOLE_DEV_STAGE=V10005_TDD_GREEN"
  set +e
  (cd "$WORK" && cargo test -p forgekonsole --test v10005_one_click_close_chrome_theme_contract) 2>&1 | tee -a "$LOG"
  GREEN_RC=${PIPESTATUS[0]}
  set -e
  if (( GREEN_RC == 0 )); then
    log "FORGEKONSOLE_DEV_V10005_TDD_GREEN=PASS"
  else
    log "FORGEKONSOLE_DEV_V10005_TDD_GREEN=FAIL:$GREEN_RC"
  fi
fi

SYSTEM_SCAN="$WORK/apps/forgekonsole/src/system_scan.rs"
SYSTEM_TEST="$WORK/apps/forgekonsole/tests/v10005_system_scan_contract.rs"
LIBRS="$WORK/apps/forgekonsole/src/lib.rs"
BRIDGE="$WORK/crates/forge-aetherai-bridge/src/lib.rs"
BRIDGE_TEST="$WORK/crates/forge-aetherai-bridge/tests/v058_terminal_health_contract.rs"
SYSTEM_SCAN_SPEC="$WORK/docs/superpowers/specs/2026-09-10-forgekonsole-system-scan-design.md"
SYSTEM_SCAN_PLAN="$WORK/docs/superpowers/plans/2026-09-10-forgekonsole-system-scan.md"

log "FORGEKONSOLE_DEV_STAGE=SYSTEM_SCAN_TEST_INSTALL"
cat > "$SYSTEM_TEST" <<'RS_SYSTEM_TEST'
use forgekonsole::system_scan::{
    MAX_SYSTEM_SCAN_REPORT_CHARS, SystemScanReport, SystemScanSection, sanitize_probe_output,
};

const LIB: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
const NATIVE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/native.rs"));
const SCAN: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/system_scan.rs"));
const BRIDGE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/forge-aetherai-bridge/src/lib.rs"
));

#[test]
fn host_owned_scan_has_fixed_read_only_authority_boundary() {
    assert!(LIB.contains("pub mod system_scan;"));
    assert!(SCAN.contains("const READ_ONLY_PROBES: &[ProbeSpec]"));
    assert!(SCAN.contains("Command::new(program).args(args).output()"));
    assert!(!SCAN.contains("Command::new(\"sh\")"));
    assert!(!SCAN.contains("Command::new(\"bash\")"));
    assert!(!SCAN.contains("Command::new(\"sudo\")"));
    assert!(!SCAN.contains("Command::new(\"pkexec\")"));
    assert!(!SCAN.contains("systemctl\", &[\"restart"));
    assert!(!SCAN.contains("systemctl\", &[\"stop"));
    assert!(!SCAN.contains("systemctl\", &[\"start"));
    assert!(!SCAN.contains("pacman\", &[\"-S"));
    assert!(!SCAN.contains("pacman\", &[\"-R"));
}

#[test]
fn system_scan_redacts_sensitive_lines_and_bounds_report() {
    let sanitized = sanitize_probe_output(
        "healthy=true\nAuthorization: Bearer TOPSECRET\npassword=hunter2\nfinished=true",
        4096,
    );
    assert!(sanitized.contains("healthy=true"));
    assert!(sanitized.contains("finished=true"));
    assert!(!sanitized.contains("TOPSECRET"));
    assert!(!sanitized.contains("hunter2"));

    let report = SystemScanReport {
        collected_unix_seconds: 7,
        sections: vec![SystemScanSection {
            name: "BOUNDARY".into(),
            body: "x".repeat(MAX_SYSTEM_SCAN_REPORT_CHARS * 2),
        }],
    };
    assert!(report.bounded_text().chars().count() <= MAX_SYSTEM_SCAN_REPORT_CHARS);
}

#[test]
fn system_scan_covers_requested_host_domains() {
    for marker in [
        "OS_RELEASE",
        "CPU",
        "GPU_PCI",
        "MEMORY",
        "THERMALS",
        "PROCESS_HOTSPOTS_CPU",
        "SYSTEMD_FAILED_SYSTEM",
        "JOURNAL_CRITICAL_SYSTEM",
        "PACKAGES",
        "NETWORK_ADDRESSES",
        "DNS_RESOLVER",
        "AUDIO_PIPEWIRE",
        "STORAGE_BLOCK_DEVICES",
        "MOUNTS",
        "AETHERFORGE_USER_UNITS",
        "AETHERFORGE_PROCESSES",
        "AETHERFORGE_PATH_PERMISSIONS",
    ] {
        assert!(SCAN.contains(marker), "missing system scan domain {marker}");
    }
}

#[test]
fn aetherai_receives_system_report_without_command_authority() {
    assert!(BRIDGE.contains("pub system_context: Option<String>"));
    assert!(BRIDGE.contains("DEFAULT_SYSTEM_HEALTH_SCAN"));
    assert!(BRIDGE.contains("system_health_scan_prompt"));
    assert!(BRIDGE.contains("send_system_health_scan"));
    assert!(BRIDGE.contains("ForgeKonsole host-owned system scan"));
    assert!(!BRIDGE.contains("send_terminal_bytes"));
    assert!(!BRIDGE.contains("send_to_pane"));
    assert!(!BRIDGE.contains("Command::new(\"sh\")"));
}

#[test]
fn native_ai_panel_runs_scan_asynchronously_and_exposes_scan_system_action() {
    assert!(NATIVE.contains("system_scan_receiver: Option<Receiver<SystemScanReport>>"));
    assert!(NATIVE.contains("fn start_system_scan(&mut self)"));
    assert!(NATIVE.contains("thread::spawn(move ||"));
    assert!(NATIVE.contains("fn poll_system_scan(&mut self, ctx: &egui::Context)"));
    assert!(NATIVE.contains("self.ai_session.send_system_health_scan(&context)"));
    assert!(NATIVE.contains("\"SCAN SYSTEM\""));
    assert!(NATIVE.contains("self.start_system_scan();"));
    assert!(NATIVE.contains("self.poll_system_scan(ctx);"));
    assert!(NATIVE.contains("system_context: self.system_scan_context.clone()"));
}
RS_SYSTEM_TEST
log "FORGEKONSOLE_DEV_SYSTEM_SCAN_REGRESSION_TEST=INSTALLED"

SYSTEM_ALREADY=0
if [[ -f "$SYSTEM_SCAN" ]] \
   && grep -Fq 'pub mod system_scan;' "$LIBRS" \
   && grep -Fq 'pub system_context: Option<String>' "$BRIDGE" \
   && grep -Fq 'fn start_system_scan(&mut self)' "$NATIVE" \
   && grep -Fq '"SCAN SYSTEM"' "$NATIVE"; then
  SYSTEM_ALREADY=1
fi

SYSTEM_RED_RC=0
if [[ "${1:-}" != "--patch-only" && "$SYSTEM_ALREADY" -eq 0 ]]; then
  log "FORGEKONSOLE_DEV_STAGE=SYSTEM_SCAN_TDD_RED"
  SYSTEM_RED_LOG="$TMPDIR_DEV/v10005-system-scan-red.log"
  set +e
  (cd "$WORK" && cargo test -p forgekonsole --test v10005_system_scan_contract -- --nocapture) >"$SYSTEM_RED_LOG" 2>&1
  SYSTEM_RED_STATUS=$?
  set -e
  cat "$SYSTEM_RED_LOG" | tee -a "$LOG"
  if (( SYSTEM_RED_STATUS != 0 )); then
    log "FORGEKONSOLE_DEV_SYSTEM_SCAN_TDD_RED=PASS_EXPECTED_FAILURE"
  else
    log "FORGEKONSOLE_DEV_SYSTEM_SCAN_TDD_RED=FAIL_TEST_WAS_ALREADY_GREEN"
    SYSTEM_RED_RC=1
  fi
else
  log "FORGEKONSOLE_DEV_SYSTEM_SCAN_TDD_RED=SKIP_ALREADY_APPLIED_OR_PATCH_ONLY"
fi

log "FORGEKONSOLE_DEV_STAGE=PATCH_V10005_HOST_OWNED_SYSTEM_SCAN"
cat > "$SYSTEM_SCAN" <<'RS_SYSTEM_SCAN'
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_SYSTEM_SCAN_REPORT_CHARS: usize = 48_000;
const MAX_SECTION_CHARS: usize = 8_000;
const MAX_LINE_CHARS: usize = 900;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemScanSection {
    pub name: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SystemScanReport {
    pub collected_unix_seconds: u64,
    pub sections: Vec<SystemScanSection>,
}

impl SystemScanReport {
    pub fn bounded_text(&self) -> String {
        let mut out = format!(
            "ForgeKonsole host-owned read-only system scan\ncollected_unix_seconds={}\npolicy=fixed-probes,no-shell,no-sudo,no-mutation,secrets-redacted\n",
            self.collected_unix_seconds
        );
        for section in &self.sections {
            let chunk = format!("\n=== {} ===\n{}\n", section.name, section.body.trim());
            append_bounded(&mut out, &chunk, MAX_SYSTEM_SCAN_REPORT_CHARS);
            if out.chars().count() >= MAX_SYSTEM_SCAN_REPORT_CHARS {
                break;
            }
        }
        out
    }
}

#[derive(Clone, Copy)]
struct ProbeSpec {
    section: &'static str,
    program: &'static str,
    args: &'static [&'static str],
    max_lines: usize,
}

const READ_ONLY_PROBES: &[ProbeSpec] = &[
    ProbeSpec {
        section: "KERNEL",
        program: "uname",
        args: &["-a"],
        max_lines: 8,
    },
    ProbeSpec {
        section: "CPU",
        program: "lscpu",
        args: &[],
        max_lines: 96,
    },
    ProbeSpec {
        section: "GPU_PCI",
        program: "lspci",
        args: &["-nnk"],
        max_lines: 180,
    },
    ProbeSpec {
        section: "THERMALS",
        program: "sensors",
        args: &[],
        max_lines: 120,
    },
    ProbeSpec {
        section: "PROCESS_HOTSPOTS_CPU",
        program: "ps",
        args: &["-eo", "pid=,user=,stat=,pcpu=,pmem=,comm=", "--sort=-pcpu"],
        max_lines: 60,
    },
    ProbeSpec {
        section: "PROCESS_HOTSPOTS_MEMORY",
        program: "ps",
        args: &["-eo", "pid=,user=,stat=,pcpu=,pmem=,comm=", "--sort=-pmem"],
        max_lines: 60,
    },
    ProbeSpec {
        section: "SYSTEMD_FAILED_SYSTEM",
        program: "systemctl",
        args: &["--failed", "--no-pager", "--plain", "--legend=no"],
        max_lines: 100,
    },
    ProbeSpec {
        section: "SYSTEMD_FAILED_USER",
        program: "systemctl",
        args: &["--user", "--failed", "--no-pager", "--plain", "--legend=no"],
        max_lines: 100,
    },
    ProbeSpec {
        section: "JOURNAL_CRITICAL_SYSTEM",
        program: "journalctl",
        args: &["-b", "-p", "0..3", "-n", "120", "--no-pager", "--output=short-monotonic"],
        max_lines: 120,
    },
    ProbeSpec {
        section: "JOURNAL_CRITICAL_USER",
        program: "journalctl",
        args: &["--user", "-b", "-p", "0..3", "-n", "120", "--no-pager", "--output=short-monotonic"],
        max_lines: 120,
    },
    ProbeSpec {
        section: "NETWORK_ADDRESSES",
        program: "ip",
        args: &["-brief", "address"],
        max_lines: 80,
    },
    ProbeSpec {
        section: "NETWORK_ROUTES",
        program: "ip",
        args: &["route"],
        max_lines: 80,
    },
    ProbeSpec {
        section: "NETWORK_LINK_STATS",
        program: "ip",
        args: &["-s", "link"],
        max_lines: 140,
    },
    ProbeSpec {
        section: "NETWORK_SOCKET_SUMMARY",
        program: "ss",
        args: &["-s"],
        max_lines: 40,
    },
    ProbeSpec {
        section: "DNS_RESOLVER",
        program: "resolvectl",
        args: &["status"],
        max_lines: 120,
    },
    ProbeSpec {
        section: "AUDIO_PIPEWIRE",
        program: "wpctl",
        args: &["status"],
        max_lines: 160,
    },
    ProbeSpec {
        section: "AUDIO_SERVER",
        program: "pactl",
        args: &["info"],
        max_lines: 80,
    },
    ProbeSpec {
        section: "STORAGE_BLOCK_DEVICES",
        program: "lsblk",
        args: &["-o", "NAME,TYPE,SIZE,FSTYPE,FSAVAIL,FSUSE%,MOUNTPOINTS"],
        max_lines: 120,
    },
    ProbeSpec {
        section: "FILESYSTEM_USAGE",
        program: "df",
        args: &["-hT"],
        max_lines: 120,
    },
    ProbeSpec {
        section: "MOUNTS",
        program: "findmnt",
        args: &["-rn", "-o", "SOURCE,TARGET,FSTYPE"],
        max_lines: 140,
    },
];

pub fn collect_system_scan() -> SystemScanReport {
    let collected_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut sections = Vec::new();

    sections.push(SystemScanSection {
        name: "OS_RELEASE".into(),
        body: read_text_file(Path::new("/etc/os-release"), 80),
    });
    sections.push(SystemScanSection {
        name: "MEMORY".into(),
        body: read_text_file(Path::new("/proc/meminfo"), 80),
    });
    sections.push(SystemScanSection {
        name: "LOAD_AVERAGE".into(),
        body: read_text_file(Path::new("/proc/loadavg"), 8),
    });
    sections.push(SystemScanSection {
        name: "UPTIME".into(),
        body: read_text_file(Path::new("/proc/uptime"), 8),
    });

    for probe in READ_ONLY_PROBES {
        sections.push(SystemScanSection {
            name: probe.section.into(),
            body: run_probe(probe.program, probe.args, probe.max_lines),
        });
    }

    sections.push(SystemScanSection {
        name: "PACKAGES".into(),
        body: package_summary(),
    });
    sections.push(SystemScanSection {
        name: "AETHERFORGE_USER_UNITS".into(),
        body: filtered_probe(
            "systemctl",
            &["--user", "list-units", "--all", "--no-pager", "--plain", "--legend=no"],
            &["aether", "forge"],
            140,
        ),
    });
    sections.push(SystemScanSection {
        name: "AETHERFORGE_PROCESSES".into(),
        body: filtered_probe(
            "ps",
            &["-eo", "pid=,user=,stat=,pcpu=,pmem=,comm="],
            &["aether", "forge", "opendeck", "opensanctuary"],
            100,
        ),
    });
    sections.push(SystemScanSection {
        name: "AETHERFORGE_PATH_PERMISSIONS".into(),
        body: aetherforge_path_metadata(),
    });

    SystemScanReport {
        collected_unix_seconds,
        sections,
    }
}

pub fn sanitize_probe_output(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let sanitized = if line_is_sensitive(line) {
            "[REDACTED SENSITIVE LINE]".to_string()
        } else {
            line.chars()
                .filter(|ch| !ch.is_control() || *ch == '\t')
                .take(MAX_LINE_CHARS)
                .collect::<String>()
        };
        let chunk = format!("{sanitized}\n");
        append_bounded(&mut out, &chunk, max_chars);
        if out.chars().count() >= max_chars {
            break;
        }
    }
    if out.is_empty() {
        "(no output)".into()
    } else {
        out
    }
}

fn line_is_sensitive(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    [
        "password",
        "passwd",
        "token",
        "secret",
        "authorization",
        "cookie",
        "api_key",
        "apikey",
        "openai_api_key",
        "bearer ",
        "credentials=",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn append_bounded(target: &mut String, text: &str, limit: usize) {
    let used = target.chars().count();
    if used >= limit {
        return;
    }
    let remaining = limit - used;
    target.extend(text.chars().take(remaining));
}

fn run_probe(program: &str, args: &[&str], max_lines: usize) -> String {
    match Command::new(program).args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut combined = String::new();
            if !stdout.trim().is_empty() {
                combined.push_str(&stdout);
            }
            if !stderr.trim().is_empty() {
                if !combined.is_empty() {
                    combined.push('\n');
                }
                combined.push_str("stderr: ");
                combined.push_str(&stderr);
            }
            if !output.status.success() {
                let code = output.status.code().map_or_else(|| "signal".into(), |code| code.to_string());
                combined.push_str(&format!("\nstatus=nonzero({code})"));
            }
            sanitize_probe_output(&take_lines(&combined, max_lines), MAX_SECTION_CHARS)
        }
        Err(error) => format!("unavailable: {program}: {error}"),
    }
}

fn filtered_probe(program: &str, args: &[&str], needles: &[&str], max_lines: usize) -> String {
    match Command::new(program).args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let filtered = stdout
                .lines()
                .filter(|line| {
                    let lower = line.to_ascii_lowercase();
                    needles.iter().any(|needle| lower.contains(needle))
                })
                .take(max_lines)
                .collect::<Vec<_>>()
                .join("\n");
            if filtered.is_empty() {
                "(no matching entries)".into()
            } else {
                sanitize_probe_output(&filtered, MAX_SECTION_CHARS)
            }
        }
        Err(error) => format!("unavailable: {program}: {error}"),
    }
}

fn package_summary() -> String {
    let installed = match Command::new("pacman").arg("-Q").output() {
        Ok(output) if output.status.success() => {
            let count = String::from_utf8_lossy(&output.stdout).lines().count();
            format!("installed_packages={count}")
        }
        Ok(output) => format!("installed_packages=unknown status={:?}", output.status.code()),
        Err(error) => format!("installed_packages=unavailable: {error}"),
    };
    let orphans = run_probe("pacman", &["-Qdtq"], 120);
    let foreign = run_probe("pacman", &["-Qmq"], 160);
    let upgrades = run_probe("pacman", &["-Qu"], 160);
    sanitize_probe_output(
        &format!(
            "{installed}\n\n-- orphan packages --\n{orphans}\n\n-- foreign/AUR packages --\n{foreign}\n\n-- upgrade candidates from current sync DB --\n{upgrades}"
        ),
        MAX_SECTION_CHARS,
    )
}

fn read_text_file(path: &Path, max_lines: usize) -> String {
    match fs::read_to_string(path) {
        Ok(text) => sanitize_probe_output(&take_lines(&text, max_lines), MAX_SECTION_CHARS),
        Err(error) => format!("unavailable: {}: {error}", path.display()),
    }
}

fn take_lines(text: &str, max_lines: usize) -> String {
    text.lines().take(max_lines).collect::<Vec<_>>().join("\n")
}

fn aetherforge_path_metadata() -> String {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return "HOME unavailable; path metadata skipped".into();
    };
    let paths = [
        home.join(".config/forgekonsole"),
        home.join(".config/forgekonsole/secrets/openai-api-key"),
        home.join(".local/share/aetherforge"),
        home.join(".local/lib/forgekonsole"),
        home.join(".local/bin/forgekonsole"),
    ];
    let mut out = String::new();
    for path in paths {
        let line = match fs::symlink_metadata(&path) {
            Ok(metadata) => format!(
                "{} present=true type={} mode={} bytes={}",
                path.display(),
                file_type_label(&metadata),
                permission_mode(&metadata),
                metadata.len()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                format!("{} present=false", path.display())
            }
            Err(error) => format!("{} metadata_error={error}", path.display()),
        };
        out.push_str(&line);
        out.push('\n');
    }
    sanitize_probe_output(&out, MAX_SECTION_CHARS)
}

fn file_type_label(metadata: &fs::Metadata) -> &'static str {
    let file_type = metadata.file_type();
    if file_type.is_dir() {
        "dir"
    } else if file_type.is_symlink() {
        "symlink"
    } else if file_type.is_file() {
        "file"
    } else {
        "other"
    }
}

#[cfg(unix)]
fn permission_mode(metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::PermissionsExt;
    format!("{:04o}", metadata.permissions().mode() & 0o7777)
}

#[cfg(not(unix))]
fn permission_mode(_metadata: &fs::Metadata) -> String {
    "platform-default".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_scan_redacts_sensitive_lines() {
        let text = "normal line\nAuthorization: Bearer abc\npassword=hunter2\nfinal line";
        let sanitized = sanitize_probe_output(text, 4096);
        assert!(sanitized.contains("normal line"));
        assert!(sanitized.contains("final line"));
        assert!(!sanitized.contains("hunter2"));
        assert!(!sanitized.contains("Bearer abc"));
        assert_eq!(sanitized.matches("[REDACTED SENSITIVE LINE]").count(), 2);
    }

    #[test]
    fn system_scan_report_is_globally_bounded() {
        let report = SystemScanReport {
            collected_unix_seconds: 1,
            sections: vec![SystemScanSection {
                name: "TEST".into(),
                body: "x".repeat(MAX_SYSTEM_SCAN_REPORT_CHARS * 2),
            }],
        };
        assert!(report.bounded_text().chars().count() <= MAX_SYSTEM_SCAN_REPORT_CHARS);
    }
}
RS_SYSTEM_SCAN
mkdir -p "$(dirname "$SYSTEM_SCAN_SPEC")" "$(dirname "$SYSTEM_SCAN_PLAN")"
cat > "$SYSTEM_SCAN_SPEC" <<'DOC_SYSTEM_SCAN_SPEC'
# ForgeKonsole AetherAI Host-Owned System Scan Design

## Goal

Add a whole-system, read-only diagnostic scan to ForgeKonsole's embedded AetherAI panel. ForgeKonsole—not the model—collects a bounded, structured report from an explicit allowlist of host probes. AetherAI receives that report as read-only context and diagnoses it. No model path gains PTY-write, arbitrary command, root, package mutation, service mutation, or file-write authority.

## User Experience

The AetherAI Chat view exposes `SCAN SYSTEM` beside `SCAN TERMINAL`. Clicking it starts one background scan. The panel shows progress/status without blocking the terminal UI. When the scan completes, ForgeKonsole stores the latest sanitized report, adds a user-visible `SCAN SYSTEM` transcript entry, and submits a dedicated system-health prompt to AetherAI. Re-clicking while a scan is active is ignored. Failed or unavailable probes are represented in the report instead of aborting the scan.

## Scan Coverage

The host-owned scanner covers these categories with fixed executable + argument vectors and no shell evaluation:

- OS/kernel and host identity: `/etc/os-release`, `uname`.
- CPU/GPU/memory/thermal: `lscpu`, `lspci -nnk`, `/proc/meminfo`, optional `sensors`.
- Processes/resource hotspots: bounded `ps` columns without command arguments or environment variables.
- systemd: failed system and user units, plus AetherForge/ForgeKonsole user-unit summaries.
- Logs: current-boot priority 0..3 journal excerpts, bounded and redacted.
- Packages/update state: installed package count, orphan/foreign/update-candidate summaries from pacman local databases only; no sync/refresh/install.
- Network: interface/address, route, DNS/resolver status; no packet capture or credential access.
- Audio: PipeWire/WirePlumber/Pulse compatibility status via fixed read-only probes.
- Storage/mounts: lsblk/findmnt/df summaries.
- AetherForge/ForgeKonsole: relevant running processes, user services, and known install/config path metadata.
- Permissions/configuration: metadata/modes for known AetherForge/ForgeKonsole paths; secret contents are never read.

## Security and Privacy Boundaries

The scanner never invokes `sh`, `bash`, `sudo`, `pkexec`, package mutation commands, `systemctl start/stop/restart/enable/disable`, or arbitrary user-provided executable/arguments. It never reads environment variables, browser data, SSH keys, credential stores, clipboard contents, or the OpenAI key file contents. Command output is capped per probe and globally. Lines containing credential indicators such as `password`, `token`, `secret`, `authorization`, `cookie`, `api_key`, or `apikey` are replaced with a redaction marker before they enter AetherAI context. Process scans omit argv. Known secret paths are reported only as metadata (present/mode), never content.

There is no privilege escalation. Root-only probes may report unavailable/permission denied. Any future repair/action system remains a separate, explicit user-approved path.

## Architecture

Create `apps/forgekonsole/src/system_scan.rs` as the sole owner of host diagnostic collection. It exports a serializable report plus `collect_system_scan()` and a bounded text renderer. `native.rs` owns scan lifecycle with an mpsc receiver and stores only the latest report text/status. `forge-aetherai-bridge::AetherAiContext` gains `system_context: Option<String>` and the contextualizer adds it under a `ForgeKonsole host-owned system scan` section. The bridge gains a dedicated `send_system_health_scan` prompt path while preserving the no-command-authority contract.

## Failure Handling

Each probe is independent. Missing executables, permission failures, and non-zero statuses become concise probe notes. The background worker returns a report unless the scanner itself cannot initialize. Output is UTF-8-lossy, line-limited, byte-limited, and redacted. UI state is always released on receiver success/disconnect.

## Verification

Tests must prove: fixed allowlist/no shell/no mutation authority; sensitive-line redaction; bounded report size; system context appears in the AetherAI prompt; scan lifecycle is asynchronous; `SCAN SYSTEM` is wired in the embedded panel; existing terminal scan remains intact; and the bridge still contains no PTY-write or arbitrary shell authority. The full DEV gate then runs fmt, Clippy `-D warnings`, full workspace tests, release build, smokes, and verifier without install/activation/reload/window launch.
DOC_SYSTEM_SCAN_SPEC
cat > "$SYSTEM_SCAN_PLAN" <<'DOC_SYSTEM_SCAN_PLAN'
# ForgeKonsole AetherAI System Scan Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give embedded AetherAI a host-owned, read-only whole-system diagnostic scan without granting arbitrary command or mutation authority.

**Architecture:** A new `system_scan` module executes a fixed allowlist of bounded read-only probes on a worker thread. `native.rs` owns lifecycle/status and passes the sanitized report through `AetherAiContext.system_context`; the bridge contextualizer sends the report to AetherAI with a dedicated system-health prompt.

**Tech Stack:** Rust 2024, std::process::Command, std::sync::mpsc, serde/serde_json, egui/eframe, existing ForgeKonsole/AetherAI bridge.

**Spec:** `docs/superpowers/specs/2026-09-10-forgekonsole-system-scan-design.md`

## Global Constraints

- Read-only collection only; no privilege escalation.
- Fixed executable + argument vectors; never `sh -c`, `bash -c`, or user-supplied commands.
- No system/package/service mutation commands.
- Never read secret contents, environment variables, browser data, SSH keys, credential stores, or clipboard data.
- Redact credential-indicator lines before AetherAI context.
- Bound each probe and the total report.
- Preserve existing terminal scan, project discovery, AetherAI isolation, PTY authority boundaries, live-core ABI, and no-second-window behavior.
- DEV gate performs no install, activation, reload request, or window launch.

---

### Task 1: Add red tests for system scan authority and report safety

**Files:**
- Create: `apps/forgekonsole/tests/v10005_system_scan_contract.rs`
- Test: `apps/forgekonsole/tests/v10005_system_scan_contract.rs`

**Interfaces:**
- Consumes: existing `forgekonsole` source tree and bridge source.
- Produces: source contracts for `system_scan`, `AetherAiContext.system_context`, native scan lifecycle, and embedded UI action.

- [ ] **Step 1: Write the failing source/runtime contract test** covering module export, fixed command allowlist, no shell/sudo/pkexec/mutation strings, sensitive redaction, report cap, bridge system context, native receiver/status, `SCAN SYSTEM`, and `send_system_health_scan`.
- [ ] **Step 2: Run** `cargo test -p forgekonsole --test v10005_system_scan_contract -- --nocapture` **and verify RED because the subsystem does not exist yet.**

### Task 2: Implement host-owned scanner

**Files:**
- Create: `apps/forgekonsole/src/system_scan.rs`
- Modify: `apps/forgekonsole/src/lib.rs`

**Interfaces:**
- Produces: `pub const MAX_SYSTEM_SCAN_REPORT_CHARS: usize`, `pub struct SystemScanReport`, `pub fn collect_system_scan() -> SystemScanReport`, and `SystemScanReport::bounded_text()`.

- [ ] **Step 1: Implement fixed read-only probe helpers using `Command::new(executable).args(args)` with no shell.**
- [ ] **Step 2: Add bounded/redacted output helpers and known-path metadata collection.**
- [ ] **Step 3: Add category probes for OS/kernel, CPU/GPU/memory/thermal, processes, systemd, journal, packages, network/DNS, audio, storage/mounts, AetherForge services/processes, and configuration permission metadata.**
- [ ] **Step 4: Export `pub mod system_scan;` from `lib.rs`.**

### Task 3: Extend read-only AetherAI context

**Files:**
- Modify: `crates/forge-aetherai-bridge/src/lib.rs`
- Modify: `crates/forge-aetherai-bridge/tests/v058_terminal_health_contract.rs`

**Interfaces:**
- Consumes: `AetherAiContext`.
- Produces: `system_context: Option<String>`, `DEFAULT_SYSTEM_HEALTH_SCAN`, `system_health_scan_prompt`, and `EmbeddedAetherAiSession::send_system_health_scan`.

- [ ] **Step 1: Add system context with an explicit bounded limit and contextualizer label.**
- [ ] **Step 2: Add the dedicated diagnostic prompt and send method.**
- [ ] **Step 3: Extend bridge tests to prove system context is present while command/PTY authority remains absent.**

### Task 4: Wire asynchronous scan lifecycle and UI

**Files:**
- Modify: `apps/forgekonsole/src/native.rs`

**Interfaces:**
- Consumes: `collect_system_scan`, `SystemScanReport`, `send_system_health_scan`.
- Produces: `system_scan_receiver`, `system_scan_status`, `system_scan_context`, `start_system_scan()`, `poll_system_scan()`, and `SCAN SYSTEM` UI action.

- [ ] **Step 1: Add app state fields and initialize them without affecting live-core ABI state serialization.**
- [ ] **Step 2: Add `start_system_scan()` worker-thread launch with single-flight behavior.**
- [ ] **Step 3: Add `poll_system_scan()` to store bounded sanitized context and submit it to AetherAI.**
- [ ] **Step 4: Add `system_context` to `aetherai_context()`.**
- [ ] **Step 5: Add `SCAN SYSTEM` beside `SCAN TERMINAL` and display current scan status.**
- [ ] **Step 6: Poll the system scan receiver from `eframe::App::update`.**

### Task 5: Verify focused and full gates

**Files:**
- Test: `apps/forgekonsole/tests/v10005_system_scan_contract.rs`
- Test: `crates/forge-aetherai-bridge/tests/v058_terminal_health_contract.rs`

- [ ] **Step 1: Run focused system scan test and verify GREEN.**
- [ ] **Step 2: Run bridge health tests and verify GREEN.**
- [ ] **Step 3: Run `cargo fmt --all` then `cargo fmt --all --check`.**
- [ ] **Step 4: Run `cargo clippy --workspace --all-targets --keep-going -- -D warnings`.**
- [ ] **Step 5: Run `cargo test --workspace --no-fail-fast`.**
- [ ] **Step 6: Remove stale release outputs and run `cargo build --workspace --release`.**
- [ ] **Step 7: Run existing diagnostics/smokes/verifier and require zero failures before v1.0.05 packaging readiness.**
DOC_SYSTEM_SCAN_PLAN

python3 - "$LIBRS" "$BRIDGE" "$BRIDGE_TEST" "$NATIVE" <<'PY_SYSTEM_SCAN' 2>&1 | tee -a "$LOG"
from pathlib import Path
import re
import sys

lib_path, bridge_path, bridge_test_path, native_path = map(Path, sys.argv[1:])

lib = lib_path.read_text()
if "pub mod system_scan;" not in lib:
    anchor = "pub mod terminal_runtime;\n"
    if anchor not in lib:
        raise SystemExit("system scan lib module anchor missing")
    lib = lib.replace(anchor, "pub mod system_scan;\n" + anchor, 1)
lib_path.write_text(lib)

bridge = bridge_path.read_text()
if "DEFAULT_SYSTEM_HEALTH_SCAN" not in bridge:
    anchor = "pub const DEFAULT_TERMINAL_HEALTH_SCAN: &str = "
    pos = bridge.find(anchor)
    if pos < 0:
        raise SystemExit("terminal health prompt anchor missing")
    end = bridge.find("\n", pos) + 1
    addition = 'pub const DEFAULT_SYSTEM_HEALTH_SCAN: &str = "Analyze this ForgeKonsole host-owned read-only system scan for confirmed failures, degraded services, resource bottlenecks, hardware or thermal concerns, storage/filesystem problems, network/DNS issues, PipeWire/audio issues, package/update risks, AetherForge service faults, permission/configuration problems, and likely root causes. Distinguish confirmed evidence from inference. Prioritize findings by severity and impact. Never claim a repair was executed. Recommend changes separately and require explicit user approval before any system mutation.";\n'
    bridge = bridge[:end] + addition + bridge[end:]
if "pub const MAX_SYSTEM_CONTEXT_CHARS" not in bridge:
    anchor = "pub const MAX_PROJECT_CONTEXT_CHARS: usize = 20_000;\n"
    if anchor not in bridge:
        raise SystemExit("project context cap anchor missing")
    bridge = bridge.replace(anchor, anchor + "pub const MAX_SYSTEM_CONTEXT_CHARS: usize = 48_000;\n", 1)
if "pub system_context: Option<String>" not in bridge:
    anchor = "    pub project_context: Option<String>,\n"
    if anchor not in bridge:
        raise SystemExit("AetherAiContext project field anchor missing")
    bridge = bridge.replace(anchor, anchor + "    pub system_context: Option<String>,\n", 1)
if "pub fn send_system_health_scan" not in bridge:
    anchor = """    pub fn send_health_scan(&mut self, context: &AetherAiContext) -> io::Result<()> {
        let prompt = health_scan_prompt(context);
        self.send_prepared_prompt(&prompt, context)
    }
"""
    if anchor not in bridge:
        raise SystemExit("send_health_scan anchor missing")
    bridge = bridge.replace(anchor, anchor + """
    pub fn send_system_health_scan(&mut self, context: &AetherAiContext) -> io::Result<()> {
        let prompt = system_health_scan_prompt(context);
        self.send_prepared_prompt(&prompt, context)
    }
""", 1)
if "pub fn system_health_scan_prompt" not in bridge:
    anchor = """pub fn health_scan_prompt(context: &AetherAiContext) -> String {
    contextualized_prompt(DEFAULT_TERMINAL_HEALTH_SCAN, context)
}
"""
    if anchor not in bridge:
        raise SystemExit("health_scan_prompt anchor missing")
    bridge = bridge.replace(anchor, anchor + """
pub fn system_health_scan_prompt(context: &AetherAiContext) -> String {
    contextualized_prompt(DEFAULT_SYSTEM_HEALTH_SCAN, context)
}
""", 1)
if '"ForgeKonsole host-owned system scan"' not in bridge:
    anchor = """    push_bounded_context(
        &mut lines,
        "Aether Work project/workload context",
        context.project_context.as_deref(),
        MAX_PROJECT_CONTEXT_CHARS,
    );
"""
    if anchor not in bridge:
        raise SystemExit("project contextualizer anchor missing")
    bridge = bridge.replace(anchor, anchor + """    push_bounded_context(
        &mut lines,
        "ForgeKonsole host-owned system scan",
        context.system_context.as_deref(),
        MAX_SYSTEM_CONTEXT_CHARS,
    );
""", 1)
bridge_path.write_text(bridge)

bridge_test = bridge_test_path.read_text()
if "v10005_system_health_prompt_contains_host_owned_report" not in bridge_test:
    bridge_test += """
#[test]
fn v10005_system_health_prompt_contains_host_owned_report() {
    let context = AetherAiContext {
        system_context: Some("SYSTEMD_FAILED_SYSTEM\\nexample.service failed".into()),
        ..Default::default()
    };
    let prompt = forge_aetherai_bridge::system_health_scan_prompt(&context);
    assert!(prompt.contains(forge_aetherai_bridge::DEFAULT_SYSTEM_HEALTH_SCAN));
    assert!(prompt.contains("ForgeKonsole host-owned system scan"));
    assert!(prompt.contains("example.service failed"));
}
"""
bridge_test_path.write_text(bridge_test)

native = native_path.read_text()
if "use crate::system_scan::{SystemScanReport, collect_system_scan};" not in native:
    anchor = "use crate::terminal_runtime::TerminalRuntime;\n"
    if anchor not in native:
        raise SystemExit("native terminal runtime import anchor missing")
    native = native.replace(
        anchor,
        "use crate::system_scan::{SystemScanReport, collect_system_scan};\n" + anchor,
        1,
    )

# Import DEFAULT_SYSTEM_HEALTH_SCAN structurally. Older v1.0.04 sources have
# changed grouped-import formatting across rustfmt revisions, so do not rely
# on one exact line.
if "DEFAULT_SYSTEM_HEALTH_SCAN" not in native:
    grouped = re.search(r"use\s+forge_aetherai_bridge::\{(?P<body>.*?)\};", native, re.S)
    if grouped is not None:
        body = grouped.group("body")
        indent_match = re.search(r"\n(?P<indent>\s*)\S", body)
        indent = indent_match.group("indent") if indent_match else "    "
        replacement = body.rstrip() + f"\n{indent}DEFAULT_SYSTEM_HEALTH_SCAN,\n"
        native = native[:grouped.start("body")] + replacement + native[grouped.end("body"):]
    else:
        first_use = re.search(r"(?m)^use\s+", native)
        if first_use is None:
            raise SystemExit("native use/import section missing")
        native = (
            native[:first_use.start()]
            + "use forge_aetherai_bridge::DEFAULT_SYSTEM_HEALTH_SCAN;\n"
            + native[first_use.start():]
        )
    print("FORGEKONSOLE_DEV_SYSTEM_SCAN_AETHERAI_IMPORT=APPLIED")
else:
    print("FORGEKONSOLE_DEV_SYSTEM_SCAN_AETHERAI_IMPORT=ALREADY_PRESENT")

if "system_scan_receiver: Option<Receiver<SystemScanReport>>" not in native:
    field = re.search(
        r"(?m)^(?P<indent>[ \t]*)project_scan_status:[ \t]*String,[ \t]*$",
        native,
    )
    if field is None:
        raise SystemExit("native project scan field anchor missing")
    indent = field.group("indent")
    addition = (
        f"\n{indent}system_scan_receiver: Option<Receiver<SystemScanReport>>,"
        f"\n{indent}system_scan_status: String,"
        f"\n{indent}system_scan_context: Option<String>,"
    )
    native = native[:field.end()] + addition + native[field.end():]

if "system_scan_receiver: None," not in native:
    init = re.search(
        r"(?m)^(?P<indent>[ \t]*)project_scan_status:(?![ \t]*String,[ \t]*$)[^\n]+,$",
        native,
    )
    if init is None:
        raise SystemExit("native project scan init anchor missing")
    indent = init.group("indent")
    addition = (
        f"\n{indent}system_scan_receiver: None,"
        f"\n{indent}system_scan_status: \"System scan ready · host-owned read-only probes\".into(),"
        f"\n{indent}system_scan_context: None,"
    )
    native = native[:init.end()] + addition + native[init.end():]

if "system_context: self.system_scan_context.clone()" not in native:
    start = native.find("fn aetherai_context(")
    if start < 0:
        raise SystemExit("native aetherai_context function missing")
    candidates = [
        value
        for value in (
            native.find("\n    fn ", start + 1),
            native.find("\n    pub fn ", start + 1),
        )
        if value >= 0
    ]
    end = min(candidates) if candidates else len(native)
    block = native[start:end]
    if "AetherAiContext {" not in block:
        raise SystemExit("AetherAI context constructor missing")
    if "system_context: None," in block:
        block = block.replace(
            "system_context: None,",
            "system_context: self.system_scan_context.clone(),",
            1,
        )
    elif "system_context:" not in block:
        project = re.search(
            r"(?m)^(?P<indent>[ \t]*)project_context:[ \t]*[^\n]*,$",
            block,
        )
        if project is None:
            raise SystemExit("AetherAI project_context field missing")
        indent = project.group("indent")
        block = (
            block[:project.end()]
            + f"\n{indent}system_context: self.system_scan_context.clone(),"
            + block[project.end():]
        )
    native = native[:start] + block + native[end:]
if "fn start_system_scan(&mut self)" not in native:
    anchor = "    fn toggle_aetherai_panel(&mut self) {\n"
    idx = native.find(anchor)
    if idx < 0:
        raise SystemExit("toggle_aetherai_panel anchor missing")
    methods = """    fn start_system_scan(&mut self) {
        if self.system_scan_receiver.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        self.system_scan_receiver = Some(receiver);
        self.system_scan_status = "System scan running · fixed read-only host probes…".into();
        thread::spawn(move || {
            let report = collect_system_scan();
            let _ = sender.send(report);
        });
    }

    fn poll_system_scan(&mut self, ctx: &egui::Context) {
        let result = self.system_scan_receiver.as_ref().and_then(|receiver| {
            match receiver.try_recv() {
                Ok(report) => Some(Ok(report)),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("system scanner disconnected".to_string()))
                }
            }
        });
        let Some(result) = result else {
            return;
        };
        self.system_scan_receiver = None;
        match result {
            Ok(report) => {
                let scan = report.bounded_text();
                let scan_chars = scan.chars().count();
                self.system_scan_context = Some(scan);
                self.system_scan_status = format!(
                    "System scan collected · {scan_chars} chars · submitting to AetherAI"
                );
                self.ai_messages.push(AiPanelMessage {
                    role: AiPanelRole::User,
                    text: "SCAN SYSTEM · host-owned read-only diagnostics".into(),
                });
                let context = self.aetherai_context();
                let mode = self.ai_session.network_mode().label();
                self.ai_status = match self.ai_session.send_system_health_scan(&context) {
                    Ok(()) => format!("{mode} · analyzing host system scan"),
                    Err(error) => format!("AetherAI system scan unavailable · {error}"),
                };
            }
            Err(error) => {
                self.system_scan_status = format!("System scan failed · {error}");
            }
        }
        ctx.request_repaint();
    }

"""
    native = native[:idx] + methods + native[idx:]
if '"SCAN SYSTEM"' not in native:
    marker = """                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("SCAN TERMINAL").strong().size(8.0))
                                    .fill(color(self.theme.cyan, 70))
                                    .corner_radius(8.0),
                            )
                            .on_hover_text(DEFAULT_TERMINAL_HEALTH_SCAN)
                            .clicked()
                        {
                            self.scan_terminal();
                        }
"""
    if marker not in native:
        raise SystemExit("SCAN TERMINAL UI anchor missing")
    addition = marker + """                        let scanning_system = self.system_scan_receiver.is_some();
                        if ui
                            .add_enabled(
                                !scanning_system,
                                egui::Button::new(
                                    egui::RichText::new(if scanning_system {
                                        "SCANNING SYSTEM…"
                                    } else {
                                        "SCAN SYSTEM"
                                    })
                                    .strong()
                                    .size(8.0),
                                )
                                .fill(color(self.theme.violet, 88))
                                .corner_radius(8.0),
                            )
                            .on_hover_text(DEFAULT_SYSTEM_HEALTH_SCAN)
                            .clicked()
                        {
                            self.start_system_scan();
                        }
"""
    native = native.replace(marker, addition, 1)
if "egui::RichText::new(&self.system_scan_status)" not in native:
    anchor = """                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(format!("selection · {context_selection}"))
"""
    idx = native.find(anchor)
    if idx < 0:
        raise SystemExit("AI context status insertion anchor missing")
    insert = """                ui.label(
                    egui::RichText::new(&self.system_scan_status)
                        .size(8.0)
                        .color(color(self.theme.label_text, 205)),
                );
"""
    native = native[:idx] + insert + native[idx:]
if "self.poll_system_scan(ctx);" not in native:
    anchor = "        self.poll_project_scan(ctx);\n"
    if anchor not in native:
        raise SystemExit("native update project scan poll anchor missing")
    native = native.replace(anchor, anchor + "        self.poll_system_scan(ctx);\n", 1)
native_path.write_text(native)

print("FORGEKONSOLE_DEV_SYSTEM_SCAN_SOURCE_PATCH=APPLIED_OR_PRESENT")
PY_SYSTEM_SCAN
PATCH_SYSTEM_SCAN_RC=${PIPESTATUS[0]}
(( PATCH_SYSTEM_SCAN_RC == 0 )) || fail_now PATCH_V10005_HOST_OWNED_SYSTEM_SCAN "$PATCH_SYSTEM_SCAN_RC"

log "FORGEKONSOLE_DEV_STAGE=STATIC_V10005_SYSTEM_SCAN"
python3 - "$SYSTEM_SCAN" "$LIBRS" "$BRIDGE" "$NATIVE" "$SYSTEM_SCAN_SPEC" "$SYSTEM_SCAN_PLAN" <<'PY_SYSTEM_STATIC' 2>&1 | tee -a "$LOG"
from pathlib import Path
import sys

scan, lib, bridge, native, spec, plan = [Path(value) for value in sys.argv[1:]]
scan_text = scan.read_text()
lib_text = lib.read_text()
bridge_text = bridge.read_text()
native_text = native.read_text()
checks = {
    "module export": "pub mod system_scan;" in lib_text,
    "fixed probe table": "const READ_ONLY_PROBES: &[ProbeSpec]" in scan_text,
    "no shell sh": 'Command::new("sh")' not in scan_text,
    "no shell bash": 'Command::new("bash")' not in scan_text,
    "no sudo": 'Command::new("sudo")' not in scan_text,
    "no pkexec": 'Command::new("pkexec")' not in scan_text,
    "report cap": "MAX_SYSTEM_SCAN_REPORT_CHARS: usize = 48_000" in scan_text,
    "sensitive redaction": "[REDACTED SENSITIVE LINE]" in scan_text,
    "bridge system context": "pub system_context: Option<String>" in bridge_text,
    "bridge system prompt": "pub fn system_health_scan_prompt" in bridge_text,
    "bridge send method": "pub fn send_system_health_scan" in bridge_text,
    "native async start": "fn start_system_scan(&mut self)" in native_text and "thread::spawn(move ||" in native_text,
    "native async poll": "fn poll_system_scan(&mut self, ctx: &egui::Context)" in native_text,
    "scan system UI": '"SCAN SYSTEM"' in native_text,
    "system scan context wiring": "system_context: self.system_scan_context.clone()" in native_text,
    "system scan app poll": "self.poll_system_scan(ctx);" in native_text,
    "design spec": spec.is_file(),
    "implementation plan": plan.is_file(),
}
for marker in [
    "OS_RELEASE", "CPU", "GPU_PCI", "MEMORY", "THERMALS",
    "PROCESS_HOTSPOTS_CPU", "SYSTEMD_FAILED_SYSTEM", "JOURNAL_CRITICAL_SYSTEM",
    "PACKAGES", "NETWORK_ADDRESSES", "NETWORK_LINK_STATS", "DNS_RESOLVER",
    "AUDIO_PIPEWIRE", "STORAGE_BLOCK_DEVICES", "MOUNTS",
    "AETHERFORGE_USER_UNITS", "AETHERFORGE_PROCESSES", "AETHERFORGE_PATH_PERMISSIONS",
]:
    checks[f"domain {marker}"] = marker in scan_text
failed = [name for name, ok in checks.items() if not ok]
if failed:
    raise SystemExit("v1.0.05 system scan static contract failed: " + ", ".join(failed))
print("FORGEKONSOLE_DEV_V10005_SYSTEM_SCAN_STATIC=PASS")
print("FORGEKONSOLE_DEV_SYSTEM_SCAN_POLICY=HOST_OWNED_READ_ONLY_FIXED_PROBES_BOUNDED_REDACTED")
PY_SYSTEM_STATIC
SYSTEM_STATIC_RC=${PIPESTATUS[0]}

SYSTEM_GREEN_RC=0
SYSTEM_BRIDGE_GREEN_RC=0
if [[ "${1:-}" != "--patch-only" ]]; then
  log "FORGEKONSOLE_DEV_STAGE=SYSTEM_SCAN_TDD_GREEN"
  set +e
  (cd "$WORK" && cargo test -p forgekonsole --test v10005_system_scan_contract -- --nocapture) 2>&1 | tee -a "$LOG"
  SYSTEM_GREEN_RC=${PIPESTATUS[0]}
  set -e
  if (( SYSTEM_GREEN_RC == 0 )); then
    log "FORGEKONSOLE_DEV_SYSTEM_SCAN_TDD_GREEN=PASS"
  else
    log "FORGEKONSOLE_DEV_SYSTEM_SCAN_TDD_GREEN=FAIL:$SYSTEM_GREEN_RC"
  fi

  log "FORGEKONSOLE_DEV_STAGE=SYSTEM_SCAN_BRIDGE_GREEN"
  set +e
  (cd "$WORK" && cargo test -p forge-aetherai-bridge --test v058_terminal_health_contract -- --nocapture) 2>&1 | tee -a "$LOG"
  SYSTEM_BRIDGE_GREEN_RC=${PIPESTATUS[0]}
  set -e
  if (( SYSTEM_BRIDGE_GREEN_RC == 0 )); then
    log "FORGEKONSOLE_DEV_SYSTEM_SCAN_BRIDGE_GREEN=PASS"
  else
    log "FORGEKONSOLE_DEV_SYSTEM_SCAN_BRIDGE_GREEN=FAIL:$SYSTEM_BRIDGE_GREEN_RC"
  fi
fi


SCROLL_TEST="$WORK/apps/forgekonsole/tests/v10005_per_pane_true_scrolling_contract.rs"
log "FORGEKONSOLE_DEV_STAGE=PER_PANE_TRUE_SCROLL_TEST_INSTALL"
cat > "$SCROLL_TEST" <<'RS_SCROLL_RED'
const WORKSPACE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/workspace_runtime.rs"));
const NATIVE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/native.rs"));
const DETACHED: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/detached_view.rs"));

#[test]
fn each_terminal_pane_owns_an_independent_viewport_state() {
    assert!(WORKSPACE.contains("FORGEKONSOLE_V10005_PER_PANE_TRUE_SCROLL_STATE"));
    assert!(WORKSPACE.contains("pane_viewports: BTreeMap<PaneId, PaneViewportState>"));
    assert!(WORKSPACE.contains("pub fn scroll_pane_lines("));
    assert!(WORKSPACE.contains("pub fn scroll_pane_page("));
    assert!(WORKSPACE.contains("pub fn clamp_pane_viewport("));
}

#[test]
fn root_terminal_scroll_is_pointer_targeted_and_not_global() {
    assert!(NATIVE.contains("FORGEKONSOLE_V10005_PER_PANE_TRUE_SCROLL_ROOT"));
    assert!(NATIVE.contains("FORGEKONSOLE_V10005_POINTER_TARGETED_PANE_SCROLL"));
    assert!(NATIVE.contains("FORGEKONSOLE_V10005_PER_PANE_PAGE_SCROLL"));
    assert!(NATIVE.contains("paint_pane_scrollbar"));
    assert!(NATIVE.contains("active_pane_scroll_offset"));
}

#[test]
fn held_scroll_survives_output_and_detached_panes_have_parity() {
    assert!(WORKSPACE.contains("note_output(read, new_history_lines)"));
    assert!(NATIVE.contains("FORGEKONSOLE_V10005_HELD_SCROLL_SURVIVES_OUTPUT"));
    assert!(DETACHED.contains("FORGEKONSOLE_V10005_DETACHED_PER_PANE_TRUE_SCROLL"));
    assert!(DETACHED.contains("tab.scroll_pane_lines(*pane"));
    assert!(DETACHED.contains("tab.scroll_pane_page(pane"));
}
RS_SCROLL_RED
log "FORGEKONSOLE_DEV_PER_PANE_TRUE_SCROLL_REGRESSION_TEST=INSTALLED"

SCROLL_ALREADY=0
if grep -Fq 'FORGEKONSOLE_V10005_PER_PANE_TRUE_SCROLL_STATE' "$WORKSPACE_RUNTIME" \
   && grep -Fq 'FORGEKONSOLE_V10005_PER_PANE_TRUE_SCROLL_ROOT' "$NATIVE" \
   && grep -Fq 'FORGEKONSOLE_V10005_DETACHED_PER_PANE_TRUE_SCROLL' "$WORK/apps/forgekonsole/src/detached_view.rs"; then
  SCROLL_ALREADY=1
fi

SCROLL_RED_RC=0
if [[ "${1:-}" != "--patch-only" && "$SCROLL_ALREADY" -eq 0 ]]; then
  log "FORGEKONSOLE_DEV_STAGE=PER_PANE_TRUE_SCROLL_TDD_RED"
  set +e
  (cd "$WORK" && cargo test -p forgekonsole --test v10005_per_pane_true_scrolling_contract -- --nocapture) 2>&1 | tee -a "$LOG"
  SCROLL_RED_CMD_RC=${PIPESTATUS[0]}
  set -e
  if (( SCROLL_RED_CMD_RC != 0 )); then
    log "FORGEKONSOLE_DEV_PER_PANE_TRUE_SCROLL_TDD_RED=PASS_EXPECTED_FAILURE"
  else
    log "FORGEKONSOLE_DEV_PER_PANE_TRUE_SCROLL_TDD_RED=FAIL_TEST_WAS_ALREADY_GREEN"
    SCROLL_RED_RC=1
  fi
else
  log "FORGEKONSOLE_DEV_PER_PANE_TRUE_SCROLL_TDD_RED=SKIP_ALREADY_APPLIED_OR_PATCH_ONLY"
fi

log "FORGEKONSOLE_DEV_STAGE=PATCH_V10005_PER_PANE_TRUE_SCROLL"
python3 - "$WORK" <<'PY_SCROLL_PATCH' 2>&1 | tee -a "$LOG"
from pathlib import Path
import re, sys
root=Path(sys.argv[1])
workspace=root/'apps/forgekonsole/src/workspace_runtime.rs'
native=root/'apps/forgekonsole/src/native.rs'
detached=root/'apps/forgekonsole/src/detached_view.rs'

w=workspace.read_text()
if 'FORGEKONSOLE_V10005_PER_PANE_TRUE_SCROLL_STATE' not in w:
    anchor='''#[derive(Debug)]\nstruct ParkedPaneRuntime {\n    profile: String,\n    terminal: TerminalRuntime,\n}\n'''
    insert=anchor+'''\n// FORGEKONSOLE_V10005_PER_PANE_TRUE_SCROLL_STATE\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]\npub struct PaneViewportState {\n    pub scroll_offset: usize,\n    pub unseen_output: bool,\n}\n\nimpl PaneViewportState {\n    pub fn max_scroll(history_lines: usize, visible_rows: usize) -> usize {\n        history_lines.saturating_sub(visible_rows.max(1))\n    }\n\n    pub fn clamp(&mut self, history_lines: usize, visible_rows: usize) {\n        self.scroll_offset = self\n            .scroll_offset\n            .min(Self::max_scroll(history_lines, visible_rows));\n        if self.scroll_offset == 0 {\n            self.unseen_output = false;\n        }\n    }\n\n    pub fn scroll_lines(&mut self, delta: isize, history_lines: usize, visible_rows: usize) {\n        if delta >= 0 {\n            self.scroll_offset = self.scroll_offset.saturating_add(delta as usize);\n        } else {\n            self.scroll_offset = self.scroll_offset.saturating_sub(delta.unsigned_abs());\n        }\n        self.clamp(history_lines, visible_rows);\n    }\n\n    pub fn scroll_page(&mut self, direction: isize, history_lines: usize, visible_rows: usize) {\n        let page = visible_rows.saturating_sub(1).max(1);\n        let delta = if direction >= 0 { page as isize } else { -(page as isize) };\n        self.scroll_lines(delta, history_lines, visible_rows);\n    }\n\n    pub fn note_output(&mut self, bytes_read: usize, new_history_lines: usize) {\n        if bytes_read == 0 {\n            return;\n        }\n        if self.scroll_offset > 0 {\n            self.scroll_offset = self.scroll_offset.saturating_add(new_history_lines);\n            self.unseen_output = true;\n        } else {\n            self.unseen_output = false;\n        }\n    }\n}\n'''
    if anchor not in w: raise SystemExit('workspace parked pane anchor missing')
    w=w.replace(anchor,insert,1)
    w=w.replace('''    process_finished_reported: bool,\n    panes: BTreeMap<PaneId, TerminalRuntime>,\n''','''    process_finished_reported: bool,\n    pane_viewports: BTreeMap<PaneId, PaneViewportState>,\n    panes: BTreeMap<PaneId, TerminalRuntime>,\n''',1)
    method_anchor='''impl TabRuntime {\n    pub fn pane_ids(&self) -> Vec<PaneId> {\n'''
    methods='''impl TabRuntime {\n    pub fn pane_viewport(&self, pane: PaneId) -> PaneViewportState {\n        self.pane_viewports.get(&pane).copied().unwrap_or_default()\n    }\n\n    pub fn pane_scroll_offset(&self, pane: PaneId) -> usize {\n        self.pane_viewport(pane).scroll_offset\n    }\n\n    pub fn pane_unseen_output(&self, pane: PaneId) -> bool {\n        self.pane_viewport(pane).unseen_output\n    }\n\n    pub fn scroll_pane_lines(\n        &mut self,\n        pane: PaneId,\n        delta: isize,\n        visible_rows: usize,\n    ) -> Result<PaneViewportState, &'static str> {\n        let history_lines = self\n            .terminal(pane)\n            .ok_or("unknown pane")?\n            .emulator()\n            .history_lines()\n            .len();\n        let state = self.pane_viewports.entry(pane).or_default();\n        state.scroll_lines(delta, history_lines, visible_rows);\n        Ok(*state)\n    }\n\n    pub fn scroll_pane_page(\n        &mut self,\n        pane: PaneId,\n        direction: isize,\n        visible_rows: usize,\n    ) -> Result<PaneViewportState, &'static str> {\n        let history_lines = self\n            .terminal(pane)\n            .ok_or("unknown pane")?\n            .emulator()\n            .history_lines()\n            .len();\n        let state = self.pane_viewports.entry(pane).or_default();\n        state.scroll_page(direction, history_lines, visible_rows);\n        Ok(*state)\n    }\n\n    pub fn set_pane_scroll_offset(\n        &mut self,\n        pane: PaneId,\n        offset: usize,\n        visible_rows: usize,\n    ) -> Result<PaneViewportState, &'static str> {\n        let history_lines = self\n            .terminal(pane)\n            .ok_or("unknown pane")?\n            .emulator()\n            .history_lines()\n            .len();\n        let state = self.pane_viewports.entry(pane).or_default();\n        state.scroll_offset = offset;\n        state.clamp(history_lines, visible_rows);\n        Ok(*state)\n    }\n\n    pub fn clamp_pane_viewport(\n        &mut self,\n        pane: PaneId,\n        visible_rows: usize,\n    ) -> Result<PaneViewportState, &'static str> {\n        let history_lines = self\n            .terminal(pane)\n            .ok_or("unknown pane")?\n            .emulator()\n            .history_lines()\n            .len();\n        let state = self.pane_viewports.entry(pane).or_default();\n        state.clamp(history_lines, visible_rows);\n        Ok(*state)\n    }\n\n    pub fn pane_ids(&self) -> Vec<PaneId> {\n'''
    if method_anchor not in w: raise SystemExit('TabRuntime impl anchor missing')
    w=w.replace(method_anchor,methods,1)
    # initialize map in every TabRuntime literal in this file
    w=re.sub(r'(process_finished_reported: false,\n)(\s+)(panes,)', r'\1\2pane_viewports: BTreeMap::new(),\n\2\3', w)
    # poll per pane and preserve held viewport while history grows
    old='''    for terminal in tab.panes.values_mut() {\n        total += terminal.poll_output();\n    }\n'''
    new='''    let mut viewport_updates = Vec::new();\n    for (pane, terminal) in &mut tab.panes {\n        let before_lines = terminal.emulator().history_lines().len();\n        let read = terminal.poll_output();\n        let after_lines = terminal.emulator().history_lines().len();\n        total += read;\n        viewport_updates.push((*pane, read, after_lines.saturating_sub(before_lines)));\n    }\n    for (pane, read, new_history_lines) in viewport_updates {\n        tab.pane_viewports\n            .entry(pane)\n            .or_default()\n            .note_output(read, new_history_lines);\n    }\n'''
    if old not in w: raise SystemExit('poll_tab loop anchor missing')
    w=w.replace(old,new,1)
    workspace.write_text(w)

n=native.read_text()
if 'FORGEKONSOLE_V10005_PER_PANE_TRUE_SCROLL_ROOT' not in n:
    # keep legacy live-state field but make active pane state authoritative
    n=n.replace('scroll_offset: self.scroll_offset,','scroll_offset: self.active_pane_scroll_offset(),',1)
    n=n.replace('''        self.scroll_offset = state.scroll_offset;\n        self.selection = state.selection.map(LiveSelectionState::into_selection);\n''','''        self.scroll_offset = state.scroll_offset;\n        self.restore_active_pane_scroll_offset(state.scroll_offset);\n        self.selection = state.selection.map(LiveSelectionState::into_selection);\n''',1)
    n=n.replace('''        if read > 0 {\n            self.scroll_offset = 0;\n            if self.search_open {\n''','''        if read > 0 {\n            // FORGEKONSOLE_V10005_HELD_SCROLL_SURVIVES_OUTPUT\n            self.scroll_offset = self.active_pane_scroll_offset();\n            if self.search_open {\n''',1)
    n=n.replace('''    fn reset_active_interaction_state(&mut self) {\n        self.scroll_offset = 0;\n''','''    fn reset_active_interaction_state(&mut self) {\n        self.scroll_offset = self.active_pane_scroll_offset();\n''',1)
    # clamp after resize for each pane
    old='''            if active == Some(pane) {\n                self.terminal_grid_size = (cols, rows);\n            }\n'''
    new='''            if let Some(workspace) = self.workspace.as_mut()\n                && let Some(tab) = workspace.active_tab_mut()\n            {\n                let _ = tab.clamp_pane_viewport(pane, rows);\n            }\n            if active == Some(pane) {\n                self.terminal_grid_size = (cols, rows);\n                self.scroll_offset = self.active_pane_scroll_offset();\n            }\n'''
    if old not in n: raise SystemExit('resize pane anchor missing')
    n=n.replace(old,new,1)
    # rendering: inject viewport lookup and pass state
    old='''            let Some(runtime) = self\n                .workspace\n                .as_ref()\n                .and_then(WorkspaceRuntime::active_tab)\n                .and_then(|tab| tab.terminal(pane))\n            else {\n                continue;\n            };\n            let content_rect = self.power_pane_content_rect(pane_rect);\n            self.paint_terminal_runtime(\n                ui,\n                content_rect,\n                runtime,\n                focused && is_active,\n                is_active,\n            );\n'''
    new='''            let Some(tab) = self.workspace.as_ref().and_then(WorkspaceRuntime::active_tab) else {\n                continue;\n            };\n            let viewport = tab.pane_viewport(pane);\n            let Some(runtime) = tab.terminal(pane) else {\n                continue;\n            };\n            let content_rect = self.power_pane_content_rect(pane_rect);\n            self.paint_terminal_runtime(\n                ui,\n                content_rect,\n                runtime,\n                focused && is_active,\n                is_active,\n                pane,\n                viewport.scroll_offset,\n                viewport.unseen_output,\n            );\n'''
    if old not in n: raise SystemExit('root terminal render anchor missing')
    n=n.replace(old,new,1)
    n=n.replace('''        runtime: &TerminalRuntime,\n        focused: bool,\n        active: bool,\n    ) {\n''','''        runtime: &TerminalRuntime,\n        focused: bool,\n        active: bool,\n        pane: PaneId,\n        scroll_offset: usize,\n        unseen_output: bool,\n    ) {\n''',1)
    n=n.replace('''        let scroll = if active { self.scroll_offset } else { 0 };\n        let end = lines.len().saturating_sub(scroll.min(lines.len()));\n''','''        // FORGEKONSOLE_V10005_PER_PANE_TRUE_SCROLL_ROOT\n        let max_scroll = lines.len().saturating_sub(visible_rows);\n        let scroll = scroll_offset.min(max_scroll);\n        let end = lines.len().saturating_sub(scroll);\n''',1)
    # paint scrollbar before cursor
    cursor_anchor='''        if focused && scroll == 0 && runtime.emulator().cursor_visible() {\n'''
    scrollbar='''        self.paint_pane_scrollbar(ui, rect, pane, lines.len(), visible_rows, scroll, unseen_output);\n        if focused && scroll == 0 && runtime.emulator().cursor_visible() {\n'''
    if cursor_anchor not in n: raise SystemExit('cursor anchor missing')
    n=n.replace(cursor_anchor,scrollbar,1)
    # history window active pane offset
    n=n.replace('''        let end = line_count.saturating_sub(self.scroll_offset.min(line_count));\n''','''        let scroll_offset = self.active_pane_scroll_offset();\n        let end = line_count.saturating_sub(scroll_offset.min(line_count));\n''',1)
    # page-up/down intercept before generic key branch
    generic='''                egui::Event::Key {\n                    key,\n                    pressed: true,\n                    modifiers,\n                    ..\n                } => {\n'''
    page='''                // FORGEKONSOLE_V10005_PER_PANE_PAGE_SCROLL\n                egui::Event::Key {\n                    key: egui::Key::PageUp,\n                    pressed: true,\n                    modifiers,\n                    ..\n                } if !modifiers.alt && !modifiers.ctrl && !modifiers.mac_cmd => {\n                    self.scroll_active_pane_page(1, rect);\n                }\n                egui::Event::Key {\n                    key: egui::Key::PageDown,\n                    pressed: true,\n                    modifiers,\n                    ..\n                } if !modifiers.alt && !modifiers.ctrl && !modifiers.mac_cmd => {\n                    self.scroll_active_pane_page(-1, rect);\n                }\n'''+generic
    if generic not in n: raise SystemExit('generic key anchor missing')
    n=n.replace(generic,page,1)
    # replace wheel/local scroll functions as a block
    start=n.index('    fn handle_mouse_wheel(')
    end=n.index('    fn start_terminal_selection(', start)
    new_funcs=r'''    fn handle_mouse_wheel(
        &mut self,
        ctx: &egui::Context,
        rect: egui::Rect,
        delta_y: f32,
        modifiers: egui::Modifiers,
    ) {
        if modifiers.ctrl {
            self.adjust_terminal_font_size(ctx, delta_y);
            self.resize_workspace_panes(rect);
            return;
        }
        let pointer = ctx.input(|input| input.pointer.latest_pos());
        let target = pointer
            .and_then(|pos| self.pane_rect_at(rect, pos))
            .or_else(|| {
                let active = self
                    .workspace
                    .as_ref()
                    .and_then(WorkspaceRuntime::active_pane_id);
                self.workspace_pane_rects(rect)
                    .into_iter()
                    .find(|(pane, _)| Some(*pane) == active)
            });
        let Some((pane, pane_rect)) = target else {
            return;
        };
        let content_rect = self.power_pane_content_rect(pane_rect);
        // FORGEKONSOLE_V10005_POINTER_TARGETED_PANE_SCROLL
        self.scroll_pane_local(pane, delta_y, content_rect);
        if self.selection_dragging
            && self.workspace.as_ref().and_then(WorkspaceRuntime::active_pane_id) == Some(pane)
            && let Some(pos) = pointer
        {
            self.update_terminal_selection(content_rect, pos);
        }
    }

    fn scroll_pane_local(&mut self, pane: PaneId, delta_y: f32, rect: egui::Rect) {
        let (_, cell_height) = self.terminal_cell_metrics();
        let visible_rows = ((rect.height() - TERMINAL_PADDING * 2.0).max(cell_height) / cell_height)
            .floor()
            .max(1.0) as usize;
        let steps = (delta_y.abs() / 12.0).ceil().max(1.0) as isize;
        let delta = if delta_y > 0.0 {
            steps
        } else if delta_y < 0.0 {
            -steps
        } else {
            0
        };
        if let Some(workspace) = self.workspace.as_mut()
            && let Some(tab) = workspace.active_tab_mut()
            && let Ok(state) = tab.scroll_pane_lines(pane, delta, visible_rows)
        {
            if tab.active_pane == pane {
                self.scroll_offset = state.scroll_offset;
            }
        }
    }

    fn scroll_active_pane_page(&mut self, direction: isize, rect: egui::Rect) {
        let pane_rect = self.active_pane_rect(rect);
        let (_, cell_height) = self.terminal_cell_metrics();
        let visible_rows = ((pane_rect.height() - TERMINAL_PADDING * 2.0).max(cell_height) / cell_height)
            .floor()
            .max(1.0) as usize;
        if let Some(workspace) = self.workspace.as_mut()
            && let Some(tab) = workspace.active_tab_mut()
        {
            let pane = tab.active_pane;
            if let Ok(state) = tab.scroll_pane_page(pane, direction, visible_rows) {
                self.scroll_offset = state.scroll_offset;
            }
        }
    }

    fn scroll_selection_view(&mut self, rect: egui::Rect, pos: egui::Pos2) {
        let (_, cell_height) = self.terminal_cell_metrics();
        let rows = if pos.y < rect.top() {
            ((rect.top() - pos.y) / cell_height).ceil().clamp(1.0, 4.0) as isize
        } else if pos.y > rect.bottom() {
            -(((pos.y - rect.bottom()) / cell_height).ceil().clamp(1.0, 4.0) as isize)
        } else {
            0
        };
        if rows == 0 {
            return;
        }
        let visible_rows = ((rect.height() - TERMINAL_PADDING * 2.0).max(cell_height) / cell_height)
            .floor()
            .max(1.0) as usize;
        if let Some(workspace) = self.workspace.as_mut()
            && let Some(tab) = workspace.active_tab_mut()
        {
            let pane = tab.active_pane;
            if let Ok(state) = tab.scroll_pane_lines(pane, rows, visible_rows) {
                self.scroll_offset = state.scroll_offset;
            }
        }
    }

    fn clamp_scroll_offset(&mut self) {
        let visible_rows = self.terminal_grid_size.1.max(1);
        if let Some(workspace) = self.workspace.as_mut()
            && let Some(tab) = workspace.active_tab_mut()
        {
            let pane = tab.active_pane;
            if let Ok(state) = tab.clamp_pane_viewport(pane, visible_rows) {
                self.scroll_offset = state.scroll_offset;
            }
        }
    }

    fn active_pane_scroll_offset(&self) -> usize {
        self.workspace
            .as_ref()
            .and_then(WorkspaceRuntime::active_tab)
            .map(|tab| tab.pane_scroll_offset(tab.active_pane))
            .unwrap_or(self.scroll_offset)
    }

    fn restore_active_pane_scroll_offset(&mut self, offset: usize) {
        let visible_rows = self.terminal_grid_size.1.max(1);
        if let Some(workspace) = self.workspace.as_mut()
            && let Some(tab) = workspace.active_tab_mut()
        {
            let pane = tab.active_pane;
            let _ = tab.set_pane_scroll_offset(pane, offset, visible_rows);
        }
    }

    fn paint_pane_scrollbar(
        &self,
        ui: &egui::Ui,
        rect: egui::Rect,
        _pane: PaneId,
        history_lines: usize,
        visible_rows: usize,
        scroll_offset: usize,
        unseen_output: bool,
    ) {
        let max_scroll = history_lines.saturating_sub(visible_rows.max(1));
        if max_scroll == 0 {
            return;
        }
        let hovered = ui.rect_contains_pointer(rect);
        if scroll_offset == 0 && !unseen_output && !hovered {
            return;
        }
        let painter = ui.painter();
        let track = egui::Rect::from_min_max(
            egui::pos2(rect.right() - 5.0, rect.top() + 5.0),
            egui::pos2(rect.right() - 2.0, rect.bottom() - 5.0),
        );
        let fraction = (visible_rows as f32 / history_lines.max(1) as f32).clamp(0.08, 1.0);
        let thumb_height = (track.height() * fraction).max(18.0).min(track.height());
        let travel = (track.height() - thumb_height).max(0.0);
        let ratio = scroll_offset as f32 / max_scroll.max(1) as f32;
        let thumb_top = track.bottom() - thumb_height - travel * ratio;
        painter.rect_filled(track, 2.0, color(self.theme.glass_alt, 60));
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(track.left(), thumb_top), egui::vec2(track.width(), thumb_height)),
            2.0,
            color(self.theme.violet, if hovered || scroll_offset > 0 { 185 } else { 120 }),
        );
        if unseen_output {
            painter.circle_filled(
                egui::pos2(track.center().x, track.bottom() - 2.0),
                2.2,
                color(self.theme.cyan, 245),
            );
        }
    }

'''
    n=n[:start]+new_funcs+n[end:]
    # search navigation target now writes active pane viewport
    old='''        self.scroll_offset = line_count.saturating_sub(desired_end);\n        self.clamp_scroll_offset();\n'''
    new='''        let offset = line_count.saturating_sub(desired_end);\n        let visible_rows = self.terminal_grid_size.1.max(1);\n        if let Some(workspace) = self.workspace.as_mut()\n            && let Some(tab) = workspace.active_tab_mut()\n        {\n            let pane = tab.active_pane;\n            if let Ok(state) = tab.set_pane_scroll_offset(pane, offset, visible_rows) {\n                self.scroll_offset = state.scroll_offset;\n            }\n        }\n'''
    if old not in n: raise SystemExit('search scroll anchor missing')
    n=n.replace(old,new,1)
    # settings active offset display
    n=n.replace('egui::RichText::new(self.scroll_offset.to_string())','egui::RichText::new(self.active_pane_scroll_offset().to_string())',1)
    native.write_text(n)

d=detached.read_text()
if 'FORGEKONSOLE_V10005_DETACHED_PER_PANE_TRUE_SCROLL' not in d:
    # render each pane with its own viewport state
    old='''            if let Some(runtime) = tab.terminal_mut(*pane) {\n                let cols = ((pane_rect.width() - PADDING * 2.0) / CELL_WIDTH)\n                    .floor()\n                    .max(1.0) as usize;\n                let rows = ((pane_rect.height() - PADDING * 2.0) / CELL_HEIGHT)\n                    .floor()\n                    .max(1.0) as usize;\n                let _ = runtime.resize(cols, rows);\n                paint_terminal(ui, *pane_rect, runtime, is_active, theme);\n            }\n'''
    new='''            let cols = ((pane_rect.width() - PADDING * 2.0) / CELL_WIDTH)\n                .floor()\n                .max(1.0) as usize;\n            let rows = ((pane_rect.height() - PADDING * 2.0) / CELL_HEIGHT)\n                .floor()\n                .max(1.0) as usize;\n            if let Some(runtime) = tab.terminal_mut(*pane) {\n                let _ = runtime.resize(cols, rows);\n            }\n            let _ = tab.clamp_pane_viewport(*pane, rows);\n            let viewport = tab.pane_viewport(*pane);\n            if let Some(runtime) = tab.terminal(*pane) {\n                paint_terminal(\n                    ui,\n                    *pane_rect,\n                    runtime,\n                    is_active,\n                    theme,\n                    viewport.scroll_offset,\n                    viewport.unseen_output,\n                );\n            }\n'''
    if old not in d: raise SystemExit('detached pane render anchor missing')
    d=d.replace(old,new,1)
    # insert pointer-targeted wheel handling without disturbing v1.0.04 clipboard menu
    anchor='''        // FORGEKONSOLE_V10004_DETACHED_SYSTEM_CLIPBOARD_PASTE\n        response.context_menu(|ui| {\n            if ui.button("Paste").clicked() {\n                if let Ok(text) = read_system_clipboard_text()\n                    && let Some(next) = detached_paste_text(tab, text)\n                {\n                    action = next;\n                }\n                ui.close_menu();\n            }\n        });\n\n        if response.has_focus() {\n'''
    insert='''        // FORGEKONSOLE_V10004_DETACHED_SYSTEM_CLIPBOARD_PASTE\n        response.context_menu(|ui| {\n            if ui.button("Paste").clicked() {\n                if let Ok(text) = read_system_clipboard_text()\n                    && let Some(next) = detached_paste_text(tab, text)\n                {\n                    action = next;\n                }\n                ui.close_menu();\n            }\n        });\n\n        // FORGEKONSOLE_V10005_DETACHED_PER_PANE_TRUE_SCROLL\n        let detached_events = ctx.input(|input| input.events.clone());\n        for event in &detached_events {\n            if let egui::Event::MouseWheel { delta, modifiers, .. } = event {\n                if modifiers.ctrl {\n                    continue;\n                }\n                if let Some(pos) = ctx.input(|input| input.pointer.latest_pos())\n                    && let Some((pane, pane_rect)) = pane_rects.iter().find(|(_, rect)| rect.contains(pos))\n                {\n                    let visible_rows = ((pane_rect.height() - PADDING * 2.0) / CELL_HEIGHT)\n                        .floor()\n                        .max(1.0) as usize;\n                    let steps = (delta.y.abs() / 12.0).ceil().max(1.0) as isize;\n                    let direction = if delta.y > 0.0 { steps } else { -steps };\n                    let _ = tab.scroll_pane_lines(*pane, direction, visible_rows);\n                }\n            }\n        }\n\n        if response.has_focus() {\n'''
    if anchor not in d: raise SystemExit('detached clipboard/focus anchor missing')
    d=d.replace(anchor,insert,1)
    # Add page up/down key intercept before generic key mapping
    anchor='''                    egui::Event::Key {\n                        key,\n                        pressed: true,\n                        modifiers,\n                        ..\n                    } => {\n'''
    insert='''                    egui::Event::Key {\n                        key: egui::Key::PageUp,\n                        pressed: true,\n                        modifiers,\n                        ..\n                    } if !modifiers.alt && !modifiers.ctrl && !modifiers.mac_cmd => {\n                        let pane = tab.active_pane;\n                        let rows = pane_rects\n                            .iter()\n                            .find(|(id, _)| *id == pane)\n                            .map(|(_, rect)| ((rect.height() - PADDING * 2.0) / CELL_HEIGHT).floor().max(1.0) as usize)\n                            .unwrap_or(1);\n                        let _ = tab.scroll_pane_page(pane, 1, rows);\n                    }\n                    egui::Event::Key {\n                        key: egui::Key::PageDown,\n                        pressed: true,\n                        modifiers,\n                        ..\n                    } if !modifiers.alt && !modifiers.ctrl && !modifiers.mac_cmd => {\n                        let pane = tab.active_pane;\n                        let rows = pane_rects\n                            .iter()\n                            .find(|(id, _)| *id == pane)\n                            .map(|(_, rect)| ((rect.height() - PADDING * 2.0) / CELL_HEIGHT).floor().max(1.0) as usize)\n                            .unwrap_or(1);\n                        let _ = tab.scroll_pane_page(pane, -1, rows);\n                    }\n'''+anchor
    if anchor not in d: raise SystemExit('detached key anchor missing')
    d=d.replace(anchor,insert,1)
    # paint signature and body
    d=d.replace('''    runtime: &crate::terminal_runtime::TerminalRuntime,\n    active: bool,\n    theme: &DragonGlassTheme,\n) {\n''','''    runtime: &crate::terminal_runtime::TerminalRuntime,\n    active: bool,\n    theme: &DragonGlassTheme,\n    scroll_offset: usize,\n    unseen_output: bool,\n) {\n''',1)
    d=d.replace('''    let start = lines.len().saturating_sub(visible_rows);\n    let origin = rect.left_top() + egui::vec2(PADDING, PADDING);\n    for (row, cells) in lines[start..].iter().enumerate() {\n''','''    let max_scroll = lines.len().saturating_sub(visible_rows);\n    let scroll = scroll_offset.min(max_scroll);\n    let end = lines.len().saturating_sub(scroll);\n    let start = end.saturating_sub(visible_rows);\n    let origin = rect.left_top() + egui::vec2(PADDING, PADDING);\n    for (row, cells) in lines[start..end].iter().enumerate() {\n''',1)
    # add detached scrollbar before function close by replacing final loop ending near next fn
    anchor='''        );\n    }\n}\n\nfn paint_styled_line(\n'''
    add='''        );\n    }\n    let hovered = ui.rect_contains_pointer(rect);\n    if max_scroll > 0 && (scroll > 0 || unseen_output || hovered) {\n        let track = egui::Rect::from_min_max(\n            egui::pos2(rect.right() - 5.0, rect.top() + 5.0),\n            egui::pos2(rect.right() - 2.0, rect.bottom() - 5.0),\n        );\n        let fraction = (visible_rows as f32 / lines.len().max(1) as f32).clamp(0.08, 1.0);\n        let thumb_height = (track.height() * fraction).max(18.0).min(track.height());\n        let travel = (track.height() - thumb_height).max(0.0);\n        let ratio = scroll as f32 / max_scroll.max(1) as f32;\n        let thumb_top = track.bottom() - thumb_height - travel * ratio;\n        ui.painter().rect_filled(track, 2.0, rgba(theme.glass_alt, 60));\n        ui.painter().rect_filled(\n            egui::Rect::from_min_size(egui::pos2(track.left(), thumb_top), egui::vec2(track.width(), thumb_height)),\n            2.0,\n            rgba(theme.violet, if hovered || scroll > 0 { 185 } else { 120 }),\n        );\n        if unseen_output {\n            ui.painter().circle_filled(\n                egui::pos2(track.center().x, track.bottom() - 2.0),\n                2.2,\n                rgba(theme.cyan, 245),\n            );\n        }\n    }\n}\n\nfn paint_styled_line(\n'''
    if anchor not in d: raise SystemExit('detached paint end anchor missing')
    d=d.replace(anchor,add,1)
    detached.write_text(d)

print('FORGEKONSOLE_DEV_PER_PANE_TRUE_SCROLL_PATCH=APPLIED_OR_PRESENT')
PY_SCROLL_PATCH
PATCH_SCROLL_RC=${PIPESTATUS[0]}
(( PATCH_SCROLL_RC == 0 )) || fail_now PATCH_V10005_PER_PANE_TRUE_SCROLL "$PATCH_SCROLL_RC"

log "FORGEKONSOLE_DEV_STAGE=STATIC_V10005_PER_PANE_TRUE_SCROLL"
python3 - "$WORKSPACE_RUNTIME" "$NATIVE" "$WORK/apps/forgekonsole/src/detached_view.rs" <<'PY_SCROLL_STATIC' 2>&1 | tee -a "$LOG"
from pathlib import Path
import sys
workspace = Path(sys.argv[1]).read_text()
native = Path(sys.argv[2]).read_text()
detached = Path(sys.argv[3]).read_text()
checks = {
    "per-pane viewport state": "FORGEKONSOLE_V10005_PER_PANE_TRUE_SCROLL_STATE" in workspace,
    "pane viewport map": "pane_viewports: BTreeMap<PaneId, PaneViewportState>" in workspace,
    "independent line scroll": "pub fn scroll_pane_lines(" in workspace,
    "independent page scroll": "pub fn scroll_pane_page(" in workspace,
    "resize clamp": "pub fn clamp_pane_viewport(" in workspace,
    "held output preservation": "note_output(read, new_history_lines)" in workspace,
    "root per-pane render": "FORGEKONSOLE_V10005_PER_PANE_TRUE_SCROLL_ROOT" in native,
    "pointer targeted wheel": "FORGEKONSOLE_V10005_POINTER_TARGETED_PANE_SCROLL" in native,
    "page keys": "FORGEKONSOLE_V10005_PER_PANE_PAGE_SCROLL" in native,
    "root scrollbar": "fn paint_pane_scrollbar(" in native,
    "held scroll no snap bottom": "FORGEKONSOLE_V10005_HELD_SCROLL_SURVIVES_OUTPUT" in native,
    "detached parity": "FORGEKONSOLE_V10005_DETACHED_PER_PANE_TRUE_SCROLL" in detached,
    "detached wheel": "tab.scroll_pane_lines(*pane" in detached,
    "detached page": "tab.scroll_pane_page(pane" in detached,
    "detached scrollbar": "unseen_output: bool" in detached and "thumb_height" in detached,
}
failed = [name for name, ok in checks.items() if not ok]
if failed:
    raise SystemExit("v1.0.05 per-pane true scrolling static contract failed: " + ", ".join(failed))
print("FORGEKONSOLE_DEV_V10005_PER_PANE_TRUE_SCROLL_STATIC=PASS")
print("FORGEKONSOLE_DEV_PER_PANE_SCROLL_POLICY=INDEPENDENT_POINTER_TARGETED_PAGE_KEYS_FOLLOW_BOTTOM_HELD_SCROLL_UNSEEN_OUTPUT_DETACHED_PARITY")
PY_SCROLL_STATIC
SCROLL_STATIC_RC=${PIPESTATUS[0]}

SCROLL_GREEN_RC=0
if [[ "${1:-}" != "--patch-only" ]]; then
  log "FORGEKONSOLE_DEV_STAGE=PER_PANE_TRUE_SCROLL_TDD_GREEN"
  set +e
  (cd "$WORK" && cargo test -p forgekonsole --test v10005_per_pane_true_scrolling_contract -- --nocapture) 2>&1 | tee -a "$LOG"
  SCROLL_GREEN_RC=${PIPESTATUS[0]}
  set -e
  if (( SCROLL_GREEN_RC == 0 )); then
    log "FORGEKONSOLE_DEV_PER_PANE_TRUE_SCROLL_TDD_GREEN=PASS"
  else
    log "FORGEKONSOLE_DEV_PER_PANE_TRUE_SCROLL_TDD_GREEN=FAIL:$SCROLL_GREEN_RC"
  fi
fi


TRANSP_SETTINGS_TEST="$WORK/apps/forgekonsole/tests/v10005_transparency_settings_density_contract.rs"
POWER_SHELL="$WORK/apps/forgekonsole/src/power_shell.rs"
V0514_SETTINGS_TEST="$WORK/apps/forgekonsole/tests/v0514_settings_pane_unification_contract.rs"
V058_WORK_UI_TEST="$WORK/apps/forgekonsole/tests/v058_aether_work_ui_contract.rs"

log "FORGEKONSOLE_DEV_STAGE=TRANSPARENCY_SETTINGS_DENSITY_TEST_INSTALL"
cat > "$TRANSP_SETTINGS_TEST" <<'RS_TRANSP_SETTINGS_TEST'
use forgekonsole::power_shell::{
    POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT, surface_alpha_from_transparency_percent,
    transparency_percent_from_surface_alpha,
};

const NATIVE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/native.rs"));
const POWER: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/power_shell.rs"));

#[test]
fn transparency_can_reach_99_percent_without_becoming_fully_invisible() {
    assert_eq!(POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT, 99);
    assert_eq!(surface_alpha_from_transparency_percent(99.0), 3);
    assert_eq!(surface_alpha_from_transparency_percent(100.0), 3);
    assert_eq!(transparency_percent_from_surface_alpha(0), 99.0);

    assert!(POWER.contains("POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT: u8 = 99"));
    assert!(NATIVE.contains("FORGEKONSOLE_V10005_TRANSPARENCY_99_PERCENT"));
    assert!(NATIVE.contains("POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT"));
    assert!(!NATIVE.contains("0.0..=95.0"));
    assert!(!NATIVE.contains("clamp(0.0, 95.0)"));
    assert!(!NATIVE.contains(".min(95)"));
}

#[test]
fn power_settings_density_tracks_assigned_pane_width() {
    let start = NATIVE
        .find("fn power_workspace_settings(")
        .expect("power settings start");
    let end = NATIVE[start..]
        .find("fn power_workspace_status_bar(")
        .map(|offset| start + offset)
        .expect("power settings end");
    let block = &NATIVE[start..end];

    assert!(block.contains("FORGEKONSOLE_V10005_SETTINGS_DENSITY_RESPONSIVE"));
    assert!(block.contains("power_settings_outer_margin(ui.available_width())"));
    assert!(block.contains("power_settings_item_spacing_y(ui.available_width())"));
    assert!(block.contains("power_settings_control_width(ui)"));
    assert!(!block.contains("egui::vec2(118.0, 20.0)"));
}

#[test]
fn settings_rows_sections_and_header_have_dense_modes_without_horizontal_overflow() {
    assert!(NATIVE.contains("fn power_settings_outer_margin(width: f32) -> f32"));
    assert!(NATIVE.contains("fn power_settings_item_spacing_y(width: f32) -> f32"));
    assert!(NATIVE.contains("fn power_settings_control_width(ui: &egui::Ui) -> f32"));
    assert!(NATIVE.contains("fn power_settings_row_metrics(width: f32) -> (i8, i8, f32)"));
    assert!(NATIVE.contains("fn power_settings_section_metrics(width: f32) -> (f32, f32)"));
    assert!(NATIVE.contains("SETTINGS_DENSE_THRESHOLD"));
    assert!(NATIVE.contains("SETTINGS_HEADER_COMPACT_THRESHOLD"));
    assert!(NATIVE.contains("let display_title = if compact_header { \"Settings\" } else { title };"));
    assert!(NATIVE.contains("ui.set_max_width(viewport_width)"));
    assert!(NATIVE.contains("ui.set_width(viewport_width)"));
}
RS_TRANSP_SETTINGS_TEST
log "FORGEKONSOLE_DEV_TRANSPARENCY_SETTINGS_DENSITY_REGRESSION_TEST=INSTALLED"

TRANSP_SETTINGS_ALREADY=0
if grep -Fq 'FORGEKONSOLE_V10005_TRANSPARENCY_99_PERCENT' "$NATIVE" \
   && grep -Fq 'FORGEKONSOLE_V10005_SETTINGS_DENSITY_RESPONSIVE' "$NATIVE" \
   && grep -Fq 'POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT: u8 = 99' "$POWER_SHELL"; then
  TRANSP_SETTINGS_ALREADY=1
fi

TRANSP_SETTINGS_RED_RC=0
if [[ "${1:-}" != "--patch-only" && "$TRANSP_SETTINGS_ALREADY" -eq 0 ]]; then
  log "FORGEKONSOLE_DEV_STAGE=TRANSPARENCY_SETTINGS_DENSITY_TDD_RED"
  TS_RED_LOG="$TMPDIR_DEV/v10005-transparency-settings-density-red.log"
  set +e
  (cd "$WORK" && cargo test -p forgekonsole --test v10005_transparency_settings_density_contract -- --nocapture) >"$TS_RED_LOG" 2>&1
  TS_RED_STATUS=$?
  set -e
  cat "$TS_RED_LOG" | tee -a "$LOG"
  if (( TS_RED_STATUS != 0 ))      && grep -Eq 'POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT|v10005_transparency_settings_density_contract' "$TS_RED_LOG"; then
    log "FORGEKONSOLE_DEV_TRANSPARENCY_SETTINGS_DENSITY_TDD_RED=PASS_EXPECTED_FAILURE"
  else
    log "FORGEKONSOLE_DEV_TRANSPARENCY_SETTINGS_DENSITY_TDD_RED=FAIL_WRONG_FAILURE_OR_ALREADY_GREEN"
    TRANSP_SETTINGS_RED_RC=1
  fi
else
  log "FORGEKONSOLE_DEV_TRANSPARENCY_SETTINGS_DENSITY_TDD_RED=SKIP_ALREADY_APPLIED_OR_PATCH_ONLY"
fi

log "FORGEKONSOLE_DEV_STAGE=PATCH_V10005_TRANSPARENCY_SETTINGS_DENSITY"
python3 - "$POWER_SHELL" "$NATIVE" "$V0514_SETTINGS_TEST" "$V058_WORK_UI_TEST" <<'PY_TRANSP_SETTINGS' 2>&1 | tee -a "$LOG"
from pathlib import Path
import re
import sys

power_path, native_path, v0514_path, v058_path = map(Path, sys.argv[1:])
power = power_path.read_text()
native = native_path.read_text()

# One canonical maximum prevents the old 95% cap from drifting between the
# conversion helpers, persisted state, live-state restore, and both settings UIs.
if "POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT: u8 = 99" not in power:
    anchor = "pub const POWER_WORKSPACE_DEFAULT_TRANSPARENCY_PERCENT: u8 = 15;\n"
    if anchor not in power:
        raise SystemExit("power-shell transparency default anchor missing")
    power = power.replace(
        anchor,
        anchor + "pub const POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT: u8 = 99;\n",
        1,
    )
power = power.replace(
    "percent.clamp(0.0, 95.0)",
    "percent.clamp(0.0, f32::from(POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT))",
)
power = power.replace(
    ").clamp(0.0, 95.0)",
    ").clamp(0.0, f32::from(POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT))",
)
power_path.write_text(power)

# Import the maximum next to the existing power-shell constants.
grouped = re.search(r"use crate::power_shell::\{(?P<body>.*?)\};", native, re.S)
if grouped is None:
    raise SystemExit("native power_shell grouped import missing")
if "POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT" not in grouped.group("body"):
    body = grouped.group("body")
    insert_after = "POWER_WORKSPACE_DEFAULT_TRANSPARENCY_PERCENT,"
    if insert_after not in body:
        raise SystemExit("native power-shell max import anchor missing")
    body = body.replace(
        insert_after,
        insert_after + "\n    POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT,",
        1,
    )
    native = native[:grouped.start("body")] + body + native[grouped.end("body"):]

# Raise every transparency-specific cap from 95% to the canonical 99% max.
native = native.replace(
    "startup_config.transparency_percent <= 95",
    "startup_config.transparency_percent <= POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT",
)
native = native.replace(
    "state.transparency_percent.min(95)",
    "state.transparency_percent.min(POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT)",
)
native = native.replace(
    "percent.min(95)",
    "percent.min(POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT)",
)
native = native.replace(
    ".clamp(0.0, 95.0)",
    ".clamp(0.0, f32::from(POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT))",
)
native = native.replace(
    "0.0..=95.0",
    "0.0..=f32::from(POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT)",
)
native = native.replace("5.0..=100.0", "1.0..=100.0")

if "FORGEKONSOLE_V10005_TRANSPARENCY_99_PERCENT" not in native:
    anchor = "    fn set_transparency_percent(&mut self, ctx: &egui::Context, percent: u8) {\n"
    if anchor not in native:
        raise SystemExit("set_transparency_percent anchor missing")
    native = native.replace(
        anchor,
        anchor + "        // FORGEKONSOLE_V10005_TRANSPARENCY_99_PERCENT\n",
        1,
    )

# Responsive settings density constants.
if "const SETTINGS_DENSE_THRESHOLD: f32" not in native:
    anchor = "const SETTINGS_STACK_THRESHOLD: f32 = 250.0;\n"
    if anchor not in native:
        raise SystemExit("settings stack threshold anchor missing")
    native = native.replace(
        anchor,
        anchor
        + "const SETTINGS_DENSE_THRESHOLD: f32 = 180.0;\n"
        + "const SETTINGS_HEADER_COMPACT_THRESHOLD: f32 = 170.0;\n",
        1,
    )

# Power-settings pane: assigned width determines margins and vertical density.
settings_start = native.find("    fn power_workspace_settings(&mut self, ui: &mut egui::Ui) {")
settings_end = native.find("    fn power_workspace_status_bar(", settings_start)
if settings_start < 0 or settings_end < 0:
    raise SystemExit("power settings function boundaries missing")
settings = native[settings_start:settings_end]

if "FORGEKONSOLE_V10005_SETTINGS_DENSITY_RESPONSIVE" not in settings:
    anchor = "        let theme = self.theme.clone();\n"
    if anchor not in settings:
        raise SystemExit("power settings theme anchor missing")
    settings = settings.replace(
        anchor,
        anchor
        + "        // FORGEKONSOLE_V10005_SETTINGS_DENSITY_RESPONSIVE\n"
        + "        let settings_margin = power_settings_outer_margin(ui.available_width());\n",
        1,
    )
settings = settings.replace(".inner_margin(11.0)", ".inner_margin(settings_margin)", 1)
settings = settings.replace(
    "ui.spacing_mut().item_spacing.y = 6.0;",
    "ui.spacing_mut().item_spacing.y = power_settings_item_spacing_y(ui.available_width());",
    1,
)
settings = settings.replace(
    "egui::vec2(118.0, 20.0)",
    "egui::vec2(power_settings_control_width(ui), 20.0)",
)

# Palette swatches shrink with the row instead of forcing a minimum width.
old_palette = """                                    ui.horizontal(|ui| {
                                        for swatch in [
                                            theme.salmon,
                                            theme.amber,
                                            theme.cyan,
                                            theme.violet,
                                            theme.lavender,
                                        ] {
                                            let (rect, _) = ui.allocate_exact_size(
                                                egui::vec2(15.0, 15.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().circle_filled(
                                                rect.center(),
                                                5.75,
                                                color(swatch, 255),
                                            );
                                        }
                                    });
"""
if old_palette in settings:
    new_palette = """                                    ui.horizontal(|ui| {
                                        let swatch_size =
                                            (ui.available_width() / 5.5).clamp(7.0, 15.0);
                                        let swatch_radius = (swatch_size * 0.38).max(2.5);
                                        for swatch in [
                                            theme.salmon,
                                            theme.amber,
                                            theme.cyan,
                                            theme.violet,
                                            theme.lavender,
                                        ] {
                                            let (rect, _) = ui.allocate_exact_size(
                                                egui::vec2(swatch_size, swatch_size),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().circle_filled(
                                                rect.center(),
                                                swatch_radius,
                                                color(swatch, 255),
                                            );
                                        }
                                    });
"""
    settings = settings.replace(old_palette, new_palette, 1)

native = native[:settings_start] + settings + native[settings_end:]

# Density helpers live beside the settings widgets and are pure width-derived
# functions so the pane remains deterministic under arbitrary surrounding sizes.
helper_anchor = "fn power_settings_pane_header(\n"
if "fn power_settings_outer_margin(width: f32) -> f32" not in native:
    helper_index = native.find(helper_anchor)
    if helper_index < 0:
        raise SystemExit("power settings helper insertion anchor missing")
    helpers = """fn power_settings_outer_margin(width: f32) -> f32 {
    if width < 160.0 {
        3.0
    } else if width < 240.0 {
        5.0
    } else if width < 360.0 {
        7.0
    } else {
        11.0
    }
}

fn power_settings_item_spacing_y(width: f32) -> f32 {
    if width < SETTINGS_DENSE_THRESHOLD {
        3.0
    } else if width < SETTINGS_STACK_THRESHOLD {
        4.0
    } else {
        6.0
    }
}

fn power_settings_control_width(ui: &egui::Ui) -> f32 {
    ui.available_width().clamp(1.0, 118.0)
}

fn power_settings_row_metrics(width: f32) -> (i8, i8, f32) {
    if width < SETTINGS_DENSE_THRESHOLD {
        (3, 2, 9.0)
    } else if width < SETTINGS_STACK_THRESHOLD {
        (5, 3, 9.5)
    } else {
        (7, 4, 10.5)
    }
}

fn power_settings_section_metrics(width: f32) -> (f32, f32) {
    if width < SETTINGS_DENSE_THRESHOLD {
        (3.0, 9.0)
    } else if width < SETTINGS_STACK_THRESHOLD {
        (4.0, 9.5)
    } else {
        (6.0, 10.5)
    }
}

"""
    native = native[:helper_index] + helpers + native[helper_index:]

# Header shortens its title and scales its close control on narrow panes.
header_start = native.find("fn power_settings_pane_header(")
header_end = native.find("fn power_settings_row(", header_start)
if header_start < 0 or header_end < 0:
    raise SystemExit("power settings header boundaries missing")
header = native[header_start:header_end]
if "let compact_header = rect.width() < SETTINGS_HEADER_COMPACT_THRESHOLD;" not in header:
    anchor = """    let close_size = (POWER_WORKSPACE_PANE_HEADER_HEIGHT - 8.0).max(14.0);
"""
    if anchor not in header:
        raise SystemExit("settings header close size anchor missing")
    replacement = """    let compact_header = rect.width() < SETTINGS_HEADER_COMPACT_THRESHOLD;
    let display_title = if compact_header { "Settings" } else { title };
    let title_size = if compact_header { 9.5 } else { 11.0 };
    let close_size = (POWER_WORKSPACE_PANE_HEADER_HEIGHT - 8.0)
        .max(14.0)
        .min((rect.width() * 0.32).max(10.0));
"""
    header = header.replace(anchor, replacement, 1)
    header = header.replace(
        """    painter.circle_filled(
        egui::pos2(rect.left() + 11.0, rect.center().y),
        3.0,
        color(theme.cyan, 220),
    );
""",
        """    if !compact_header {
        painter.circle_filled(
            egui::pos2(rect.left() + 11.0, rect.center().y),
            3.0,
            color(theme.cyan, 220),
        );
    }
""",
        1,
    )
    header = header.replace(
        """        egui::pos2(rect.left() + 20.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::monospace(11.0),
""",
        """        egui::pos2(
            rect.left() + if compact_header { 6.0 } else { 20.0 },
            rect.center().y,
        ),
        egui::Align2::LEFT_CENTER,
        display_title,
        egui::FontId::monospace(title_size),
""",
        1,
    )
native = native[:header_start] + header + native[header_end:]

# Rows shrink margins/font size and stack controls at the existing threshold.
row_start = native.find("fn power_settings_row(")
row_end = native.find("fn power_settings_section(", row_start)
if row_start < 0 or row_end < 0:
    raise SystemExit("power settings row boundaries missing")
row = native[row_start:row_end]
if "power_settings_row_metrics" not in row.split("egui::Frame::NONE", 1)[0]:
    anchor = ") {\n    egui::Frame::NONE\n"
    if anchor not in row:
        raise SystemExit("power settings row frame anchor missing")
    row = row.replace(
        anchor,
        ") {\n"
        "    let row_width = ui.available_width().max(1.0);\n"
        "    let (horizontal_margin, vertical_margin, label_size) =\n"
        "        power_settings_row_metrics(row_width);\n"
        "    egui::Frame::NONE\n",
        1,
    )
row = row.replace(
    ".inner_margin(egui::Margin::symmetric(7, 4))",
    ".inner_margin(egui::Margin::symmetric(horizontal_margin, vertical_margin))",
    1,
)
row = row.replace(
    "let compact = ui.available_width() < SETTINGS_STACK_THRESHOLD;",
    "let compact = row_width < SETTINGS_STACK_THRESHOLD;",
    1,
)
row = row.replace(".size(10.5)", ".size(label_size)")
native = native[:row_start] + row + native[row_end:]

# Section cards also shed padding and typography as width gets tight.
section_start = native.find("fn power_settings_section(")
section_end = native.find("fn drawer_close_button(", section_start)
if section_start < 0 or section_end < 0:
    raise SystemExit("power settings section boundaries missing")
section = native[section_start:section_end]
if "power_settings_section_metrics" not in section.split("egui::Frame::NONE", 1)[0]:
    anchor = ") {\n    egui::Frame::NONE\n"
    if anchor not in section:
        raise SystemExit("power settings section frame anchor missing")
    section = section.replace(
        anchor,
        ") {\n"
        "    let (section_margin, title_size) =\n"
        "        power_settings_section_metrics(ui.available_width().max(1.0));\n"
        "    egui::Frame::NONE\n",
        1,
    )
section = section.replace(".inner_margin(6.0)", ".inner_margin(section_margin)", 1)
section = section.replace(".size(10.5)", ".size(title_size)", 1)
section = section.replace(
    "ui.spacing_mut().item_spacing.y = 5.0;",
    "ui.spacing_mut().item_spacing.y = power_settings_item_spacing_y(ui.available_width());",
    1,
)
native = native[:section_start] + section + native[section_end:]

native_path.write_text(native)

# Sync older source-contract tests with the new canonical range; this is a
# deliberate product change, not a compatibility regression.
for test_path in (v0514_path, v058_path):
    text = test_path.read_text()
    text = text.replace(
        'assert!(compact.contains("0.0..=95.0"));',
        'assert!(compact.contains("POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT"));',
    )
    text = text.replace(
        'assert!(native.contains("0.0..=95.0"));',
        'assert!(native.contains("POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT"));',
    )
    test_path.write_text(text)

print("FORGEKONSOLE_DEV_TRANSPARENCY_99_PERCENT_PATCH=APPLIED_OR_PRESENT")
print("FORGEKONSOLE_DEV_SETTINGS_DENSITY_PATCH=APPLIED_OR_PRESENT")
PY_TRANSP_SETTINGS
PATCH_TRANSP_SETTINGS_RC=${PIPESTATUS[0]}
(( PATCH_TRANSP_SETTINGS_RC == 0 )) || fail_now PATCH_V10005_TRANSPARENCY_SETTINGS_DENSITY "$PATCH_TRANSP_SETTINGS_RC"

log "FORGEKONSOLE_DEV_STAGE=STATIC_V10005_TRANSPARENCY_SETTINGS_DENSITY"
python3 - "$POWER_SHELL" "$NATIVE" "$V0514_SETTINGS_TEST" "$V058_WORK_UI_TEST" <<'PY_TRANSP_SETTINGS_STATIC' 2>&1 | tee -a "$LOG"
from pathlib import Path
import sys

power, native, v0514, v058 = [Path(value).read_text() for value in sys.argv[1:]]
settings_start = native.find("fn power_workspace_settings(")
settings_end = native.find("fn power_workspace_status_bar(", settings_start)
settings = native[settings_start:settings_end] if settings_start >= 0 and settings_end >= 0 else ""

checks = {
    "max constant": "POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT: u8 = 99" in power,
    "power clamp canonical": "f32::from(POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT)" in power,
    "native max import/use": "POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT" in native,
    "99 marker": "FORGEKONSOLE_V10005_TRANSPARENCY_99_PERCENT" in native,
    "old transparency slider gone": "0.0..=95.0" not in native,
    "old transparency clamp gone": "clamp(0.0, 95.0)" not in native,
    "old state min gone": ".min(95)" not in native,
    "responsive marker": "FORGEKONSOLE_V10005_SETTINGS_DENSITY_RESPONSIVE" in settings,
    "dynamic outer margin": "power_settings_outer_margin(ui.available_width())" in settings,
    "dynamic spacing": "power_settings_item_spacing_y(ui.available_width())" in settings,
    "dynamic controls": settings.count("power_settings_control_width(ui)") >= 3,
    "fixed settings slider width gone": "egui::vec2(118.0, 20.0)" not in settings,
    "row density helper": "fn power_settings_row_metrics(width: f32) -> (i8, i8, f32)" in native,
    "section density helper": "fn power_settings_section_metrics(width: f32) -> (f32, f32)" in native,
    "header compact title": 'let display_title = if compact_header { "Settings" } else { title };' in native,
    "bounded viewport width": "ui.set_max_width(viewport_width)" in native and "ui.set_width(viewport_width)" in native,
    "v0514 contract synced": "POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT" in v0514 and "0.0..=95.0" not in v0514,
    "v058 contract synced": "POWER_WORKSPACE_MAX_TRANSPARENCY_PERCENT" in v058 and "0.0..=95.0" not in v058,
}
failed = [name for name, ok in checks.items() if not ok]
if failed:
    raise SystemExit(
        "v1.0.05 transparency/settings density static contract failed: "
        + ", ".join(failed)
    )

print("FORGEKONSOLE_DEV_V10005_TRANSPARENCY_SETTINGS_DENSITY_STATIC=PASS")
print("FORGEKONSOLE_DEV_TRANSPARENCY_RANGE=0_TO_99_PERCENT")
print("FORGEKONSOLE_DEV_SETTINGS_DENSITY=RESPONSIVE_TO_ASSIGNED_PANE_WIDTH")
PY_TRANSP_SETTINGS_STATIC
TRANSP_SETTINGS_STATIC_RC=${PIPESTATUS[0]}

TRANSP_SETTINGS_GREEN_RC=0
if [[ "${1:-}" != "--patch-only" ]]; then
  log "FORGEKONSOLE_DEV_STAGE=TRANSPARENCY_SETTINGS_DENSITY_TDD_GREEN"
  set +e
  (cd "$WORK" && cargo test -p forgekonsole --test v10005_transparency_settings_density_contract -- --nocapture) 2>&1 | tee -a "$LOG"
  TRANSP_SETTINGS_GREEN_RC=${PIPESTATUS[0]}
  set -e
  if (( TRANSP_SETTINGS_GREEN_RC == 0 )); then
    log "FORGEKONSOLE_DEV_TRANSPARENCY_SETTINGS_DENSITY_TDD_GREEN=PASS"
  else
    log "FORGEKONSOLE_DEV_TRANSPARENCY_SETTINGS_DENSITY_TDD_GREEN=FAIL:$TRANSP_SETTINGS_GREEN_RC"
  fi
fi

if [[ "${1:-}" == "--patch-only" ]]; then
  if (( STATIC_RC == 0 && SYSTEM_STATIC_RC == 0 && SCROLL_STATIC_RC == 0 && TRANSP_SETTINGS_STATIC_RC == 0 )); then
    log "FORGEKONSOLE_DEV_PATCH_ONLY=PASS"
    exit 0
  fi
  log "FORGEKONSOLE_DEV_PATCH_ONLY=FAIL"
  exit 1
fi

FAILS=0
(( RED_RC == 0 )) || FAILS=$((FAILS + 1))
(( STATIC_RC == 0 )) || FAILS=$((FAILS + 1))
(( GREEN_RC == 0 )) || FAILS=$((FAILS + 1))
(( SYSTEM_RED_RC == 0 )) || FAILS=$((FAILS + 1))
(( SYSTEM_STATIC_RC == 0 )) || FAILS=$((FAILS + 1))
(( SYSTEM_GREEN_RC == 0 )) || FAILS=$((FAILS + 1))
(( SYSTEM_BRIDGE_GREEN_RC == 0 )) || FAILS=$((FAILS + 1))
(( SCROLL_RED_RC == 0 )) || FAILS=$((FAILS + 1))
(( SCROLL_STATIC_RC == 0 )) || FAILS=$((FAILS + 1))
(( SCROLL_GREEN_RC == 0 )) || FAILS=$((FAILS + 1))
(( TRANSP_SETTINGS_RED_RC == 0 )) || FAILS=$((FAILS + 1))
(( TRANSP_SETTINGS_STATIC_RC == 0 )) || FAILS=$((FAILS + 1))
(( TRANSP_SETTINGS_GREEN_RC == 0 )) || FAILS=$((FAILS + 1))

run_stage() {
  local key="$1"; shift
  log "FORGEKONSOLE_DEV_STAGE=$key"
  log "FORGEKONSOLE_DEV_COMMAND=$*"
  set +e
  (cd "$WORK" && "$@") 2>&1 | tee -a "$LOG"
  local rc=${PIPESTATUS[0]}
  set -e
  if (( rc == 0 )); then
    log "FORGEKONSOLE_DEV_${key}=PASS"
  else
    log "FORGEKONSOLE_DEV_${key}=FAIL:$rc"
    FAILS=$((FAILS + 1))
  fi
  return 0
}

set -e
run_stage CARGO_FMT_NORMALIZE cargo fmt --all
run_stage CARGO_FMT_CHECK cargo fmt --all --check
run_stage CARGO_GENERATE_LOCKFILE cargo generate-lockfile
run_stage CARGO_CLIPPY cargo clippy --workspace --all-targets --keep-going -- -D warnings
run_stage CARGO_TEST cargo test --workspace --no-fail-fast
rm -f "$WORK/target/release/forgekonsole" "$WORK/target/release/libforgekonsole.so" "$WORK/target/release/forgekonsole-verify"
log "FORGEKONSOLE_DEV_RELEASE_ARTIFACT_STALE_GUARD=PASS"
run_stage CARGO_BUILD_RELEASE cargo build --workspace --release

BIN="$WORK/target/release/forgekonsole"
CORE="$WORK/target/release/libforgekonsole.so"
VERIFIER="$WORK/target/release/forgekonsole-verify"

if [[ -x "$BIN" ]]; then log "FORGEKONSOLE_DEV_RELEASE_BINARY=PASS"; else log "FORGEKONSOLE_DEV_RELEASE_BINARY=FAIL"; FAILS=$((FAILS + 1)); fi
if [[ -f "$CORE" ]]; then log "FORGEKONSOLE_DEV_LIVE_CORE_BINARY=PASS"; else log "FORGEKONSOLE_DEV_LIVE_CORE_BINARY=FAIL"; FAILS=$((FAILS + 1)); fi
if [[ -x "$VERIFIER" ]]; then log "FORGEKONSOLE_DEV_VERIFIER_BINARY=PASS"; else log "FORGEKONSOLE_DEV_VERIFIER_BINARY=FAIL"; FAILS=$((FAILS + 1)); fi

if [[ -x "$BIN" ]]; then
  run_stage DIAGNOSTIC "$BIN" --diagnostic
  for pair in \
    "RENDERER_SMOKE --renderer-smoke" \
    "PTY_CELL_SMOKE --pty-cell-smoke" \
    "CORE_PROTOCOL_SMOKE --core-protocol-smoke" \
    "DBUS_SMOKE --dbus-smoke" \
    "INTERACTION_SMOKE --interaction-smoke" \
    "WORKSPACE_SMOKE --workspace-smoke" \
    "MULTI_WINDOW_SMOKE --multi-window-smoke" \
    "COMPLETION_SMOKE --completion-smoke"
  do
    key=${pair%% *}
    arg=${pair#* }
    if command -v timeout >/dev/null 2>&1; then
      run_stage "$key" timeout 20s "$BIN" "$arg"
    else
      run_stage "$key" "$BIN" "$arg"
    fi
  done
fi

if [[ -x "$VERIFIER" ]]; then
  run_stage VERIFIER "$VERIFIER" --root "$WORK"
fi

log "FORGEKONSOLE_DEV_INSTALL_ATTEMPTED=NO"
log "FORGEKONSOLE_DEV_ACTIVATION_ATTEMPTED=NO"
log "FORGEKONSOLE_DEV_RELOAD_REQUEST_WRITTEN=NO"
log "FORGEKONSOLE_DEV_WINDOW_LAUNCH_ATTEMPTED=NO"
log "FORGEKONSOLE_DEV_FAILURE_COUNT=$FAILS"
if (( FAILS == 0 )); then
  log "FORGEKONSOLE_NEXT_DEV_GATE=PASS"
  log "FORGEKONSOLE_NEXT_RELEASE_VERSION=1.0.05"
  log "FORGEKONSOLE_NEXT_RELEASE_PACKAGING_READY=YES"
  exit 0
else
  log "FORGEKONSOLE_NEXT_DEV_GATE=FAIL"
  log "FORGEKONSOLE_NEXT_RELEASE_PACKAGING_READY=NO"
  log "NEXT_UPLOAD=$LOG"
  exit 1
fi
