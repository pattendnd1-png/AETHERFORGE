from pathlib import Path
import sys

root = Path(__file__).resolve().parents[1]
app = (root/'crates/forgehx-gui/src/app.rs').read_text()
settings = (root/'crates/forgehx-gui/src/settings.rs').read_text()
mic = (root/'crates/forgehx-gui/src/microphone.rs').read_text()
runtime = (root/'crates/forgehx-dsp/src/runtime.rs').read_text()
service = (root/'packaging/systemd/forgehx-daemon.service').read_text()

checks = {
    'refresh_after_changes preference': 'refresh_after_changes' in settings,
    'one-second default fallback': 'inventory_refresh_seconds: 1' in settings,
    'immediate refresh helper': 'fn refresh_after_update(&mut self)' in app,
    'device writes refresh inventory': 'self.refresh_after_update();' in app,
    'mic writes refresh inventory': 'handle_mic_dsp_reply' in app and 'self.refresh_after_update();' in app,
    'explicit mic reattach button': 'Reattach DSP now' in mic and 'DSP permanently enabled' in mic,
    'background daemon restart always': 'Restart=always' in service,
    'lifetime fix clones voiceprint handle': 'Arc::clone(&handle.completed_voiceprint)' in runtime,
    'lifetime fix stores terminal value': 'let voiceprint = completed_voiceprint.lock().ok()?.take();' in runtime,
}
failed = [name for name, ok in checks.items() if not ok]
if failed:
    print('FAIL: ' + ', '.join(failed))
    sys.exit(1)
print('PASS: ForgeHX 10.0.6 background/refresh/user-control contract')
