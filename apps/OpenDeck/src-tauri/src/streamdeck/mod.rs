pub(crate) mod protocol;
mod render;
mod runtime;

use crate::editor::{Workspace, validate_workspace};
use runtime::{StreamDeckService, StreamDeckStatus};
use tauri::State;

pub(crate) use runtime::StreamDeckService as Service;

#[tauri::command]
pub(crate) fn streamdeck_status(service: State<'_, StreamDeckService>) -> StreamDeckStatus {
    service.status()
}

#[tauri::command]
pub(crate) fn streamdeck_sync_workspace(
    workspace: Workspace,
    service: State<'_, StreamDeckService>,
) -> Result<(), String> {
    validate_workspace(&workspace)?;
    service.sync_workspace(workspace)
}

#[tauri::command]
pub(crate) fn streamdeck_set_brightness(
    percent: u8,
    service: State<'_, StreamDeckService>,
) -> Result<(), String> {
    service.set_brightness(percent)
}
