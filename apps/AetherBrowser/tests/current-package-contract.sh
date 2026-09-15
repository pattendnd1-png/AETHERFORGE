#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_CURRENT_PACKAGE=FAIL:$1"; exit 1; }
for f in packaging/wrappers/aether-browser packaging/wrappers/aether-media-service packaging/systemd/aether-browser-media.service packaging/desktop/org.aetherforge.AetherBrowser.desktop.in packaging/LICENSE-MIT-OR-APACHE-2.0; do
  [[ -f "$f" ]] || fail "missing:$f"
done
grep -qF 'Exec=/usr/bin/aether-browser %U' packaging/desktop/org.aetherforge.AetherBrowser.desktop.in || fail desktop-exec
grep -qF 'X-AetherForge-Version=2.1.60' packaging/desktop/org.aetherforge.AetherBrowser.desktop.in || fail desktop-version
for removed in crates/aether-deck packaging/wrappers/aether-deck-daemon packaging/wrappers/aether-deck-monitor packaging/wrappers/aether-streamdeck-studio packaging/systemd/aether-browser-deck.service packaging/udev/70-aetherforge-streamdeck.rules packaging/desktop/org.aetherforge.StreamDeckStudio.desktop scripts/streamdeck-plus-runtime-test.sh scripts/elgato-streamdeck-pkg-normalize.py; do
  [[ ! -e "$removed" ]] || fail "removed-streamdeck-path-present:$removed"
done
[[ ! -e crates/aether-ui/assets/browser.html ]] || fail legacy-browser-html
for asset in crates/aether-ui/assets/aetherforge-cosmic-wallpaper.jpg crates/aether-ui/assets/aether-stream-studio-preview.png crates/aether-ui/assets/aether-vault-preview.png; do
  [[ -f "$asset" ]] || fail "missing-native-ui-asset:$asset"
  grep -qF "'$asset'" scripts/package-release.sh || fail "package-manifest-missing:$asset"
done
if grep -qF "'crates/aether-ui/assets/browser.html'" scripts/package-release.sh; then fail legacy-browser-html-manifest; fi
echo 'AETHER_BROWSER_CURRENT_PACKAGE=PASS'
