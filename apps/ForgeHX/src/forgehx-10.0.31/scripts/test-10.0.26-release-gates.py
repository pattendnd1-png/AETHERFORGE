from pathlib import Path
root=Path(__file__).resolve().parents[1]
verify=(root/'scripts/verify-10.0.26.sh').read_text() if (root/'scripts/verify-10.0.26.sh').exists() else ''
checks={
 'clippy diagnostic only':'FORGEHX_CLIPPY=INFO:FAIL:' in verify and 'cargo clippy --workspace --all-targets' in verify,
 'tests hard gate':'run TEST' in verify,
 'release build hard gate':'run BUILD' in verify,
 'live rejection hard gate':'run LIVE_NOISE_REJECTION' in verify,
 'live rejection supersedes old background implementation gate':'test-10.0.21-background-rejection.py' not in verify,
 'routing lock hard gate':'run SOLOCAST_ROUTING_LOCK' in verify,
 'aetherstream bridge hard gate':'run MIC_AETHERSTREAM_BRIDGE' in verify,
 'output socket hard gate':'FORGEHX_AETHERSTREAM_OUTPUT_DSP_SOCKET=FAIL:' in verify and 'fail=1' in verify,
}
failed=[k for k,v in checks.items() if not v]
if failed: raise SystemExit('FORGEHX_10_0_26_RELEASE_GATES=FAIL '+', '.join(failed))
print('FORGEHX_10_0_26_RELEASE_GATES=PASS')
