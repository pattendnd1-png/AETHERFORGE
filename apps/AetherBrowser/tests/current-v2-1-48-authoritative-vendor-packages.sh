#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MOD="$ROOT/crates/aether-creator-integrations/src/vendor_packages.rs"
LIB="$ROOT/crates/aether-creator-integrations/src/lib.rs"
PAGES="$ROOT/crates/aether-native-pages/src/lib.rs"
HELPER="$ROOT/scripts/vendor-package-normalize.py"
INSTALL="$ROOT/scripts/install-current-tree.sh"
fail(){ echo "AETHER_BROWSER_V2_1_48_AUTHORITATIVE_VENDOR_PACKAGES=FAIL:$1"; exit 1; }
[[ -f "$MOD" ]] || fail registry-module-missing
[[ -f "$HELPER" ]] || fail normalizer-helper-missing
grep -q 'pub mod vendor_packages;' "$LIB" || fail module-export-missing
for value in 'OBS-Studio-32.2.2-Windows-x64-Installer.exe' 'c3a0b880adbe64dc4bcb68f93016916ab5b55ae43fd227115287bf80257d92dc' 'obs-streamelements-setup-latest.exe' '79be650ee593f79727e94c098a7d9a60fbe16ef73db2c7c54472aa7753a29ad3' 'Streamlabs+Desktop+Setup+1.21.9-5qLbAShV5RGxPpP.exe' '57e0c280bc4a85e66411a1ed0f874d56b8f2a33590d42146c6d07bab96ca9ceb' 'Ground Control_x64_en-US.msi' 'b374dbf545d00626fe134b4e23e6e542eba64ebb8b9f66f4b056c11233da0ed5'; do grep -qF "$value" "$MOD" || fail "registry-value-missing:$value"; done
for version in '32.2.2' '1.21.9' '2.1.20'; do grep -qF "$version" "$MOD" || fail "version-missing:$version"; done
for symbol in VendorPackageDescriptor VendorPackageFormat VendorExecutionPolicy BrowserIntegrationTarget authoritative_vendor_packages discover_authoritative_package; do grep -q "$symbol" "$MOD" || fail "registry-symbol-missing:$symbol"; done
grep -q 'MetadataReferenceOnly' "$MOD" || fail vendor-execution-policy-not-reference-only
for action in 'Mute Alerts' 'UnMute Alerts' 'Pause Alerts' 'Resume Alerts' 'Skip Alert' 'Toggle Alerts'; do grep -qF "$action" "$MOD" || fail "ground-control-action-missing:$action"; done
grep -q 'Authoritative Vendor Baselines' "$PAGES" || fail browser-baseline-section-missing
grep -q 'authoritative_vendor_packages' "$PAGES" || fail browser-registry-consumption-missing
grep -q 'OBS Studio 32.2.2' "$PAGES" || fail obs-baseline-copy-missing
grep -q 'Streamlabs Desktop 1.21.9' "$PAGES" || fail streamlabs-baseline-copy-missing
grep -q 'Ground Control 2.1.20' "$PAGES" || fail ground-control-baseline-copy-missing
grep -q 'vendor_code_executed' "$HELPER" || fail no-execution-marker-missing
grep -q 'False' "$HELPER" || fail no-execution-false-missing
if grep -Eq 'subprocess\.(run|Popen)|os\.system|exec\(|eval\(' "$HELPER"; then fail helper-must-not-execute-vendor-code; fi
grep -q 'vendor-package-normalize.py' "$INSTALL" || fail normalizer-not-installed
if find "$ROOT" -type f \( -iname '*.pkg' -o -iname '*.msi' -o -iname '*.exe' \) -print -quit | grep -q .; then fail vendor-installer-must-not-be-redistributed; fi
echo 'AETHER_BROWSER_V2_1_48_AUTHORITATIVE_VENDOR_PACKAGES=PASS'
