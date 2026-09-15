#!/usr/bin/env python3
from pathlib import Path
root = Path(__file__).resolve().parents[1]
direct = (root/'crates/forgehx-dsp/src/direct_pipewire.rs').read_text()
runtime = (root/'crates/forgehx-dsp/src/runtime.rs').read_text()
speaker = (root/'crates/forgehx-dsp/src/speaker_lock.rs').read_text()
dsp_lib = (root/'crates/forgehx-dsp/src/lib.rs').read_text()
device = (root/'crates/forgehx-gui/src/device_page.rs').read_text()
checks = {
    'direct pipewire constant chunks use as_chunks': '.chunks_exact(SAMPLE_BYTES)' not in direct and '.as_chunks::<SAMPLE_BYTES>()' in direct,
    'runtime frame decode uses as_chunks': '.chunks_exact(4)' not in runtime and '.as_chunks::<4>()' in runtime,
    'runtime worker state grouped': 'struct RuntimeWorkerShared' in runtime and 'fn run_worker(\n    raw_source: &str,\n    processed_source: &str,\n    shared: RuntimeWorkerShared,' in runtime,
    'speaker correlation avoids indexed range loop': 'for i in 0..DOWNSAMPLED' not in speaker and 'for (i, &x) in a.iter().enumerate()' in speaker,
    'bool then uses then_some': '.then(|| ())?' not in dsp_lib and '.then_some(())?' in dsp_lib,
    'device page dpi uses slice': 'dpi_stages: &mut Vec<u16>' not in device and 'dpi_stages: &mut [u16]' in device,
}
failed = [name for name, ok in checks.items() if not ok]
if failed:
    raise SystemExit('FORGEHX_10_0_25_DSP_DEVICEPAGE_CLIPPY=FAIL ' + ','.join(failed))
print('PASS: ForgeHX 10.0.25 DSP/device-page clippy cleanup contract')
