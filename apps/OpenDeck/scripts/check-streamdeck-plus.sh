#!/usr/bin/env bash
set -u

VID="0fd9"
PID="0084"
visible=0
hidraw_path=""

printf '%s\n' 'OPENDECK_STREAMDECK_PLUS_PROBE=START'
printf 'OPENDECK_STREAMDECK_PLUS_VID=%s\n' "$VID"
printf 'OPENDECK_STREAMDECK_PLUS_PID=%s\n' "$PID"

# Prefer the kernel HID topology because this probe must remain read-only.
for node in /sys/class/hidraw/hidraw*; do
  [[ -e "$node" ]] || continue
  uevent="$node/device/uevent"
  [[ -r "$uevent" ]] || continue
  if grep -Eqi '^HID_ID=[0-9A-Fa-f]+:00000FD9:00000084$' "$uevent"; then
    visible=1
    hidraw_path="/dev/$(basename "$node")"
    break
  fi
done

# Some kernels expose the match more clearly under /sys/bus/hid/devices.
if (( visible == 0 )); then
  for node in /sys/bus/hid/devices/*; do
    [[ -d "$node" ]] || continue
    uevent="$node/uevent"
    [[ -r "$uevent" ]] || continue
    if grep -Eqi '^HID_ID=[0-9A-Fa-f]+:00000FD9:00000084$' "$uevent"; then
      visible=1
      for child in "$node"/hidraw/hidraw*; do
        [[ -e "$child" ]] || continue
        hidraw_path="/dev/$(basename "$child")"
        break
      done
      break
    fi
  done
fi

# lsusb is diagnostic fallback only; it does not change the device.
if (( visible == 0 )) && command -v lsusb >/dev/null 2>&1; then
  if lsusb -d 0fd9:0084 2>/dev/null | grep -q .; then
    visible=1
  fi
fi

if (( visible == 1 )); then
  printf '%s\n' 'OPENDECK_STREAMDECK_PLUS_OS_VISIBLE=PASS'
else
  printf '%s\n' 'OPENDECK_STREAMDECK_PLUS_OS_VISIBLE=FAIL'
  printf '%s\n' 'OPENDECK_STREAMDECK_PLUS_HINT=Connect the Stream Deck + directly by USB and rerun this probe.'
  exit 1
fi

if [[ -n "$hidraw_path" ]]; then
  printf 'OPENDECK_STREAMDECK_PLUS_HIDRAW=%s\n' "$hidraw_path"
  if [[ -r "$hidraw_path" && -w "$hidraw_path" ]]; then
    printf '%s\n' 'OPENDECK_STREAMDECK_PLUS_HIDRAW_RW=PASS'
  else
    printf '%s\n' 'OPENDECK_STREAMDECK_PLUS_HIDRAW_RW=FAIL'
    printf '%s\n' 'OPENDECK_STREAMDECK_PLUS_HINT=Install packaging/70-opendeck-streamdeck.rules, reload udev rules, then reconnect the device.'
    exit 2
  fi
else
  printf '%s\n' 'OPENDECK_STREAMDECK_PLUS_HIDRAW=NOT_FOUND'
  printf '%s\n' 'OPENDECK_STREAMDECK_PLUS_HIDRAW_RW=FAIL'
  printf '%s\n' 'OPENDECK_STREAMDECK_PLUS_HINT=The USB device is visible but no Stream Deck + hidraw node is accessible.'
  exit 2
fi

printf '%s\n' 'OPENDECK_STREAMDECK_PLUS_PROBE=PASS'
