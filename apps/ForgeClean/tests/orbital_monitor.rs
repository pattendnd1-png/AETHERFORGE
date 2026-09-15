use forgeclean::orbital_monitor_model::{SyncProgress, parse_diagnostic, parse_proc_net_dev};

#[test]
fn proc_net_parser_sums_non_loopback_interfaces() {
    let text = "Inter-| Receive | Transmit\n lo: 100 0 0 0 0 0 0 0 200 0 0 0 0 0 0 0\n enp1s0: 1000 0 0 0 0 0 0 0 2500 0 0 0 0 0 0 0\n wlan0: 400 0 0 0 0 0 0 0 700 0 0 0 0 0 0 0\n";
    let totals = parse_proc_net_dev(text);
    assert_eq!(totals.rx_bytes, 1400);
    assert_eq!(totals.tx_bytes, 3200);
}

#[test]
fn diagnostic_parser_tracks_phase_project_and_upload_progress() {
    let text = "PHASE=SOURCE_IMPORT\nSOURCE_IMPORT_START=ForgeClean\nSOURCE_IMPORT_DONE=ForgeClean\nPHASE=ARTIFACT_UPLOAD_AFTER_COMMIT\nARTIFACT_UPLOAD_QUEUE=12\nARTIFACT_UPLOAD_PROGRESS=7/12:ForgeClean-v1.0.4-SOURCE.zip\n";
    let progress = parse_diagnostic(text, true);
    assert_eq!(progress.phase, "ARTIFACT_UPLOAD_AFTER_COMMIT");
    assert_eq!(progress.project, "ForgeClean");
    assert_eq!(progress.current_file, "ForgeClean-v1.0.4-SOURCE.zip");
    assert_eq!(progress.completed, 7);
    assert_eq!(progress.total, 12);
    assert!(progress.active);
}

#[test]
fn completed_diagnostic_is_not_active() {
    let text = "PHASE=PROFILE_SYNC\nAETHERFORGE_GITHUB_MIGRATION_SWEEP=PASS\n";
    let progress = parse_diagnostic(text, false);
    assert_eq!(
        progress,
        SyncProgress {
            phase: "COMPLETE".into(),
            project: String::new(),
            current_file: String::new(),
            completed: 0,
            total: 0,
            active: false
        }
    );
}
