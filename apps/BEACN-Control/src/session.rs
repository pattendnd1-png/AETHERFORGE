use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

const REEXEC_ARG: &str = "--graphical-reexec";
const GRAPHICAL_KEYS: [&str; 6] = [
    "WAYLAND_DISPLAY",
    "DISPLAY",
    "XDG_RUNTIME_DIR",
    "XDG_SESSION_TYPE",
    "DBUS_SESSION_BUS_ADDRESS",
    "XAUTHORITY",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicalSessionAction {
    Ready,
    ReexecStarted,
}

pub fn parse_graphical_environment(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(parse_environment_line)
        .filter(|(key, value)| is_graphical_key(key) && !value.is_empty())
        .collect()
}

pub fn graphical_environment_is_usable(values: &BTreeMap<String, String>) -> bool {
    let wayland_ready =
        values.contains_key("WAYLAND_DISPLAY") && values.contains_key("XDG_RUNTIME_DIR");
    wayland_ready || values.contains_key("DISPLAY")
}

pub fn ensure_graphical_session() -> io::Result<GraphicalSessionAction> {
    if current_process_has_display() {
        return Ok(GraphicalSessionAction::Ready);
    }

    if env::args_os().any(|arg| arg.to_string_lossy() == REEXEC_ARG) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "graphical session variables are still unavailable after recovery",
        ));
    }

    let recovered = recover_graphical_environment();
    if !graphical_environment_is_usable(&recovered) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "neither WAYLAND_DISPLAY nor DISPLAY could be recovered from the user session",
        ));
    }

    let executable = env::current_exe()?;
    let mut command = Command::new(executable);
    command.stdin(Stdio::null());
    let arguments = env::args_os()
        .skip(1)
        .filter(|arg| arg.to_string_lossy() != REEXEC_ARG);
    command.args(arguments);
    command.arg(REEXEC_ARG);
    for (key, value) in recovered {
        command.env(key, value);
    }
    command.spawn()?;

    Ok(GraphicalSessionAction::ReexecStarted)
}

fn current_process_has_display() -> bool {
    let wayland_display = env::var_os("WAYLAND_DISPLAY").is_some();
    let wayland_socket = env::var_os("WAYLAND_SOCKET").is_some();
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR").is_some();
    let x11_display = env::var_os("DISPLAY").is_some();
    wayland_socket || (wayland_display && runtime_dir) || x11_display
}

pub fn recover_graphical_environment() -> BTreeMap<String, String> {
    let mut recovered = from_systemd_user_environment();
    for (key, value) in from_graphical_process_environment() {
        recovered.insert(key, value);
    }
    recovered
}

fn from_systemd_user_environment() -> BTreeMap<String, String> {
    let Ok(output) = Command::new("systemctl")
        .args(["--user", "show-environment"])
        .output()
    else {
        return BTreeMap::new();
    };
    if !output.status.success() {
        return BTreeMap::new();
    }
    parse_graphical_environment(&String::from_utf8_lossy(&output.stdout))
}

fn from_graphical_process_environment() -> BTreeMap<String, String> {
    const CANDIDATES: [&str; 3] = ["kwin_wayland", "plasmashell", "ksmserver"];
    let Ok(entries) = fs::read_dir("/proc") else {
        return BTreeMap::new();
    };
    let process_dirs: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|pid| pid.bytes().all(|byte| byte.is_ascii_digit()))
        })
        .map(|entry| entry.path())
        .collect();

    for candidate in CANDIDATES {
        for process_dir in &process_dirs {
            if process_name(process_dir).as_deref() != Some(candidate) {
                continue;
            }
            if let Some(environment) = process_environment(process_dir)
                && graphical_environment_is_usable(&environment)
            {
                return environment;
            }
        }
    }
    BTreeMap::new()
}

fn process_name(process_dir: &Path) -> Option<String> {
    fs::read_to_string(process_dir.join("comm"))
        .ok()
        .map(|value| value.trim().to_owned())
}

fn process_environment(process_dir: &Path) -> Option<BTreeMap<String, String>> {
    let bytes = fs::read(process_dir.join("environ")).ok()?;
    let mut values = BTreeMap::new();
    for field in bytes
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
    {
        let text = String::from_utf8_lossy(field);
        if let Some((key, value)) = parse_environment_line(&text)
            && is_graphical_key(&key)
            && !value.is_empty()
        {
            values.insert(key, value);
        }
    }
    Some(values)
}

fn parse_environment_line(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once('=')?;
    if key.is_empty() {
        return None;
    }
    Some((key.to_owned(), value.to_owned()))
}

fn is_graphical_key(key: &str) -> bool {
    GRAPHICAL_KEYS.contains(&key)
}
