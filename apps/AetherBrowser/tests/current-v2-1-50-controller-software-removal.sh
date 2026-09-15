#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
fail(){ echo "AETHER_BROWSER_V2_1_52_NO_STREAMDECK_SOFTWARE=FAIL:$1"; exit 1; }
[[ ! -d "$ROOT/crates/aether-deck" ]] || fail deck-crate-present
for path in \
  scripts/streamdeck-plus-runtime-test.sh \
  scripts/elgato-streamdeck-pkg-normalize.py \
  packaging/systemd/aether-browser-deck.service \
  packaging/udev/70-aetherforge-streamdeck.rules \
  packaging/desktop/org.aetherforge.StreamDeckStudio.desktop \
  packaging/wrappers/aether-deck-daemon \
  packaging/wrappers/aether-deck-monitor \
  packaging/wrappers/aether-streamdeck-studio; do
  [[ ! -e "$ROOT/$path" ]] || fail "present:$path"
done
for file in \
  crates/aether-native-pages/src/lib.rs \
  crates/aether-engine-servo/src/live.rs \
  crates/aether-browser/src/main.rs \
  crates/aether-creator-integrations/src/vendor_packages.rs \
  scripts/verify.sh \
  scripts/install-current-tree.sh \
  scripts/build-first-party-release.sh \
  scripts/package-consolidated.sh; do
  [[ -f "$ROOT/$file" ]] || continue
  if grep -qiE 'stream[ _-]?deck|aether-deck|marketplace\.elgato\.com|STREAM_DECK_751|STREAMDECK' "$ROOT/$file"; then
    fail "reference:$file"
  fi
done
echo 'AETHER_BROWSER_V2_1_52_NO_STREAMDECK_SOFTWARE=PASS'
