use std::{env, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioReadiness {
    pub pipewire_available: bool,
    pub detail: String,
}

pub fn detect() -> AudioReadiness {
    let socket = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .map(|root| root.join("pipewire-0").exists())
        .unwrap_or(false);
    let available = socket || command_exists("pipewire") || command_exists("wpctl");
    AudioReadiness {
        pipewire_available: available,
        detail: if available {
            "PipeWire detected".into()
        } else {
            "PipeWire not detected".into()
        },
    }
}

fn command_exists(name: &str) -> bool {
    env::var_os("PATH")
        .is_some_and(|paths| env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_probe_is_non_fatal() {
        let report = detect();
        assert!(!report.detail.is_empty());
    }
}
