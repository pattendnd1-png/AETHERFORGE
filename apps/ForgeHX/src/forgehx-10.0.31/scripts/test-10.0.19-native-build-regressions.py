from pathlib import Path
cli = Path('crates/forgehx-cli/src/main.rs').read_text()
page = Path('crates/forgehx-gui/src/device_page.rs').read_text()
app = Path('crates/forgehx-gui/src/app.rs').read_text()
errors=[]
if 'Reply::OutputDspState { profile, live, generation, target_device_id }' not in cli:
    errors.append('CLI OutputDspState formatter arm missing')
for variant in ['MicDspBypass','MicVoiceEnrollStart','MicVoiceEnrollCancel']:
    if f'{variant}(DeviceId)' in page:
        errors.append(f'stale GUI DeviceAction variant {variant} remains')
    if f'DeviceAction::{variant}' in app:
        errors.append(f'stale GUI dispatcher arm {variant} remains')
if errors:
    raise SystemExit('FAIL: ' + '; '.join(errors))
print('FORGEHX_10_0_19_NATIVE_BUILD_REGRESSIONS=PASS')
