from pathlib import Path
root=Path(__file__).resolve().parents[1]
core=(root/'crates/forgehx-core/src/lib.rs').read_text(); client=(root/'crates/forgehx-aetherstream/src/lib.rs').read_text(); daemon=(root/'crates/forgehx-daemon/src/lib.rs').read_text(); gui=(root/'crates/forgehx-gui/src/output_dsp.rs').read_text()
assert 'PRESERVED_AUDIO_ENDPOINT: &str = "HyperX SoloCast 2 Analog Stereo"' in core
assert 'PRESERVED_AUDIO_ENDPOINT' in client and 'PRESERVED_AUDIO_ENDPOINT' in gui
joined='\n'.join([client,daemon,gui])
for token in ['set-default','set_default','pactl','wpctl']:
    if token in joined: raise SystemExit('FORGEHX_SOLOCAST_ROUTING_LOCK=FAIL token='+token)
print('FORGEHX_SOLOCAST_ROUTING_LOCK=PASS endpoint=HyperX SoloCast 2 Analog Stereo')
