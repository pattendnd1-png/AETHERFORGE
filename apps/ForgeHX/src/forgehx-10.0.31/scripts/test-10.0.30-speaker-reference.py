from pathlib import Path
import sys
root=Path(sys.argv[1])
s=(root/'crates/forgehx-dsp/src/runtime.rs').read_text()
checks={
 'no_sink_as_capture_source':'spawn_record("@DEFAULT_AUDIO_SINK@")' not in s,
 'default_sink_resolved_read_only':'get-default-sink' in s,
 'monitor_source_target':'.monitor' in s,
 'parec_reference_capture':'Command::new("parec")' in s,
 'float32_reference':'float32le' in s,
 'reference_failure_visible':'ForgeHX AEC reference' in s and ('warn!' in s or 'eprintln!' in s),
}
for k,v in checks.items(): print(f'{k}={"PASS" if v else "FAIL"}')
if not all(checks.values()): raise SystemExit(1)
print('FORGEHX_10_0_30_SPEAKER_REFERENCE_CONTRACT=PASS')
