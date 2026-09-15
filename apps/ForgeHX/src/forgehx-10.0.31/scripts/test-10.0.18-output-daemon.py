from pathlib import Path
s=Path('crates/forgehx-daemon/src/lib.rs').read_text()
need=['OutputDspClient','output_dsp_profile','output_dsp_state_path','Command::OutputDspGet','Command::OutputDspLiveUpdate','Command::OutputDspResetFactory','DEFAULT_OUTPUT_DSP_TARGET','serde_json::to_vec_pretty']
miss=[x for x in need if x not in s]
for forbidden in ['wpctl set-default','pactl set-default-source','pactl set-default-sink']:
    if forbidden in s: raise SystemExit('FAIL routing mutation '+forbidden)
if miss: raise SystemExit('FAIL missing '+', '.join(miss))
print('FORGEHX_10_0_18_OUTPUT_DAEMON=PASS')
