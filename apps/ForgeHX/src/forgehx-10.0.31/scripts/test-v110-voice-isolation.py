from pathlib import Path
root=Path(__file__).resolve().parents[1]
core=(root/'crates/forgehx-core/src/lib.rs').read_text()
dsp=(root/'crates/forgehx-dsp/src/lib.rs').read_text()
engine=(root/'crates/forgehx-dsp/src/engine.rs').read_text()
runtime=(root/'crates/forgehx-dsp/src/runtime.rs').read_text()
daemon=(root/'crates/forgehx-daemon/src/lib.rs').read_text()
gui=(root/'crates/forgehx-gui/src/microphone.rs').read_text()
app=(root/'crates/forgehx-gui/src/app.rs').read_text()
settings=(root/'crates/forgehx-gui/src/settings.rs').read_text()
checks={
 'SpeakerLockConfig': 'struct SpeakerLockConfig' in core,
 'PlaybackRejectionConfig': 'struct PlaybackRejectionConfig' in core,
 'VoiceIsolationTelemetry': 'struct VoiceIsolationTelemetry' in core,
 'enroll command': 'MicVoiceEnrollStart' in core and 'MicVoiceEnrollCancel' in core and 'MicVoiceForget' in core,
 'speaker verifier module': 'pub mod speaker_lock;' in dsp,
 'speaker gate in engine': 'speaker_verifier' in engine and 'playback_guard' in engine,
 'runtime enrollment': 'start_enrollment' in runtime and 'cancel_enrollment' in runtime and 'forget_voice' in runtime,
 'daemon route': 'mic_voice_enroll_start' in daemon and 'mic_voice_forget' in daemon,
 'GUI identity controls': 'Continuous voice learning' in gui and 'Reset Learned Voice' in gui and 'Only pass my enrolled voice' in gui,
 'all-page scroll': 'FORGEHX_ALL_PAGE_SCROLL_HOST' in app and 'ScrollArea::vertical()' in app,
 'editable settings': 'UiPreferences' in settings and 'Automatic inventory refresh' in settings and 'Refresh immediately after setting changes' in settings,
}
failed=[name for name,ok in checks.items() if not ok]
if failed:
    raise SystemExit('FAIL: '+', '.join(failed))
print('PASS: ForgeHX voice isolation/settings/scrolling source contract')
