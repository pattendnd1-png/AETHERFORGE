#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ENGINE="$ROOT/crates/aether-engine-servo/src/live.rs"
UI="$ROOT/crates/aether-ui/src/lib.rs"
fail(){ echo "AETHER_BROWSER_V2_1_47_OMNIBOX_EDITING=FAIL:$1"; exit 1; }
grep -q 'omnibox_caret: usize' "$ENGINE" || fail caret-state-missing
grep -q 'omnibox_caret: usize' "$UI" || fail ui-caret-state-missing
grep -q 'KeyCode::ArrowLeft' "$ENGINE" || fail left-arrow-missing
grep -q 'KeyCode::ArrowRight' "$ENGINE" || fail right-arrow-missing
grep -q 'KeyCode::Home' "$ENGINE" || fail home-missing
grep -q 'KeyCode::End' "$ENGINE" || fail end-missing
grep -q 'KeyCode::Delete' "$ENGINE" || fail delete-missing
grep -q 'KeyCode::Backspace' "$ENGINE" || fail backspace-missing
grep -q 'insert_str(self.omnibox_caret' "$ENGINE" || fail insert-at-caret-missing
grep -q 'replace_range(previous..self.omnibox_caret' "$ENGINE" || fail backspace-at-caret-missing
grep -q 'replace_range(self.omnibox_caret..next' "$ENGINE" || fail delete-at-caret-missing
grep -q 'let caret_prefix = &model.display_omnibox()\[..caret\];' "$UI" || fail painted-caret-prefix-missing
grep -q 'AETHER_BROWSER_OMNIBOX_EDITING=CARET_DELETE_ARROWS' "$ENGINE" || fail runtime-marker-missing
echo 'AETHER_BROWSER_V2_1_47_OMNIBOX_EDITING=PASS'
