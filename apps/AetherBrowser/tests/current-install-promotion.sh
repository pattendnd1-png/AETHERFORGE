#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_INSTALL_PROMOTION=FAIL:$1"; exit 1; }
HOST=scripts/install-host.sh
VERIFY=scripts/verify.sh
INSTALL=scripts/install-current-tree.sh

# Installer must run only hard preinstall verification before package promotion.
grep -qF 'AETHER_BROWSER_VERIFY_PHASE=preinstall' "$HOST" || fail host-not-using-preinstall-phase
if grep -Eq '^AETHER_BROWSER_MEDIA_RENDERER=cpu-bgra AETHER_BROWSER_OUT_DIR="\$DOWNLOADS" bash scripts/verify\.sh \|\| fail' "$HOST"; then
  fail host-still-blocked-by-full-runtime-verify
fi
grep -qF 'AETHER_BROWSER_PREINSTALL_VISUAL_PROBES=DEFERRED' "$VERIFY" || fail no-preinstall-visual-deferred-marker
grep -qF "record 'AETHER_BROWSER_POSTINSTALL_VISUAL=FAIL'" "$INSTALL" || fail no-postinstall-visual-hard-fail

grep -qF 'STAGE_DIR="$STATE_DIR/staging/${VERSION}-${PKGREL}"' "$INSTALL" || fail no-versioned-stage-dir
grep -qF 'AETHER_BROWSER_STAGE_PACKAGE_INTEGRITY=PASS' "$INSTALL" || fail no-stage-package-integrity
grep -qF 'AETHER_BROWSER_STAGE_VERSION_IDENTITY=PASS' "$INSTALL" || fail no-stage-version-identity
grep -qF 'AETHER_BROWSER_ATOMIC_PROMOTION=PASS:' "$INSTALL" || fail no-promotion-pass-marker

