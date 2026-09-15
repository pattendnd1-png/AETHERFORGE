from pathlib import Path
import sys
mic=Path('crates/forgehx-gui/src/microphone.rs').read_text()
core=Path('crates/forgehx-core/src/lib.rs').read_text()
daemon=Path('crates/forgehx-daemon/src/lib.rs').read_text()
app=Path('crates/forgehx-gui/src/app.rs').read_text()
required_ui=[
 'Automatic Voice Processing','Broadcast Full','Adaptation strength','Echo Cancellation','Noise Reduction',
 'Bass Boost','Treble Boost','Warmth','Body / Fullness','Clarity','Presence','Air','Advanced DSP',
 'Multiband EQ','Dynamic EQ','Multiband Compressor','Parallel mix','Saturation','True-peak protection',
 'AUTO','MANUAL','LOCKED','BYPASSED','VoicePilot','Continuous voice learning','Reset Learned Voice','Only pass my enrolled voice','Hard-block playback matches / loops',
]
missing=[x for x in required_ui if x not in mic]
if missing:
 print('MISSING_UI='+';'.join(missing));sys.exit(1)
for marker in ['MicDspLiveUpdate', 'MicVoiceEnrollStart', 'IPC_PROTOCOL_VERSION: u32 = 11']:
 if marker not in core:
  print('MISSING_CORE='+marker);sys.exit(1)
if 'MicDspLiveUpdate' not in daemon or 'MicDspLiveUpdate' not in app:
 print('MISSING_LIVE_UPDATE_PATH');sys.exit(1)
if 'Audio/Sink' in mic or 'EasyEffects' in mic or 'JamesDSP' in mic:
 print('PLAYBACK_DSP_EXPOSED');sys.exit(1)
print('ForgeHX 10.0.6 VoicePilot microphone GUI/live-update invariants passed.')
