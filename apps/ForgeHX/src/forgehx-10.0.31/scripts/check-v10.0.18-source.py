from pathlib import Path
root=Path(__file__).resolve().parents[1]
read=lambda p:(root/p).read_text()
cargo=read('Cargo.toml'); pkg=read('PKGBUILD'); core=read('crates/forgehx-core/src/lib.rs'); daemon=read('crates/forgehx-daemon/src/lib.rs'); gui=read('crates/forgehx-gui/src/output_dsp.rs'); client=read('crates/forgehx-aetherstream/src/lib.rs')
checks={
'version':'version = "10.0.18"' in cargo and 'pkgver=10.0.18' in pkg,
'ipc12':'IPC_PROTOCOL_VERSION: u32 = 12' in core,
'output schema':(root/'crates/forgehx-core/src/output_dsp.rs').exists(),
'aetherstream client':'aetherstream/output-dsp.sock' in client,
'daemon bridge':'OutputDspClient' in daemon and 'OutputDspLiveUpdate' in daemon,
'gui':'Output DSP' in gui and 'Bass boost' in gui and 'Clarity boost' in gui,
'solocast lock':'HyperX SoloCast 2 Analog Stereo' in core,
'10.0.17 bridge preserved':(root/'scripts/test-10.0.17-aetherstream-bridge.py').exists(),
}
for forbidden in ['wpctl set-default','pactl set-default-source','pactl set-default-sink']:
    checks['no '+forbidden]=forbidden not in daemon+client+gui
failed=[k for k,v in checks.items() if not v]
if failed: raise SystemExit('FORGEHX_10_0_18_SOURCE=FAIL '+','.join(failed))
print('FORGEHX_10_0_18_SOURCE=PASS')
