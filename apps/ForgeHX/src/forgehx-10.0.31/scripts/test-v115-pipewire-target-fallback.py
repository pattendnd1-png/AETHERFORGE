from pathlib import Path
root=Path(__file__).resolve().parents[1]
audio=(root/'crates/forgehx-audio/src/lib.rs').read_text()
dsp=(root/'crates/forgehx-dsp/src/lib.rs').read_text()
required=[
 ('audio serial parser','object.serial' in audio and 'parse_pipewire_target' in audio),
 ('dsp serial parser','object.serial' in dsp and 'parse_pipewire_target' in dsp),
 ('audio no node-name-only failure','has no stable node.name' not in audio),
 ('dsp no node-name-only failure','has no stable node.name' not in dsp),
 ('audio stable target failure','has neither node.name nor object.serial' in audio),
 ('dsp stable target failure','has neither node.name nor object.serial' in dsp),
]
failed=[name for name,ok in required if not ok]
if failed:
    raise SystemExit('FAIL: '+', '.join(failed))
print('PASS: ForgeHX 10.0.6 PipeWire target fallback contract')
