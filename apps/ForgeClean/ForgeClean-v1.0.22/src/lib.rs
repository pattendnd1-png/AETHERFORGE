//! ForgeClean core library.
//! Linux/AetherForge focused, manifest-gated permanent cleanup with conditional verified offload.

pub mod coldpack_gc;
pub mod coldstore;
pub mod downloads_inventory;
pub mod manifest;
pub mod offload;
pub mod organizer;
pub mod package;
pub mod pre_rebase;
pub mod project_routing;
pub mod project_storage;
pub mod purge;
pub mod registry;
pub mod scan;
pub mod storage;
pub mod system_scan;

pub mod gui;
pub mod gui_state;

pub mod adaptive;

pub mod orbital;
pub mod orbital_monitor_model;
pub mod orbital_ui;
