from pathlib import Path
files=[Path('crates/forgehx-aetherstream/Cargo.toml'),Path('crates/forgehx-aetherstream/src/lib.rs')]
if not all(f.exists() for f in files): raise SystemExit('FAIL missing forgehx-aetherstream crate')
s=files[1].read_text()
need=['aetherstream/output-dsp.sock','IpcEnvelope','OutputDspRuntimeRequest','OutputDspRuntimeResponse','postcard::to_stdvec','u32::to_be_bytes','PRESERVED_AUDIO_ENDPOINT']
miss=[x for x in need if x not in s]
if miss: raise SystemExit('FAIL missing '+', '.join(miss))
print('FORGEHX_10_0_18_AETHERSTREAM_OUTPUT_CLIENT=PASS')
