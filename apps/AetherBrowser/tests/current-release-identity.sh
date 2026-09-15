#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_CURRENT_RELEASE=FAIL:$1"; exit 1; }
grep -qF 'version = "2.1.60"' Cargo.toml || fail workspace-version
grep -qF 'Aether Browser 2.1.60' crates/aether-browser/src/main.rs || fail binary-version
grep -qF 'AetherForge Browser v2.1.60' crates/aether-ui/src/lib.rs || fail native-chrome-version
for f in scripts/verify.sh scripts/host-build.sh scripts/install-host.sh scripts/install-current-tree.sh scripts/package-release.sh; do
  grep -qF "VERSION='2.1.60'" "$f" || fail "script-version:$f"
done
grep -qF 'X-AetherForge-Version=2.1.60' packaging/desktop/org.aetherforge.AetherBrowser.desktop.in || fail desktop-version
echo 'AETHER_BROWSER_CURRENT_RELEASE=PASS'
