#!/usr/bin/env python3
from pathlib import Path
src = Path('crates/forgehx-audio/src/lib.rs').read_text()
start = src.index('fn registry_parser_recovers_physical_source_missing_from_wpctl_view()')
end = src.index('fn wpctl_entry_wins_when_registry_has_same_node_id()', start)
fixture = src[start:end]
errors=[]
for required in [
    'node.name = "alsa_input.usb-HP__Inc_HyperX_SoloCast_2_SERIAL-00.analog-stereo"',
    'node.description = "HyperX SoloCast 2 Analog Stereo"',
    'media.class = "Audio/Source"',
    'node.name = "forgehx_mic_a20cca526b23023a"',
    'assert_eq!(nodes.len(), 2);',
]:
    if required not in fixture:
        errors.append('missing canonical fixture token: '+required)
if r'\"Audio/Source\"' in fixture or r'\"alsa_input.' in fixture:
    errors.append('raw-string fixture still contains backslash-escaped quotes')
# Production parser must stay strict; do not normalize malformed escaped fixture data.
parser = src[src.index('pub fn parse_pw_cli_audio_nodes'):src.index('pub fn merge_audio_nodes')]
for strict in ['Some("Audio/Source") => "source"','Some("Audio/Sink") => "sink"']:
    if strict not in parser:
        errors.append('production media.class strict match changed: '+strict)
if errors:
    raise SystemExit('FORGEHX_10_0_20_PWCLI_REGISTRY_FIXTURE=FAIL ' + '; '.join(errors))
print('FORGEHX_10_0_20_PWCLI_REGISTRY_FIXTURE=PASS')
