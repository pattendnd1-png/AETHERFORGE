#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_RESIZABLE_WINDOW=FAIL:$1"; exit 1; }
UI=crates/aether-ui/src/lib.rs
LIVE=crates/aether-engine-servo/src/live.rs
grep -qF 'pub enum WindowResizeEdge' "$UI" || fail resize-model
grep -qF 'pub const RESIZE_BORDER_PX: u32 = 7;' "$UI" || fail resize-border
grep -qF 'pub const MIN_WINDOW_WIDTH_PX: u32 = 760;' "$UI" || fail min-width
grep -qF 'pub const MIN_WINDOW_HEIGHT_PX: u32 = 520;' "$UI" || fail min-height
for edge in North South East West NorthEast NorthWest SouthEast SouthWest; do grep -qF "WindowResizeEdge::$edge" "$UI" || fail "edge:$edge"; done
grep -qF '.with_resizable(true)' "$LIVE" || fail window-not-resizable
grep -qF '.with_min_inner_size(' "$LIVE" || fail no-min-size
grep -qF 'drag_resize_window(resize_direction(edge))' "$LIVE" || fail no-drag-resize
grep -qF 'ResizeDirection::NorthWest' "$LIVE" || fail no-native-directions
grep -qF 'CursorIcon::NwseResize' "$LIVE" || fail no-resize-cursor
grep -qF 'WindowEvent::Resized(size)' "$LIVE" || fail no-resize-event
grep -qF 'self.content_size_for_size(size)' "$LIVE" || fail resize-not-driving-content
echo 'AETHER_BROWSER_RESIZABLE_WINDOW=PASS'
