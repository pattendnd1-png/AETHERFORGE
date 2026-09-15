#![forbid(unsafe_code)]

use aether_stream_studio::{ObsWebSocketConfig, verify_obs_control_plane};
use std::process::ExitCode;

fn main() -> ExitCode {
    println!("AETHER_BROWSER_OBS_RUNTIME=START");
    let config = ObsWebSocketConfig::from_env();
    match verify_obs_control_plane(config) {
        Ok(report) => {
            println!("AETHER_BROWSER_OBS_WEBSOCKET=PASS");
            println!("AETHER_BROWSER_OBS_VERSION={}", report.obs_version);
            println!(
                "AETHER_BROWSER_OBS_WEBSOCKET_VERSION={}",
                report.websocket_version
            );
            println!("AETHER_BROWSER_OBS_SCENES={}", pass(report.scenes_ok));
            println!("AETHER_BROWSER_OBS_SOURCES={}", pass(report.sources_ok));
            println!("AETHER_BROWSER_OBS_INPUTS={}", pass(report.inputs_ok));
            println!("AETHER_BROWSER_OBS_FILTERS={}", pass(report.filters_ok));
            println!(
                "AETHER_BROWSER_OBS_TRANSITIONS={}",
                pass(report.transitions_ok)
            );
            println!("AETHER_BROWSER_OBS_OUTPUTS={}", pass(report.outputs_ok));
            println!("AETHER_BROWSER_OBS_STATS={}", pass(report.stats_ok));
            println!("AETHER_BROWSER_OBS_PROFILES={}", pass(report.profiles_ok));
            println!(
                "AETHER_BROWSER_OBS_SCENE_COLLECTIONS={}",
                pass(report.scene_collections_ok)
            );
            if report.virtual_camera_available {
                println!(
                    "AETHER_BROWSER_OBS_VIRTUAL_CAMERA={}",
                    pass(report.virtual_camera_ok)
                );
            } else {
                println!("AETHER_BROWSER_OBS_VIRTUAL_CAMERA=NOT_AVAILABLE");
            }
            println!(
                "AETHER_BROWSER_OBS_SCRATCH_SCENE={}",
                pass(report.scratch_scene_ok)
            );
            println!(
                "AETHER_BROWSER_OBS_SCENE_RESTORE={}",
                pass(report.scene_restore_ok)
            );
            let all = report.scenes_ok
                && report.sources_ok
                && report.inputs_ok
                && report.filters_ok
                && report.transitions_ok
                && report.outputs_ok
                && report.stats_ok
                && report.profiles_ok
                && report.scene_collections_ok
                && report.virtual_camera_ok
                && report.scratch_scene_ok
                && report.scene_restore_ok;
            println!("AETHER_BROWSER_OBS_RUNTIME={}", pass(all));
            if all {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            println!("AETHER_BROWSER_OBS_WEBSOCKET=FAIL");
            println!("AETHER_BROWSER_OBS_RUNTIME=FAIL:{error}");
            ExitCode::FAILURE
        }
    }
}

fn pass(value: bool) -> &'static str {
    if value { "PASS" } else { "FAIL" }
}
