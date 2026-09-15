use crate::backend::BackendError;
use forgehx_core::MouseDeviceState;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatbagDevice {
    pub id: String,
    pub name: String,
    pub model: Option<String>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RatbagClient;

impl RatbagClient {
    pub fn available(&self) -> bool {
        Command::new("ratbagctl")
            .arg("list")
            .output()
            .is_ok_and(|out| out.status.success())
    }

    pub fn devices(&self) -> Result<Vec<RatbagDevice>, BackendError> {
        let text = run(&["list".into()])?;
        let mut devices = parse_list(&text);
        for device in &mut devices {
            if let Ok(info) = run(&[device.id.clone(), "info".into()]) {
                device.model = info.lines().find_map(|line| {
                    line.trim()
                        .strip_prefix("Model:")
                        .map(|value| value.trim().to_owned())
                });
            }
        }
        Ok(devices)
    }

    pub fn state(&self, device: &str) -> Result<MouseDeviceState, BackendError> {
        validate_id(device)?;
        let text = run(&[device.into(), "info".into()])?;
        parse_info(device, &text)
    }

    pub fn set_dpi(
        &self,
        device: &str,
        profile: u8,
        resolution: usize,
        dpi: u16,
    ) -> Result<(), BackendError> {
        validate_id(device)?;
        validate_dpi(dpi)?;
        run_ok(&[
            device.into(),
            "profile".into(),
            profile.to_string(),
            "resolution".into(),
            resolution.to_string(),
            "dpi".into(),
            "set".into(),
            dpi.to_string(),
        ])
    }

    pub fn set_active_resolution(
        &self,
        device: &str,
        profile: u8,
        resolution: usize,
    ) -> Result<(), BackendError> {
        validate_id(device)?;
        run_ok(&[
            device.into(),
            "profile".into(),
            profile.to_string(),
            "resolution".into(),
            "active".into(),
            "set".into(),
            resolution.to_string(),
        ])
    }

    pub fn set_report_rate(&self, device: &str, profile: u8, hz: u16) -> Result<(), BackendError> {
        validate_id(device)?;
        if ![125, 250, 500, 1000, 2000, 4000, 8000].contains(&hz) {
            return Err(BackendError::Invalid("unsupported report rate".into()));
        }
        run_ok(&[
            device.into(),
            "profile".into(),
            profile.to_string(),
            "rate".into(),
            "set".into(),
            hz.to_string(),
        ])
    }

    pub fn set_profile(&self, device: &str, profile: u8) -> Result<(), BackendError> {
        validate_id(device)?;
        run_ok(&[
            device.into(),
            "profile".into(),
            "active".into(),
            "set".into(),
            profile.to_string(),
        ])
    }

    pub fn set_button_action(
        &self,
        device: &str,
        profile: u8,
        button: u32,
        action: &str,
    ) -> Result<(), BackendError> {
        validate_id(device)?;
        let action = validate_action(action)?;
        let mut args = vec![
            device.into(),
            "profile".into(),
            profile.to_string(),
            "button".into(),
            button.to_string(),
            "action".into(),
            "set".into(),
        ];
        args.extend(action);
        run_ok(&args)
    }
}

fn run(args: &[String]) -> Result<String, BackendError> {
    let output = Command::new("ratbagctl")
        .args(args)
        .output()
        .map_err(|e| BackendError::Unavailable(format!("ratbagctl is not available: {e}")))?;
    if !output.status.success() {
        return Err(BackendError::Io(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
fn run_ok(args: &[String]) -> Result<(), BackendError> {
    run(args).map(|_| ())
}
fn validate_id(value: &str) -> Result<(), BackendError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        Err(BackendError::Invalid(
            "invalid ratbag device identifier".into(),
        ))
    } else {
        Ok(())
    }
}
fn validate_dpi(dpi: u16) -> Result<(), BackendError> {
    if (100..=30000).contains(&dpi) {
        Ok(())
    } else {
        Err(BackendError::Invalid("DPI must be 100..=30000".into()))
    }
}
fn validate_action(value: &str) -> Result<Vec<String>, BackendError> {
    if value.is_empty() || value.len() > 512 || value.contains('\n') || value.contains('\0') {
        return Err(BackendError::Invalid("invalid ratbag button action".into()));
    }
    let tokens = value
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if tokens.is_empty() || tokens.len() > 64 {
        return Err(BackendError::Invalid("invalid ratbag button action".into()));
    }
    if tokens.iter().any(|token| {
        token.len() > 64
            || !token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_+".contains(c))
    }) {
        return Err(BackendError::Invalid(
            "ratbag action contains unsupported characters".into(),
        ));
    }
    Ok(tokens)
}

pub fn parse_list(text: &str) -> Vec<RatbagDevice> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let (id, name) = line.split_once(':').or_else(|| line.split_once(" - "))?;
            let id = id.trim();
            let name = name.trim();
            if validate_id(id).is_err() || name.is_empty() {
                return None;
            }
            Some(RatbagDevice {
                id: id.into(),
                name: name.into(),
                model: None,
            })
        })
        .collect()
}

