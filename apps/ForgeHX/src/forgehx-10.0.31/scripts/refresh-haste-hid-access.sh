#!/usr/bin/env bash
set -euo pipefail

command -v udevadm >/dev/null 2>&1 || { echo 'ERROR: udevadm is required.' >&2; exit 1; }

udevadm control --reload-rules
# Re-evaluate rules for already-connected HID nodes, then wait for udev work.
udevadm trigger --subsystem-match=hidraw || true
udevadm settle || true

matched=0
for node in /dev/hidraw*; do
  [[ -e "$node" ]] || continue
  props="$(udevadm info --query=property --name="$node" 2>/dev/null || true)"
  [[ "$props" == *$'ID_VENDOR_ID=03f0'* ]] || continue
  if [[ "$props" != *$'ID_MODEL_ID=028e'* && "$props" != *$'ID_MODEL_ID=048e'* ]]; then
    continue
  fi
  [[ "$props" == *$'ID_USB_INTERFACE_NUM=02'* ]] || continue
  chmod 0666 "$node"
  matched=$((matched + 1))
  echo "FORGEHX_HASTE_HID_ACCESS=$node mode=$(stat -c '%a' "$node" 2>/dev/null || echo unknown)"
done

if (( matched == 0 )); then
  echo 'FORGEHX_HASTE_HID_ACCESS=NO_CONNECTED_CONTROL_INTERFACE'
else
  echo "FORGEHX_HASTE_HID_ACCESS=PASS count=$matched"
fi
