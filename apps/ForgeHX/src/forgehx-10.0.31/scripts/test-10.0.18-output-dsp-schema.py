from pathlib import Path
p=Path('crates/forgehx-core/src/lib.rs').read_text()+Path('crates/forgehx-core/src/output_dsp.rs').read_text()
need=[
'pub const IPC_PROTOCOL_VERSION: u32 = 12;',
'pub enum OutputDeviceClass', 'pub enum OutputFilterKind', 'pub struct OutputFilter',
'pub struct OutputEnhancement', 'pub struct OutputLimiter', 'pub struct OutputGamingMode',
'pub struct OutputDspProfile', 'pub const ALL: [Self; 19]',
'OutputDspGet', 'OutputDspLiveUpdate', 'OutputDspResetFactory', 'OutputDspState',
'HyperX SoloCast 2 Analog Stereo'
]
missing=[x for x in need if x not in p]
if missing:
    raise SystemExit('FAIL missing: '+', '.join(missing))
print('FORGEHX_10_0_18_OUTPUT_DSP_SCHEMA=PASS')
