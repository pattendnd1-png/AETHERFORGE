#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_V2_1_56_LOOP_BREAK_CLOSURE=FAIL:$1"; exit 1; }
VERIFY=scripts/verify.sh
PACKAGE=scripts/package-release.sh

# Verification must inspect the source as packaged, never mutate it into compliance first.
! grep -qF 'AETHER_BROWSER_FMT_NORMALIZE' "$VERIFY" || fail verify-still-self-formats
grep -qF 'cargo fmt --all -- --check' "$VERIFY" || fail verify-no-format-check
grep -qF 'gate AETHER_BROWSER_TEST_FULL cargo test --locked --workspace --all-features --no-fail-fast' "$VERIFY" || fail verify-no-full-workspace-test

# Heavy build/package scratch space must live on persistent user storage, not a small /tmp tmpfs.
for scratch_script in scripts/package-consolidated.sh scripts/install-current-tree.sh; do
  grep -qF 'AETHER_BROWSER_TMP_ROOT' "$scratch_script" || fail "persistent-tmp-root-missing:$scratch_script"
done
! grep -qF 'mktemp "${TMPDIR:-/tmp}' scripts/package-consolidated.sh || fail consolidated-heavy-tmp-still-system-tmp
! grep -qF 'mktemp -d "${TMPDIR:-/tmp}' scripts/install-current-tree.sh || fail install-heavy-tmp-still-system-tmp

# No hand-maintained test allowlist: every current contract must be exercised automatically.
grep -qF "find \"\$ROOT/tests\" -maxdepth 1 -type f -name 'current-*.sh'" "$VERIFY" || fail verify-not-discovering-full-current-suite
grep -qF "find \"\$ROOT/tests\" -maxdepth 1 -type f -name 'current-*.sh'" "$PACKAGE" || fail package-not-discovering-full-current-suite
! grep -qF 'allowed_tests={' tests/current-clean-break.sh || fail clean-break-still-hardcodes-test-manifest

# These old contracts were superseded by the current Chromium/installed-runtime architecture.
for retired in \
  current-custom-resolution-runtime.sh \
  current-endpoint-click-execution.sh \
  current-full-height-web.sh \
  current-live-frame-contract.sh \
  current-media-diagnostics.sh \
  current-native-content-probe-readiness.sh \
  current-no-flicker.sh \
  current-omnibox-caret.sh \
  current-provider-rendering.sh \
  current-v2-1-28-media-stability.sh \
  current-v2-1-29-x11-compat-cutover.sh \
  current-v2-1-30-chromium-web-cutover.sh \
  current-viewport-transform.sh \
  current-youtube-runtime-resolver.sh \
  current-youtube.sh; do
  [[ ! -e "tests/$retired" ]] || fail "retired-contract-still-present:$retired"
done

echo 'AETHER_BROWSER_V2_1_56_LOOP_BREAK_CLOSURE=PASS'
