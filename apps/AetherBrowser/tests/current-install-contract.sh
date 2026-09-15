#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_CURRENT_INSTALL=FAIL:$1"; exit 1; }
for f in scripts/install-host.sh scripts/install-current-tree.sh; do
  grep -qF "VERSION='2.1.60'" "$f" || fail "wrong-version:$f"
done
grep -qF 'AETHER_BROWSER_VERIFY_PHASE=preinstall' scripts/install-host.sh || fail no-preinstall-hard-verify-phase
grep -qF 'AETHER_BROWSER_PREINSTALL_VISUAL_PROBES=DEFERRED' scripts/install-host.sh || fail no-visual-defer-marker
grep -qF 'systemctl --user restart aether-browser-media.service' scripts/install-current-tree.sh || fail no-service-restart
grep -qF '/usr/bin/aether-browser --status' scripts/install-current-tree.sh || fail no-current-status-smoke
grep -qF 'AETHER_BROWSER_INSTALLED_EXTERNAL_WEB_FRAME_PROBE=PASS' scripts/install-current-tree.sh || fail no-installed-external-web-probe
grep -qF 'https://example.com/' scripts/install-current-tree.sh || fail no-installed-external-web-url
grep -qF 'AETHER_BROWSER_INSTALLED_VISIBLE_TEST_SCREEN=PASS' scripts/install-current-tree.sh || fail no-installed-visible-test-screen
grep -qF 'AETHER_BROWSER_INSTALLED_EXTERNAL_WEB_CHILD_VISIBLE=PASS' scripts/install-current-tree.sh || fail no-installed-external-child-visible
grep -qF 'shutdown_browser_processes' scripts/install-current-tree.sh || fail no-stale-browser-shutdown
grep -qF 'shutdown_browser_processes' scripts/install-host.sh || fail no-preverify-stale-browser-shutdown
grep -qF 'AETHER_BROWSER_INSTALLED_VERSION_IDENTITY=PASS' scripts/install-current-tree.sh || fail no-installed-version-identity
grep -qF 'AETHER_BROWSER_STAGE_PACKAGE_INTEGRITY=PASS' scripts/install-current-tree.sh || fail no-stage-integrity
grep -qF 'AETHER_BROWSER_ATOMIC_PROMOTION=PASS:' scripts/install-current-tree.sh || fail no-atomic-promotion
grep -qF "record 'AETHER_BROWSER_POSTINSTALL_VISUAL=FAIL'" scripts/install-current-tree.sh || fail no-postinstall-visual-hard-fail

grep -qF "TAG='V2_1_60'" scripts/verify.sh || fail verify-tag
grep -qF "TAG='V2_1_60'" scripts/install-host.sh || fail install-tag
grep -qF 'tee -a "$VERIFY_FILE"' scripts/verify.sh || fail no-live-verify-output
grep -qF 'AETHER_BROWSER_INSTALL_LOCK=' scripts/install-host.sh || fail no-install-lock
grep -qF 'bash scripts/install-current-tree.sh 2>&1 | tee -a "$BOOTSTRAP_VERIFY"' scripts/install-host.sh || fail native-install-not-streamed
grep -qF 'flock -n 9' scripts/install-host.sh || fail no-install-flock
echo 'AETHER_BROWSER_CURRENT_INSTALL=PASS'
