from pathlib import Path
root=Path(__file__).resolve().parents[1]
read=lambda p:(root/p).read_text()
cargo=read('Cargo.toml'); pkg=read('PKGBUILD'); core=read('crates/forgehx-core/src/lib.rs'); daemon=read('crates/forgehx-daemon/src/lib.rs'); gui=read('crates/forgehx-gui/src/output_dsp.rs'); client=read('crates/forgehx-aetherstream/src/lib.rs'); engine=read('crates/forgehx-dsp/src/engine.rs'); rejection=read('crates/forgehx-dsp/src/background_rejection.rs')
checks={
'version':'version = "10.0.23"' in cargo and 'pkgver=10.0.23' in pkg,
'ipc12':'IPC_PROTOCOL_VERSION: u32 = 12' in core,
'output schema':(root/'crates/forgehx-core/src/output_dsp.rs').exists(),
'aetherstream client':'aetherstream/output-dsp.sock' in client,
'daemon bridge':'OutputDspClient' in daemon and 'OutputDspLiveUpdate' in daemon,
'gui':'Output DSP' in gui and 'Bass boost' in gui and 'Clarity boost' in gui,
'solocast lock':'HyperX SoloCast 2 Analog Stereo' in core,
'background rejector module':(root/'crates/forgehx-dsp/src/background_rejection.rs').exists(),
'background rejector integrated':'background_rejector: BackgroundRejector' in engine and 'background_rejector.process' in engine,
'adaptive noise floor':'noise_floor_db' in rejection and 'update_noise_floor' in rejection,
'noise strength controls rejector':'strength_percent' in rejection and 'rejection_gain_db' in rejection,
'vad threshold controls rejector':'vad_threshold_percent' in rejection and 'decision_margin_db' in rejection,
'grace hangover':'grace_ms' in rejection and 'hangover_frames' in rejection,
'10.0.20 fixture regression':(root/'scripts/test-10.0.20-pw-cli-registry-fixture.py').exists(),
'10.0.23 clippy cleanup regression':(root/'scripts/test-10.0.23-clippy-cleanup.py').exists(),
}
for forbidden in ['wpctl set-default','pactl set-default-source','pactl set-default-sink']:
    checks['no '+forbidden]=forbidden not in daemon+client+gui+engine+rejection
failed=[k for k,v in checks.items() if not v]
if failed: raise SystemExit('FORGEHX_10_0_23_SOURCE=FAIL '+','.join(failed))
print('FORGEHX_10_0_23_SOURCE=PASS')
