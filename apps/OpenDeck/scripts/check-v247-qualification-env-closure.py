from pathlib import Path

root = Path(__file__).resolve().parents[1]
qualification = (root / 'apps/opendeck-studio/src-tauri/src/qualification.rs').read_text()
qualifier = (root / 'scripts/qualify-v247-host.sh').read_text()
startup = (root / 'apps/opendeck-studio/src-tauri/src/startup.rs').read_text()
harness = (root / 'apps/opendeck-studio/src/perf/QualificationHarness.tsx').read_text()

checks = {
    'STABLE_ENABLE_ENV': 'std::env::var("OPENDECK_QUALIFICATION")' in qualification,
    'STABLE_PHASE_ENV': 'std::env::var("OPENDECK_QUALIFICATION_PHASE")' in qualification,
    'NO_VERSIONED_QUALIFICATION_ENV_IN_RUNTIME': 'OPENDECK_V244_QUALIFICATION' not in qualification and 'OPENDECK_V245_QUALIFICATION' not in qualification and 'OPENDECK_V247_QUALIFICATION' not in qualification,
    'QUALIFIER_EXPORTS_STABLE_ENV': 'OPENDECK_QUALIFICATION=1 OPENDECK_QUALIFICATION_PHASE=visual' in qualifier,
    'QUALIFIER_DOES_NOT_EXPORT_VERSIONED_ENV': 'OPENDECK_V247_QUALIFICATION=1' not in qualifier,
    'QUALIFICATION_HIDES_NATIVE_STARTUP': 'if crate::qualification::enabled()' in startup and 'return Ok(());' in startup,
    'HARNESS_RELEASE_MATCHES': "release: '2.0.47'" in harness and "focusAck.release !== '2.0.47'" in harness,
    'QUALIFIER_VISUAL_FILE_MATCHES': 'OpenDeck-v2.0.47-VISUAL-METRICS.json' in qualifier,
}
failed = [name for name, ok in checks.items() if not ok]
if failed:
    raise SystemExit('OPENDECK_V247_QUALIFICATION_ENV_CLOSURE=FAIL:' + ','.join(failed))
for name in checks:
    print(f'OPENDECK_V247_{name}=PASS')
print('OPENDECK_V247_QUALIFICATION_ENV_CLOSURE=PASS')
