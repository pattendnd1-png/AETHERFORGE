from pathlib import Path

runner = Path('scripts/qualify-v222-host.sh')
if not runner.exists():
    raise SystemExit('QUALIFIER_ORDER=FAIL:qualify-v222-host.sh missing')
text = runner.read_text()
required = [
    'gate "PREVIEW_TAURI_BUILD"',
    'say "OPENDECK_V222_PREVIEW_SCREENSHOT=START"',
    'capture-v222-window.sh',
    'say "OPENDECK_V222_PREVIEW_SCREENSHOT=PASS:',
    'gate "CARGO_FMT"',
    'gate "CARGO_CLIPPY_STRICT"',
]
for token in required:
    if token not in text:
        raise SystemExit(f'QUALIFIER_ORDER=FAIL:missing {token}')
pos = {token: text.index(token) for token in required}
if not (pos['gate "PREVIEW_TAURI_BUILD"'] < pos['say "OPENDECK_V222_PREVIEW_SCREENSHOT=START"'] < pos['capture-v222-window.sh'] < pos['say "OPENDECK_V222_PREVIEW_SCREENSHOT=PASS:'] < pos['gate "CARGO_FMT"'] < pos['gate "CARGO_CLIPPY_STRICT"']):
    raise SystemExit('QUALIFIER_ORDER=FAIL:preview screenshot is not before later Rust gates')

# The preview must survive later failures and final recapture attempts.
for token in [
    'OPENDECK_V222_PREVIEW_SCREENSHOT_AVAILABLE=$SCREENSHOT',
    'FINAL_SCREENSHOT="$QDIR/OpenDeck-v2.0.22-QUALIFICATION-FINAL.png"',
    'mv -f "$FINAL_SCREENSHOT" "$SCREENSHOT"',
]:
    if token not in text:
        raise SystemExit(f'QUALIFIER_ORDER=FAIL:missing persistence safeguard {token}')
preview_pass = text.index('say \"OPENDECK_V222_PREVIEW_SCREENSHOT=PASS:')
if 'rm -f \"$VISUAL_JSON\" \"$SCREENSHOT\"' in text[preview_pass:]:
    raise SystemExit('QUALIFIER_ORDER=FAIL:later stage deletes preview screenshot')

print('OPENDECK_V222_SCREENSHOT_FIRST_CONTRACT=PASS')
