#![forbid(unsafe_code)]
//! Screenshot and page-region capture contracts.

use std::path::PathBuf;

pub const UI_SNAPSHOT_FILENAME: &str = "Aether-Browser-v2.1.60-UI-SNAPSHOT.png";
pub const LIVE_FRAME_PROBE_FILENAME: &str = "Aether-Browser-v2.1.60-LIVE-FRAME.png";
pub const SURFACE_CAPTURE_FRAMES_DIRNAME: &str = "Aether-Browser-v2.1.60-NATIVE-SURFACE-FRAMES";
pub const SURFACE_CAPTURE_GIF_FILENAME: &str = "Aether-Browser-v2.1.60-NATIVE-SURFACE.gif";
pub const SURFACE_CAPTURE_WEBP_FILENAME: &str = "Aether-Browser-v2.1.60-NATIVE-SURFACE.webp";
pub const SURFACE_CAPTURE_MP4_FILENAME: &str = "Aether-Browser-v2.1.60-NATIVE-SURFACE.mp4";
pub const SURFACE_CAPTURE_MANIFEST_FILENAME: &str = "Aether-Browser-v2.1.60-NATIVE-SURFACE.txt";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub const STRICT_NO_RECURSION: &str = "STRICT_NO_RECURSION";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureSafetyPolicy {
    StrictNoRecursion,
}

#[must_use]
pub fn clip_region_to_canvas(
    region: CaptureRegion,
    canvas_width: u32,
    canvas_height: u32,
) -> Option<CaptureRegion> {
    if canvas_width == 0
        || canvas_height == 0
        || region.x >= canvas_width
        || region.y >= canvas_height
    {
        return None;
    }
    let width = region.width.min(canvas_width.saturating_sub(region.x));
    let height = region.height.min(canvas_height.saturating_sub(region.y));
    (width > 0 && height > 0).then_some(CaptureRegion {
        x: region.x,
        y: region.y,
        width,
        height,
    })
}

#[must_use]
pub fn is_recursive_capture_target(input_kind: &str, target_descriptor: &str) -> bool {
    let kind = input_kind.to_ascii_lowercase();
    let target = target_descriptor.to_ascii_lowercase();
    let whole_display = [
        "xshm_input",
        "monitor_capture",
        "display_capture",
        "screen_capture",
        "pipewire-desktop-capture-source",
    ]
    .iter()
    .any(|candidate| kind.contains(candidate));
    if whole_display {
        return true;
    }
    let window_capture = kind.contains("window") || kind.contains("xcomposite");
    let self_surface = [
        "aetherbrowser",
        "aether browser",
        "obs studio",
        "streamlabs desktop",
        "streamlabs",
    ]
    .iter()
    .any(|candidate| target.contains(candidate));
    window_capture && self_surface
}

fn downloads_root() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Downloads")
}

#[must_use]
pub fn ui_snapshot_path() -> PathBuf {
    downloads_root().join(UI_SNAPSHOT_FILENAME)
}

#[must_use]
pub fn live_frame_probe_path() -> PathBuf {
    downloads_root().join(LIVE_FRAME_PROBE_FILENAME)
}

#[must_use]
pub fn surface_capture_frames_path() -> PathBuf {
    downloads_root().join(SURFACE_CAPTURE_FRAMES_DIRNAME)
}

#[must_use]
pub fn surface_capture_gif_path() -> PathBuf {
    downloads_root().join(SURFACE_CAPTURE_GIF_FILENAME)
}

#[must_use]
pub fn surface_capture_webp_path() -> PathBuf {
    downloads_root().join(SURFACE_CAPTURE_WEBP_FILENAME)
}

#[must_use]
pub fn surface_capture_mp4_path() -> PathBuf {
    downloads_root().join(SURFACE_CAPTURE_MP4_FILENAME)
}

#[must_use]
pub fn surface_capture_manifest_path() -> PathBuf {
    downloads_root().join(SURFACE_CAPTURE_MANIFEST_FILENAME)
}

#[cfg(test)]
mod capture_safety_tests {
    use super::*;

    #[test]
    fn strict_policy_blocks_recursive_capture_targets() {
        assert_eq!(
            CaptureSafetyPolicy::StrictNoRecursion,
            CaptureSafetyPolicy::StrictNoRecursion
        );
        assert!(is_recursive_capture_target("xshm_input", "Display 1"));
        assert!(is_recursive_capture_target(
            "xcomposite_input",
            "AetherBrowser — OBS Studio"
        ));
        assert!(!is_recursive_capture_target(
            "game_capture",
            "Star Trek Online"
        ));
    }

    #[test]
    fn scene_regions_are_clipped_to_canvas() {
        let clipped = clip_region_to_canvas(
            CaptureRegion {
                x: 1600,
                y: 900,
                width: 640,
                height: 480,
            },
            1920,
            1080,
        )
        .expect("overlapping region remains visible");
        assert_eq!(
            clipped,
            CaptureRegion {
                x: 1600,
                y: 900,
                width: 320,
                height: 180
            }
        );
        assert!(
            clip_region_to_canvas(
                CaptureRegion {
                    x: 1920,
                    y: 0,
                    width: 100,
                    height: 100
                },
                1920,
                1080,
            )
            .is_none()
        );
    }
}
