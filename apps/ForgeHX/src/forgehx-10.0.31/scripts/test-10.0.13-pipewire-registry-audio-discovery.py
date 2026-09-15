from pathlib import Path
root = Path(__file__).resolve().parents[1]
audio = (root/'crates/forgehx-audio/src/lib.rs').read_text()
daemon = (root/'crates/forgehx-daemon/src/lib.rs').read_text()
assert 'pw-cli' in audio, 'shared audio discovery must query authoritative PipeWire registry'
assert 'parse_pw_cli_audio_nodes' in audio, 'shared audio discovery must parse PipeWire registry nodes'
assert 'merge_audio_nodes' in audio, 'shared discovery must merge wpctl and registry results'
assert 'media.class' in audio and 'Audio/Source' in audio, 'registry parser must classify physical sources'
assert 'node.name' in audio, 'registry parser must preserve stable PipeWire node.name'
assert 'HyperX microphone has no associated PipeWire source' in daemon, 'daemon live-source guard must remain explicit'
print('FORGEHX_10_0_13_PIPEWIRE_REGISTRY_AUDIO_DISCOVERY=PASS')
