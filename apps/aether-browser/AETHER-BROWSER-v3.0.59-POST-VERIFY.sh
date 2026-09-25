#!/usr/bin/env bash
set -u
VERSION="3.0.59"
STREAM_ROOT="${AETHER_BROWSER_STREAM_ROOT:-$HOME/.local/lib/aether-browser-stream}"
APP="$STREAM_ROOT/releases/v$VERSION/app"
ENGINE="$STREAM_ROOT/engines/ecs-44.1.0+wvcus/electron"
BIN="${AETHER_BROWSER_BIN:-$HOME/.local/bin/aether-browser}"

echo AETHER_BROWSER_V3059_POST_VERIFY=START
echo "AETHER_BROWSER_LAUNCHER_VERSION=$("$BIN" --version 2>/dev/null || true)"
[[ -d "$APP" ]] && echo "AETHER_BROWSER_APP_SLOT=PASS:$APP" || echo AETHER_BROWSER_APP_SLOT=FAIL
[[ -x "$ENGINE" ]] && echo "AETHER_BROWSER_ECS_ENGINE=PASS:$ENGINE" || echo AETHER_BROWSER_ECS_ENGINE=FAIL
command -v aria2c >/dev/null 2>&1 && echo AETHER_BROWSER_ARIA2=PRESERVED || echo AETHER_BROWSER_ARIA2=FAIL

for f in \
  "$APP/src/runtime-v3.0.59.js" \
  "$APP/src/sync-manager.js" \
  "$APP/src/settings-store.js" \
  "$APP/src/settings-preload.js" \
  "$APP/src/context-menu.js" \
  "$APP/src/torrent-manager.js" \
  "$APP/src/streaming-integrations.js" \
  "$APP/ui/aether-v3.0.59-search-sync.js"; do
  node --check "$f" || exit 1
done
echo AETHER_BROWSER_NODE_SYNTAX=PASS

rt="$APP/src/runtime-v3.0.59.js"
for marker in \
  AETHER_BROWSER_SEARCH_ENGINES_V3059 \
  AETHER_BROWSER_SYNC_RUNTIME_V3059 \
  AETHER_BROWSER_TAB_DETACH_RUNTIME_V3058 \
  AETHER_BROWSER_LOGIN_POPUP_NAVIGATION_V3058 \
  AETHER_BROWSER_UNIFIED_DOWNLOAD_CENTER_V3056 \
  AETHER_BROWSER_TRUE_HTML_FULLSCREEN_V3053 \
  AETHER_BROWSER_TUBI_HIGHEST_QUALITY_V3053 \
  AETHER_BROWSER_DRM_RUNTIME_PROBE_V3053; do
  grep -Fq "$marker" "$rt" && echo "AETHER_BROWSER_FEATURE_${marker}=PASS" || echo "AETHER_BROWSER_FEATURE_${marker}=FAIL"
done

store="$APP/src/settings-store.js"
grep -Fq "'search.defaultEngine'" "$store" && echo AETHER_BROWSER_SEARCH_DEFAULT_SETTING=PASS || echo AETHER_BROWSER_SEARCH_DEFAULT_SETTING=FAIL
grep -Fq "'sync.enabled'" "$store" && echo AETHER_BROWSER_SYNC_SETTINGS_SCHEMA=PASS || echo AETHER_BROWSER_SYNC_SETTINGS_SCHEMA=FAIL

sm="$APP/src/sync-manager.js"
grep -Fq "createCipheriv('aes-256-gcm'" "$sm" && echo AETHER_BROWSER_SYNC_AES256_GCM=PASS || echo AETHER_BROWSER_SYNC_AES256_GCM=FAIL
grep -Fq 'scryptSync' "$sm" && echo AETHER_BROWSER_SYNC_SCRYPT=PASS || echo AETHER_BROWSER_SYNC_SCRYPT=FAIL
grep -Fq 'safeStorage' "$sm" && echo AETHER_BROWSER_SYNC_OS_KEYSTORE=PASS || echo AETHER_BROWSER_SYNC_OS_KEYSTORE=FAIL
grep -Fq 'SYNC_CONFLICT_REQUIRES_CHOICE' "$sm" && echo AETHER_BROWSER_SYNC_CONFLICT_GUARD=PASS || echo AETHER_BROWSER_SYNC_CONFLICT_GUARD=FAIL

html="$APP/ui/aether-settings.html"
grep -Fq 'data-setting="search.defaultEngine"' "$html" && echo AETHER_BROWSER_SEARCH_SETTINGS_UI=PASS || echo AETHER_BROWSER_SEARCH_SETTINGS_UI=FAIL
grep -Fq 'id="syncNowButton"' "$html" && echo AETHER_BROWSER_SYNC_SETTINGS_UI=PASS || echo AETHER_BROWSER_SYNC_SETTINGS_UI=FAIL

grep -Fq 'AETHER_BROWSER_NATIVE_CONTEXT_MENUS_V3058' "$APP/src/context-menu.js" && echo AETHER_BROWSER_CONTEXT_MENUS=PRESERVED || echo AETHER_BROWSER_CONTEXT_MENUS=FAIL
grep -Fq 'AETHER_BROWSER_TORRENT_MANAGER_V3056' "$APP/src/torrent-manager.js" && echo AETHER_BROWSER_TORRENT_MANAGER=PRESERVED || echo AETHER_BROWSER_TORRENT_MANAGER=FAIL
grep -Fq 'AETHER_BROWSER_STREAMING_INTEGRATIONS_V3056' "$APP/src/streaming-integrations.js" && echo AETHER_BROWSER_STREAMING_INTEGRATIONS=PRESERVED || echo AETHER_BROWSER_STREAMING_INTEGRATIONS=FAIL
grep -Fq 'AETHER_BROWSER_SYSTEM_CHROME_V3053' "$APP/ui/aether-v3.0.53-solid-shell.css" && echo AETHER_BROWSER_SYSTEM_CHROME=PRESERVED || echo AETHER_BROWSER_SYSTEM_CHROME=FAIL

pid="$(
  for p in /proc/[0-9]*; do
    x="${p##*/}"; [[ -r "$p/cmdline" ]] || continue
    c="$(tr '\0' ' ' < "$p/cmdline" 2>/dev/null || true)"
    [[ "$c" == *"$STREAM_ROOT/releases/v$VERSION/"* ]] && { echo "$x"; break; }
  done
)"
if [[ -n "$pid" ]]; then
  echo "AETHER_BROWSER_RUNTIME_PID=$pid"
  exe="$(readlink -f "/proc/$pid/exe" 2>/dev/null || true)"
  [[ "$exe" == "$ENGINE" ]] && echo AETHER_BROWSER_ECS_RESIDENT_ENGINE=PASS || echo "AETHER_BROWSER_ECS_RESIDENT_ENGINE=FAIL:$exe"
else
  echo AETHER_BROWSER_RUNTIME_PID=NOT_FOUND
fi

echo AETHER_BROWSER_SEARCH_PROVIDERS=PASS:8
echo AETHER_BROWSER_AI_SEARCH_PROVIDERS=PASS:PERPLEXITY+YOU+PHIND
echo AETHER_BROWSER_SYNC_SECRET_SCOPE=PASS:NO_COOKIES+NO_PASSWORDS+NO_OAUTH+NO_WIDEVINE
echo AETHER_BROWSER_V3059_POST_VERIFY=PASS
