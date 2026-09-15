#!/usr/bin/env bash
set -euo pipefail
PROFILE="AetherForge v3.0.5 1080p"
COLLECTION="AetherForge v3.0.5 1920x1080"
SCENE="AF • Starting Soon"
if pgrep -x obs >/dev/null 2>&1 || pgrep -x obs-studio >/dev/null 2>&1; then
  echo "OBS is already running. Close it first so launch parameters select the intended AetherForge profile/collection." >&2
  exit 2
fi
if command -v obs >/dev/null 2>&1; then
  exec obs --profile "$PROFILE" --collection "$COLLECTION" --scene "$SCENE"
fi
if command -v flatpak >/dev/null 2>&1 && flatpak info com.obsproject.Studio >/dev/null 2>&1; then
  exec flatpak run com.obsproject.Studio --profile "$PROFILE" --collection "$COLLECTION" --scene "$SCENE"
fi
echo "OBS Studio was not found as a native 'obs' command or Flatpak com.obsproject.Studio." >&2
exit 127
