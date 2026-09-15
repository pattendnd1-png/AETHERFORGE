#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_V2_1_34_COMPAT_SESSION=FAIL:$1" >&2; exit 1; }
PROFILE='crates/aether-profile/src/lib.rs'
COMPAT='crates/aether-compat/src/lib.rs'
ENGINE='crates/aether-engine-servo/src/live.rs'
INSTALL='scripts/install-current-tree.sh'
WRAPPER='packaging/wrappers/aether-browser'

# Persistent normal/private compatibility profile contract.
grep -qF 'profiles/compat' "$PROFILE" || fail compat-profile-root-missing
grep -qF 'compat_root' "$PROFILE" || fail compat-root-api-missing

# External web routing contract: every HTTP/HTTPS origin uses Chromium.
grep -qF 'ContentEngine::Compatibility' "$COMPAT" || fail compat-classifier-missing
grep -qF 'matches!(url.scheme(), "http" | "https")' "$COMPAT" || fail all-http-routing-missing
! grep -qF 'COMPATIBILITY_HOSTS' "$COMPAT" || fail legacy-provider-allowlist-still-present

# The compatibility surface must be actual system Chromium embedded in-window,
# not a second Servo shim or a WebKit webview.
grep -qF 'ChromiumCompatHost' "$COMPAT" || fail chromium-host-missing
grep -qF '/usr/bin/chromium' "$COMPAT" || fail system-chromium-path-missing
grep -qF -- '--user-data-dir=' "$COMPAT" || fail persistent-chromium-user-data-dir-missing
grep -qF 'reparent_window' "$COMPAT" || fail in-window-x11-reparent-missing
grep -qF 'WM_DELETE_WINDOW' "$COMPAT" || fail graceful-chromium-close-missing
grep -qF 'AETHER_BROWSER_COMPAT_ENGINE=SYSTEM_CHROMIUM_X11_CHILD' "$ENGINE" || fail engine-marker-missing
grep -qF 'AETHER_BROWSER_COMPAT_PROFILE=' "$ENGINE" || fail compat-profile-marker-missing
grep -qF 'PERSISTENT_NORMAL' "$ENGINE" || fail persistent-profile-marker-missing
grep -qF 'PRIVATE_EPHEMERAL' "$ENGINE" || fail private-profile-marker-missing
grep -qF 'AETHER_BROWSER_COMPAT_WINDOW_CLOSE=FLUSH_ONLY_NO_LOGOUT' "$ENGINE" || fail no-logout-marker-missing
grep -qF 'site_data_manager()' "$ENGINE" || fail servo-cookie-export-api-missing
grep -qF 'import_servo_cookies' "$COMPAT" || fail servo-to-chromium-cookie-import-missing
grep -qF 'Storage.setCookies' "$COMPAT" || fail chromium-cookie-cdp-import-missing
grep -qF '.aether-servo-cookie-migration-v1' "$COMPAT" || fail cookie-migration-marker-missing
grep -qF 'classify_content_engine(&requested) == ContentEngine::Compatibility' "$ENGINE" || fail servo-to-compat-navigation-intercept-missing

# AetherBrowser itself must use X11/XWayland so the Chromium child has a real
# X11 parent, and Chromium must be an explicit host dependency.
grep -qF 'WINIT_UNIX_BACKEND' "$WRAPPER" || fail x11-wrapper-enforcement-missing
grep -qF "'chromium'" "$INSTALL" || fail chromium-install-dependency-missing
grep -qF 'AETHER_BROWSER_COMPAT_PROFILE_PRESERVED=PASS' "$INSTALL" || fail installer-profile-preservation-gate-missing

# Upgrades are never allowed to clear the authenticated compatibility profile.
! grep -Eq 'remove_dir_all\([^)]*compat|rm -rf[^\n]*profiles/compat' "$INSTALL" || fail installer-deletes-compat-profile

echo 'AETHER_BROWSER_V2_1_34_COMPAT_SESSION=PASS'
