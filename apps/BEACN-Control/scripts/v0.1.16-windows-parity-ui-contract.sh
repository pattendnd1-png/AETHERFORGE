#!/usr/bin/env bash
set -euo pipefail
main="${1:-src/main.rs}"
fail=0
expect(){ grep -Eq "$1" "$main" || { echo "FAIL: $2" >&2; fail=1; }; }
expect 'fn beacn_header\(' 'BEACN header missing'
expect 'fn live_profile_bar\(' 'Live Profiles bar missing'
expect 'fn beacn_device_rail\(' 'device/module rail missing'
expect 'fn mic_chain_canvas\(' 'mic chain canvas missing'
expect 'fn plugin_tabs\(' 'secondary processor tabs missing'
expect 'fn voice_recorder_strip\(' 'voice recorder strip missing'
expect 'fn right_device_panel\(' 'right device/preset panel missing'
expect 'fn processor_cards\(' 'processor card row missing'
expect 'fn bottom_capability_strip\(' 'bottom capability strip missing'
expect 'fn apply_quick_preset\(' 'quick preset action missing'
expect 'Real-Time Meters' 'real-time meter bank missing'
expect 'Quick Presets' 'quick presets surface missing'
expect 'Monitor Mix' 'monitor mix surface missing'
expect 'FULL WINDOWS 1.4 PARITY' 'parity capability strip missing'
expect 'DRAGONGLASS UI' 'DragonGlass capability strip missing'

expect 'fn mic_output_meter\(' 'Mic Output end-of-chain surface missing'
expect 'EQUALIZER & ENHANCEMENT' 'anchored Equalizer & Enhancement missing'
expect 'SECONDARY PROCESSING' 'secondary processing missing'
expect 'De-Esser' 'De-Esser workflow missing'
expect 'Exciter' 'Exciter workflow missing'
expect '10-BAND PER-EAR EQ' 'Enhanced Headphones 10-band workflow missing'
expect 'Binaural Personalization' 'binaural personalization workflow missing'
expect 'Left / Right Balance' 'headphone balance workflow missing'
expect 'Mono Mode' 'headphone mono workflow missing'
expect 'SNAPSHOT' 'Live Profile snapshot control missing'
expect 'AUTO-SAVE' 'Live Profile autosave indicator missing'
expect 'SYSTEM DSP ISOLATED' 'system-DSP isolation badge missing'
expect 'RAW MIC PRESERVED' 'raw-mic preservation badge missing'
expect 'START PRIVATE DSP' 'private-DSP start control missing'
expect 'AetherForge BEACN Processed' 'processed-source workflow missing'
exit "$fail"
