from pathlib import Path
text = Path('crates/forgehx-dsp/src/direct_pipewire.rs').read_text()
assert 'Audio/Source' in text, 'ForgeHX must publish one normal app-facing Audio/Source'
assert 'ForgeHX Mic' in text, 'app-facing microphone description must be ForgeHX Mic'
assert 'Direction::Output' in text, 'ForgeHX Mic must be a producer/source stream'
assert 'Audio/Sink' not in text, 'direct path must not create an intermediate sink'
assert 'node.hidden' not in text.lower(), 'app-facing ForgeHX Mic must not be hidden'
assert 'node.virtual' not in text.lower(), 'primary direct path must not create a virtual loopback chain'
print('FORGEHX_10_0_8_COMMUNICATION_SOURCE_ELIGIBILITY=PASS')