pub fn parse_info(device: &str, text: &str) -> Result<MouseDeviceState, BackendError> {
    let mut state = MouseDeviceState {
        backend_device_id: device.into(),
        ..Default::default()
    };
    let mut current_profile: Option<u8> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("Profile ") {
            if let Some(number) = line
                .strip_prefix("Profile ")
                .and_then(|s| s.split(':').next())
                .and_then(|s| s.parse::<u8>().ok())
            {
                current_profile = Some(number);
                if line.contains("(active)") {
                    state.active_profile = Some(number);
                }
            }
            continue;
        }
        if current_profile == state.active_profile {
            if let Some(rate) = line
                .strip_prefix("Report Rate:")
                .and_then(|s| s.trim().strip_suffix("Hz"))
                .and_then(|s| s.parse::<u16>().ok())
            {
                state.report_rate_hz = Some(rate);
            }
            if let Some((index, rest)) = line.split_once(':') {
                if index.trim().parse::<usize>().is_ok() && rest.contains("dpi") {
                    let token = rest.split_whitespace().next().unwrap_or("");
                    let dpi = token
                        .split('x')
                        .next()
                        .unwrap_or(token)
                        .strip_suffix("dpi")
                        .unwrap_or(token)
                        .parse::<u16>()
                        .ok();
                    if let Some(dpi) = dpi {
                        let idx = index.trim().parse::<usize>().unwrap();
                        if state.dpi_stages.len() <= idx {
                            state.dpi_stages.resize(idx + 1, 0);
                        }
                        state.dpi_stages[idx] = dpi;
                        if rest.contains("(active)") {
                            state.active_dpi_stage = Some(idx);
                        }
                    }
                }
            }
        }
        if let Some(value) = line
            .strip_prefix("Number of Buttons:")
            .and_then(|s| s.trim().parse::<u16>().ok())
        {
            state.button_count = Some(value);
        }
    }
    if state.active_profile.is_none() {
        current_profile = Some(0);
        state.active_profile = current_profile;
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_ratbag_list() {
        let values = parse_list("hollering-marmot: Logitech G502 HERO\n");
        assert_eq!(values[0].id, "hollering-marmot");
    }
    #[test]
    fn parses_active_profile_dpi_and_rate() {
        let text="Number of Buttons: 11\nProfile 0: (active)\n  Report Rate: 1000Hz\n  Resolutions:\n    0: 800dpi\n    1: 1600dpi (active)\n";
        let state = parse_info("mouse", text).unwrap();
        assert_eq!(state.active_profile, Some(0));
        assert_eq!(state.report_rate_hz, Some(1000));
        assert_eq!(state.dpi_stages, vec![800, 1600]);
        assert_eq!(state.active_dpi_stage, Some(1));
    }
    #[test]
    fn rejects_shell_metacharacters_in_actions() {
        assert!(validate_action("macro ; rm").is_err());
        assert!(validate_id("mouse;rm").is_err());
    }
}
