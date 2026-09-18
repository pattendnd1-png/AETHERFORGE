from pathlib import Path
root = Path(__file__).resolve().parents[1]
p = root / 'apps/opendeck-studio/src-tauri/src/streamdeck/render.rs'
s = p.read_text()
need = [
    'hardware_key_renderer_draws_an_icon_even_without_an_explicit_asset',
    'visible_upper_pixels',
    'delta > 45',
    'icon pixels with visible contrast above the title',
]
for item in need:
    if item not in s:
        raise SystemExit(f'OPENDECK_V254_RENDER_TEST_CLOSURE=FAIL:missing:{item}')
for forbidden in ['pixel.0[0] > 150 && pixel.0[1] > 150 && pixel.0[2] > 150', 'bright_upper_pixels']:
    if forbidden in s:
        raise SystemExit(f'OPENDECK_V254_RENDER_TEST_CLOSURE=FAIL:stale:{forbidden}')
print('OPENDECK_V254_RENDER_TEST_CLOSURE=PASS')
