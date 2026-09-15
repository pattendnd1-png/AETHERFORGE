from pathlib import Path
import sys
root = Path(__file__).resolve().parents[1]
daemon = (root/'crates/forgehx-daemon/src/lib.rs').read_text()
dsp = (root/'crates/forgehx-dsp/src/lib.rs').read_text()
checks = {
    'daemon restore accepts discovered audio nodes': 'fn restore_always_on_dsp(&self, devices: &[DeviceInfo], audio_nodes: &[AudioNode])' in daemon,
    'refresh passes discovered audio nodes into restore': 'self.restore_always_on_dsp(&all_devices, &audio_nodes);' in daemon,
    'restore uses stable discovered node name': 'ensure_always_on_with_source_name(&device.id, &node.name)' in daemon,
    'manager exposes stable-name always-on API': 'pub fn ensure_always_on_with_source_name' in dsp,
    'manual apply resolves discovered target name': 'fn live_microphone_source_target(&self, id: &DeviceId) -> Result<String, ForgeHxError>' in daemon,
    'manual apply uses stable-name manager API': 'self.mic_dsp.apply_with_source_name(id, name, &raw_source_target)' in daemon,
    'old manual apply node-id roundtrip removed': 'self.mic_dsp.apply(id, name, node_id)' not in daemon,
}
failed=[name for name, ok in checks.items() if not ok]
for name, ok in checks.items(): print(('PASS' if ok else 'FAIL')+': '+name)
if failed:
    print('FORGEHX_10_0_27_ALWAYS_ON_DSP_ACTIVATION=FAIL')
    sys.exit(1)
print('FORGEHX_10_0_27_ALWAYS_ON_DSP_ACTIVATION=PASS')
