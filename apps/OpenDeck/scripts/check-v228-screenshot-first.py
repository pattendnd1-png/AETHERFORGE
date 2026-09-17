from pathlib import Path

runner = Path('scripts/qualify-v228-host.sh')
if not runner.exists():
    raise SystemExit('QUALIFIER_ORDER=FAIL:qualify-v228-host.sh missing')
text = runner.read_text()
required = [
    'gate "PREVIEW_TAURI_BUILD"',
    'say "OPENDECK_V228_PREVIEW_SCREENSHOT=START"',
    'capture-v228-window.sh',
    'say "OPENDECK_V228_PREVIEW_SCREENSHOT=PASS:',
    'gate "CARGO_FMT"',
    'gate "CARGO_CLIPPY_STRICT"',
]
for token in required:
    if token not in text:
        raise SystemExit(f'QUALIFIER_ORDER=FAIL:missing {token}')
pos = {token: text.index(token) for token in required}
if not (pos['gate "PREVIEW_TAURI_BUILD"'] < pos['say "OPENDECK_V228_PREVIEW_SCREENSHOT=START"'] < pos['capture-v228-window.sh'] < pos['say "OPENDECK_V228_PREVIEW_SCREENSHOT=PASS:'] < pos['gate "CARGO_FMT"'] < pos['gate "CARGO_CLIPPY_STRICT"']):
    raise SystemExit('QUALIFIER_ORDER=FAIL:preview screenshot is not before later Rust gates')

# The preview must survive later failures without launching a second visual window.
for token in [
    'OPENDECK_V228_PREVIEW_SCREENSHOT_AVAILABLE=$SCREENSHOT',
    'QUALIFIED_VISUAL_JSON="$QDIR/OpenDeck-v2.0.28-VISUAL-METRICS-QUALIFIED.json"',
    'cp -f "$VISUAL_JSON" "$QUALIFIED_VISUAL_JSON"',
    'check-v228-visual-metrics.py" "$QUALIFIED_VISUAL_JSON"',
]:
    if token not in text:
        raise SystemExit(f'QUALIFIER_ORDER=FAIL:missing persistence safeguard {token}')
if 'QUALIFICATION-FINAL.png' in text or 'mv -f "$FINAL_SCREENSHOT" "$SCREENSHOT"' in text:
    raise SystemExit('QUALIFIER_ORDER=FAIL:second visual recapture reintroduced')
preview_pass = text.index('say \"OPENDECK_V228_PREVIEW_SCREENSHOT=PASS:')
if 'rm -f \"$VISUAL_JSON\" \"$SCREENSHOT\"' in text[preview_pass:]:
    raise SystemExit('QUALIFIER_ORDER=FAIL:later stage deletes preview screenshot')

print('OPENDECK_V228_SCREENSHOT_FIRST_CONTRACT=PASS')
