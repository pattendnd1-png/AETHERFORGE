#!/usr/bin/env bash
set -euo pipefail
main="${1:-src/main.rs}"
layout="${2:-src/layout.rs}"
fail=0
expect(){ grep -Eq "$1" "$2" || { echo "FAIL: $3" >&2; fail=1; }; }
reject(){ if grep -Eqi "$1" "$2"; then echo "FAIL: $3" >&2; fail=1; fi; }
expect 'const VERSION: &str = "0\.1\.17"' "$main" 'canonical version is not 0.1.17'
expect 'with_min_inner_size\(\[640\.0, 480\.0\]\)' "$main" 'compact window minimum missing'
expect 'ResponsiveLayout::from_width' "$main" 'responsive breakpoint selection missing'
expect 'Panel::left\("beacn-device-rail"\)' "$main" 'device/module rail missing'
expect '\.resizable\(false\)' "$main" 'fixed render-target side rails missing'
expect 'Equalizer & Enhancement' "$main" 'anchored EQ/enhancement surface missing'
expect 'Secondary Processing' "$main" 'secondary processing tabs missing'
expect 'Mic Output' "$main" 'mic output surface missing'
expect 'VOICE RECORDER' "$main" 'voice recorder surface missing'
expect 'LIVE PROFILE' "$main" 'live-profile surface missing'
expect 'enum ResponsiveLayout' "$layout" 'responsive layout model missing'
expect 'Compact' "$layout" 'compact layout missing'
expect 'Standard' "$layout" 'standard layout missing'
expect 'Wide' "$layout" 'wide layout missing'
reject 'stream[ _-]?deck|twitch|oauth' "$main" 'Stream Deck/Twitch code remains in BEACN app'
exit "$fail"
