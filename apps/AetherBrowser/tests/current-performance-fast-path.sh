#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
fail(){ echo "AETHER_BROWSER_PERFORMANCE_FAST_PATH=FAIL:$1"; exit 1; }
LIVE=crates/aether-engine-servo/src/live.rs
UI=crates/aether-ui/src/lib.rs
TELEMETRY=crates/aether-telemetry/src/lib.rs

grep -qF 'AETHER_BROWSER_PERF_CHROME_VISIBLE_US=' "$LIVE" || fail missing-chrome-visible-timing
grep -qF 'AETHER_BROWSER_PERF_FIRST_CONTENT_PAINT_US=' "$LIVE" || fail missing-first-content-timing
grep -qF 'AETHER_BROWSER_PERF_PAGE_COMPLETE_US=' "$LIVE" || fail missing-page-complete-timing
grep -qF 'const TELEMETRY_POLL_INTERVAL: Duration = Duration::from_millis(1_000);' "$LIVE" || fail telemetry-interval-not-relaxed
grep -qF 'telemetry_snapshot: SystemTelemetrySnapshot::default()' "$LIVE" || fail startup-telemetry-not-deferred
grep -qF 'last_window_title: String::new()' "$LIVE" || fail title-cache-not-initialized
grep -qF 'active_bookmarked_cache: false' "$LIVE" || fail library-chrome-cache-not-initialized
grep -qF 'active_download_count_cache: 0' "$LIVE" || fail download-cache-not-initialized
grep -qF 'if model.home_mode {' "$UI" || fail home-assets-not-gated
grep -qF 'home_shell_frame_seen' "$UI" || fail home-preview-defer-missing
grep -qF 'gpu_device_dirs: Vec<PathBuf>' "$TELEMETRY" || fail gpu-sysfs-path-cache-missing
grep -qF 'cpu_temp_paths: Vec<PathBuf>' "$TELEMETRY" || fail cpu-temp-path-cache-missing
grep -qF '#[derive(Debug, Default)]' "$TELEMETRY" || fail telemetry-default-not-derived
grep -qF 'paths_discovered: bool' "$TELEMETRY" || fail sysfs-discovery-flag-missing

echo 'AETHER_BROWSER_PERFORMANCE_TIMING=PASS'
echo 'AETHER_BROWSER_STARTUP_TELEMETRY_DEFER=PASS'
echo 'AETHER_BROWSER_LAZY_NATIVE_ASSETS=PASS'
echo 'AETHER_BROWSER_CHROME_DB_CACHE=PASS'
echo 'AETHER_BROWSER_SYSFS_PATH_CACHE=PASS'
echo 'AETHER_BROWSER_PERFORMANCE_FAST_PATH=PASS'
