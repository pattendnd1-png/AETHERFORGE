#!/usr/bin/env python3
"""Normalize identity evidence from user-owned vendor installers without executing them."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePath
import sys
import time

XAR_MAGIC = b"xar!"
OLE_MAGIC = bytes.fromhex("d0cf11e0a1b11ae1")
PE_MAGIC = b"MZ"
NSIS_MARKER = b"Nullsoft"

PACKAGES = {
    "obs-studio-32.2.2": {
        "display_name": "OBS Studio 32.2.2",
        "version": "32.2.2",
        "sha256": "c3a0b880adbe64dc4bcb68f93016916ab5b55ae43fd227115287bf80257d92dc",
        "format": "windows_nsis_exe",
        "browser_target": "obs_workspace",
        "evidence": ["OBS Studio", "32.2.2", "Nullsoft"],
    },
    "obs-streamelements-latest": {
        "display_name": "OBS + StreamElements",
        "version": None,
        "sha256": "79be650ee593f79727e94c098a7d9a60fbe16ef73db2c7c54472aa7753a29ad3",
        "format": "windows_nsis_exe",
        "browser_target": "streamelements_studio",
        "evidence": ["streamelements", "Nullsoft"],
    },
    "streamlabs-desktop-1.21.9": {
        "display_name": "Streamlabs Desktop 1.21.9",
        "version": "1.21.9",
        "sha256": "57e0c280bc4a85e66411a1ed0f874d56b8f2a33590d42146c6d07bab96ca9ceb",
        "format": "windows_nsis_exe",
        "browser_target": "streamlabs_studio",
        "evidence": ["Streamlabs Desktop", "1.21.9", "Nullsoft"],
    },
    "streamelements-ground-control-2.1.20": {
        "display_name": "Ground Control 2.1.20",
        "version": "2.1.20",
        "sha256": "b374dbf545d00626fe134b4e23e6e542eba64ebb8b9f66f4b056c11233da0ed5",
        "format": "windows_msi",
        "browser_target": "streamelements_ground_control",
        "evidence": [
            "Ground Control",
            "2.1.20",
            "com.streamelements.ground-control",
            "Mute Alerts",
            "UnMute Alerts",
            "Pause Alerts",
            "Resume Alerts",
            "Skip Alert",
            "Toggle Alerts",
        ],
    },
}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def contains_text(data: bytes, text: str) -> bool:
    raw = text.encode("utf-8")
    utf16 = text.encode("utf-16le")
    lower = data.lower()
    return raw.lower() in lower or utf16.lower() in lower


def validate_container(data: bytes, package_format: str) -> bool:
    if package_format == "mac_xar_pkg":
        return data.startswith(XAR_MAGIC)
    if package_format == "windows_msi":
        return data.startswith(OLE_MAGIC)
    if package_format == "windows_nsis_exe":
        return data.startswith(PE_MAGIC) and NSIS_MARKER.lower() in data.lower()
    return False


def normalize(package_id: str, source: Path) -> dict:
    if package_id not in PACKAGES:
        raise ValueError(f"unknown-package-id:{package_id}")
    if not source.is_file():
        raise ValueError(f"package-not-found:{source}")
    spec = PACKAGES[package_id]
    data = source.read_bytes()
    actual_sha = hashlib.sha256(data).hexdigest()
    evidence = {text: contains_text(data, text) for text in spec["evidence"]}
    return {
        "schema_version": 1,
        "package_id": package_id,
        "display_name": spec["display_name"],
        "source_path": str(source),
        "source_basename": PurePath(source).name,
        "source_size": len(data),
        "source_sha256": actual_sha,
        "expected_sha256": spec["sha256"],
        "hash_match": actual_sha == spec["sha256"],
        "version": spec["version"],
        "format": spec["format"],
        "format_signature_match": validate_container(data, spec["format"]),
        "browser_target": spec["browser_target"],
        "execution_policy": "metadata_reference_only",
        "vendor_code_executed": False,
        "evidence": evidence,
        "normalized_at_unix": int(time.time()),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-id", required=True, choices=sorted(PACKAGES))
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        result = normalize(args.package_id, args.input)
    except Exception as error:
        print(f"vendor-package-normalize:{error}", file=sys.stderr)
        return 2
    payload = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        temporary = args.output.with_suffix(args.output.suffix + ".tmp")
        temporary.write_text(payload, encoding="utf-8")
        temporary.chmod(0o600)
        temporary.replace(args.output)
    else:
        sys.stdout.write(payload)
    return 0 if result["hash_match"] and result["format_signature_match"] else 3


if __name__ == "__main__":
    raise SystemExit(main())
