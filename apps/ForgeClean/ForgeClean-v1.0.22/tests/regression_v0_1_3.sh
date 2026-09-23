#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
grep -q 'PackageSignature' "$ROOT/src/manifest.rs"
grep -q 'package-signature' "$ROOT/src/manifest.rs"
grep -q 'signature_path_for_archive' "$ROOT/src/scan.rs"
grep -q 'is_package_signature' "$ROOT/src/purge.rs"
grep -q 'FORGECLEAN_SIGNATURE_SIDECAR_TEST' "$ROOT/build-and-verify.sh"
grep -q 'FORGECLEAN_INSTALLED_SIGNATURE_PIN_TEST' "$ROOT/build-and-verify.sh"
grep -q 'installed_version_pin_keeps_archive_and_signature_pair' "$ROOT/tests/scan_manifest.rs"
