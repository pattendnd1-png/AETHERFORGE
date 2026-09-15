use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackend {
    Vulkan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderReadiness {
    pub backend: RenderBackend,
    pub available: bool,
    pub detail: String,
}

pub fn detect() -> RenderReadiness {
    let available = vulkan_loader_available() || command_exists("vulkaninfo");
    RenderReadiness {
        backend: RenderBackend::Vulkan,
        available,
        detail: if available {
            "Vulkan loader detected".into()
        } else {
            "Vulkan loader not detected".into()
        },
    }
}

pub fn vulkan_loader_available() -> bool {
    [
        "/usr/lib/libvulkan.so.1",
        "/usr/lib64/libvulkan.so.1",
        "/usr/lib/x86_64-linux-gnu/libvulkan.so.1",
        "/lib/x86_64-linux-gnu/libvulkan.so.1",
    ]
    .iter()
    .any(|p| Path::new(p).exists())
}

fn command_exists(name: &str) -> bool {
    env::var_os("PATH")
        .is_some_and(|paths| env::split_paths(&paths).any(|dir: PathBuf| dir.join(name).is_file()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_renderer_contract_is_vulkan() {
        assert_eq!(detect().backend, RenderBackend::Vulkan);
    }
}
