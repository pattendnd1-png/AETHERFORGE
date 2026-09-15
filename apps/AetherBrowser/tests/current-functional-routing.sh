#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_FUNCTIONAL_ROUTING=FAIL:$1"; exit 1; }
NATIVE=crates/aether-native-pages/src/lib.rs
LIVE=crates/aether-engine-servo/src/live.rs
UI=crates/aether-ui/src/lib.rs
for needle in \
  'aether://auth/start?provider={id}' \
  'aether://provider/open?id={id}' \
  'aether://provider/configure?id={id}' \
  'aether://vault?action=unlock' \
  'aether://vault?action=add-login' \
  'aether://vault?action=generate-password' \
  'aether://comms?action=add-mail' \
  'aether://comms?action=connect-provider' \
  'aether://library?section=history' \
  'aether://library?section=bookmarks' \
  'aether://library?section=downloads' \
  'aether://library?section=recently-closed' \
  'aether://youtube-music'; do grep -qF "$needle" "$NATIVE" || fail "missing-native-route:$needle"; done
for id in '"twitch"' '"google"' '"apple"' '"local"'; do grep -qF "$id" "$NATIVE" || fail "missing-account-provider:$id"; done
grep -qF 'NativePageRoute::Settings' "$NATIVE" || fail settings-page
grep -qF 'fn resolve_interface_action_url' "$LIVE" || fail dispatcher
grep -qF 'https://www.twitch.tv/login' "$LIVE" || fail twitch-login
grep -qF 'https://accounts.google.com/' "$LIVE" || fail google-login
grep -qF 'https://appleid.apple.com/' "$LIVE" || fail apple-login
grep -qF 'ChromeHitTarget::Downloads' "$LIVE" || fail downloads-dispatch
grep -qF 'ChromeHitTarget::Settings' "$LIVE" || fail settings-dispatch
grep -qF 'ChromeHitTarget::Downloads' "$UI" || fail downloads-hit
grep -qF 'ChromeHitTarget::Settings' "$UI" || fail settings-hit
echo 'AETHER_BROWSER_FUNCTIONAL_ROUTING=PASS'
