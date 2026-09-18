//! Private PipeWire pass-through runtime for AetherForge BEACN Control.
//!
//! The runtime explicitly captures the already-existing raw BEACN PipeWire
//! source, processes it in `private_dsp`, and publishes a separate namespaced
//! source through PipeWire Pulse's `module-pipe-source`. It never changes the
//! system default source/sink and never talks to any AetherForge system-DSP service.

use crate::private_dsp::{PRIVATE_DSP_BLOCK_FRAMES, PRIVATE_DSP_SAMPLE_RATE_HZ, PrivateDspEngine};
use crate::software_dsp::SoftwareDspState;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub const PRIVATE_SOURCE_NAME: &str = "aetherforge_beacn_private_dsp";
pub const PRIVATE_SOURCE_DESCRIPTION: &str = "AetherForge BEACN Processed";
const PRIVATE_SOURCE_PROPERTY_DESCRIPTION: &str = "AetherForge-BEACN-Processed";
const PRIVATE_FIFO_NAME: &str = "private-dsp.float32-mono.fifo";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrivateDspRuntimeState {
    Starting,
    Running,
    #[default]
    Stopped,
    Error,
}

impl PrivateDspRuntimeState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Starting => "PRIVATE DSP STARTING",
            Self::Running => "PRIVATE DSP LIVE",
            Self::Stopped => "PRIVATE DSP STOPPED",
            Self::Error => "PRIVATE DSP ERROR",
        }
    }

    pub const fn is_live(self) -> bool {
        matches!(self, Self::Running)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateDspRuntimeStatus {
    pub state: PrivateDspRuntimeState,
    pub raw_source: String,
    pub processed_source: String,
    pub detail: String,
}

impl PrivateDspRuntimeStatus {
    fn new(state: PrivateDspRuntimeState, raw_source: &str, detail: impl Into<String>) -> Self {
        Self {
            state,
            raw_source: raw_source.to_owned(),
            processed_source: PRIVATE_SOURCE_NAME.to_owned(),
            detail: detail.into(),
        }
    }
}

pub struct PrivateDspRuntime {
    stop: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
    status: Arc<Mutex<PrivateDspRuntimeStatus>>,
    config_tx: Sender<SoftwareDspState>,
    worker: Option<JoinHandle<()>>,
    module_id: Option<u32>,
    fifo_anchor: Option<File>,
    fifo_path: PathBuf,
}

impl PrivateDspRuntime {
    pub fn start(raw_source: &str, profile: SoftwareDspState) -> Result<Self, String> {
        if raw_source.trim().is_empty() {
            return Err("private DSP requires an explicit BEACN raw source".to_owned());
        }
        if !is_beacn_source(raw_source) {
            return Err(format!(
                "private DSP refuses non-BEACN capture source: {raw_source}"
            ));
        }

        require_command("pactl")?;
        require_command("pw-record")?;
        require_command("mkfifo")?;

        let runtime_dir = private_runtime_dir();
        fs::create_dir_all(&runtime_dir)
            .map_err(|error| format!("create private DSP runtime directory: {error}"))?;
        let fifo_path = runtime_dir.join(PRIVATE_FIFO_NAME);
        remove_own_fifo(&fifo_path)?;
        create_fifo(&fifo_path)?;
        // Keep one read/write descriptor open so neither side of the FIFO can
        // deadlock while the PipeWire module and worker attach in sequence.
        let fifo_anchor = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&fifo_path)
            .map_err(|error| format!("open private DSP FIFO anchor: {error}"))?;

        let module_id = match load_private_pipe_source(&fifo_path) {
            Ok(module_id) => module_id,
            Err(error) => {
                let _ = fs::remove_file(&fifo_path);
                return Err(error);
            }
        };

        let stop = Arc::new(AtomicBool::new(false));
        let child = Arc::new(Mutex::new(None));
        let status = Arc::new(Mutex::new(PrivateDspRuntimeStatus::new(
            PrivateDspRuntimeState::Starting,
            raw_source,
            "private source created; starting targeted BEACN capture",
        )));
        let (config_tx, config_rx) = mpsc::channel::<SoftwareDspState>();

        let worker_stop = Arc::clone(&stop);
        let worker_child = Arc::clone(&child);
        let worker_status = Arc::clone(&status);
        let worker_fifo = fifo_path.clone();
        let worker_source = raw_source.to_owned();
        let worker = thread::Builder::new()
            .name("aetherforge-beacn-private-dsp".to_owned())
            .spawn(move || {
                let result = run_private_dsp_worker(
                    &worker_source,
                    &worker_fifo,
                    profile,
                    config_rx,
                    &worker_stop,
                    &worker_child,
                    &worker_status,
                );
                if let Err(error) = result {
                    set_status(
                        &worker_status,
                        PrivateDspRuntimeStatus::new(
                            PrivateDspRuntimeState::Error,
                            &worker_source,
                            error,
                        ),
                    );
                } else if !matches!(status_state(&worker_status), PrivateDspRuntimeState::Error) {
                    set_status(
                        &worker_status,
                        PrivateDspRuntimeStatus::new(
                            PrivateDspRuntimeState::Stopped,
                            &worker_source,
                            "private DSP worker stopped",
                        ),
                    );
                }
            })
            .map_err(|error| {
                let _ = unload_private_pipe_source(module_id);
                let _ = fs::remove_file(&fifo_path);
                format!("start private DSP worker: {error}")
            })?;

        Ok(Self {
            stop,
            child,
            status,
            config_tx,
            worker: Some(worker),
            module_id: Some(module_id),
            fifo_anchor: Some(fifo_anchor),
            fifo_path,
        })
    }

    pub fn update_profile(&self, profile: &SoftwareDspState) -> Result<(), String> {
        self.config_tx
            .send(profile.clone())
            .map_err(|_| "private DSP worker is no longer accepting profile updates".to_owned())
    }

    pub fn status(&self) -> PrivateDspRuntimeStatus {
        self.status.lock().map_or_else(
            |_| {
                PrivateDspRuntimeStatus::new(
                    PrivateDspRuntimeState::Error,
                    "",
                    "private DSP status lock poisoned",
                )
            },
            |status| status.clone(),
        )
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        kill_capture_child(&self.child);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        if let Some(module_id) = self.module_id.take() {
            let _ = unload_private_pipe_source(module_id);
        }
        drop(self.fifo_anchor.take());
        let _ = fs::remove_file(&self.fifo_path);
    }
}

impl Drop for PrivateDspRuntime {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn is_beacn_source(source_name: &str) -> bool {
    let lower = source_name.to_ascii_lowercase();
    lower.contains("beacn")
        && !lower.contains(PRIVATE_SOURCE_NAME)
        && !lower.contains("aetherforge beacn processed")
}

pub fn private_runtime_dir() -> PathBuf {
    env::var_os("XDG_RUNTIME_DIR").map_or_else(
        || env::temp_dir().join("aetherforge-beacn-control"),
        |dir| PathBuf::from(dir).join("aetherforge-beacn-control"),
    )
}

pub fn capture_arguments(raw_source: &str) -> Vec<String> {
    vec![
        "--target".to_owned(),
        raw_source.to_owned(),
        "--raw".to_owned(),
        "--rate".to_owned(),
        PRIVATE_DSP_SAMPLE_RATE_HZ.to_string(),
        "--channels".to_owned(),
        "1".to_owned(),
        "--channel-map".to_owned(),
        "mono".to_owned(),
        "--format".to_owned(),
        "f32".to_owned(),
        "-".to_owned(),
    ]
}

pub fn pipe_source_arguments(fifo_path: &Path) -> Vec<String> {
    vec![
        "load-module".to_owned(),
        "module-pipe-source".to_owned(),
        format!("file={}", fifo_path.display()),
        format!("source_name={PRIVATE_SOURCE_NAME}"),
        format!("source_properties=device.description={PRIVATE_SOURCE_PROPERTY_DESCRIPTION}"),
        "format=float32le".to_owned(),
        format!("rate={PRIVATE_DSP_SAMPLE_RATE_HZ}"),
        "channels=1".to_owned(),
        "channel_map=mono".to_owned(),
    ]
}

fn require_command(command: &str) -> Result<(), String> {
    let path = env::var_os("PATH").unwrap_or_default();
    let found = env::split_paths(&path).any(|directory| directory.join(command).is_file());
    if found {
        Ok(())
    } else {
        Err(format!("required private-DSP helper is missing: {command}"))
    }
}

fn create_fifo(path: &Path) -> Result<(), String> {
    let status = Command::new("mkfifo")
        .arg("-m")
        .arg("600")
        .arg(path)
        .status()
        .map_err(|error| format!("create private DSP FIFO: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("mkfifo failed with {status}"))
    }
}

fn remove_own_fifo(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(path).map_err(|error| format!("remove stale private DSP FIFO: {error}"))
}

fn load_private_pipe_source(fifo_path: &Path) -> Result<u32, String> {
    let output = Command::new("pactl")
        .args(pipe_source_arguments(fifo_path))
        .output()
        .map_err(|error| format!("load private PipeWire source: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "load private PipeWire source failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .map_err(|error| format!("parse private pipe-source module id: {error}"))
}

fn unload_private_pipe_source(module_id: u32) -> Result<(), String> {
    let status = Command::new("pactl")
        .arg("unload-module")
        .arg(module_id.to_string())
        .status()
        .map_err(|error| format!("unload private PipeWire source: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("pactl unload-module failed with {status}"))
    }
}

fn run_private_dsp_worker(
    raw_source: &str,
    fifo_path: &Path,
    mut profile: SoftwareDspState,
    config_rx: mpsc::Receiver<SoftwareDspState>,
    stop: &Arc<AtomicBool>,
    child_slot: &Arc<Mutex<Option<Child>>>,
    status: &Arc<Mutex<PrivateDspRuntimeStatus>>,
) -> Result<(), String> {
    let mut capture = Command::new("pw-record")
        .args(capture_arguments(raw_source))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("start targeted BEACN pw-record capture: {error}"))?;
    let mut capture_stdout = capture
        .stdout
        .take()
        .ok_or_else(|| "pw-record stdout was not captured".to_owned())?;
    if let Ok(mut slot) = child_slot.lock() {
        *slot = Some(capture);
    }

    let mut fifo = OpenOptions::new()
        .write(true)
        .open(fifo_path)
        .map_err(|error| format!("open private DSP FIFO: {error}"))?;

    set_status(
        status,
        PrivateDspRuntimeStatus::new(
            PrivateDspRuntimeState::Running,
            raw_source,
            format!("publishing {PRIVATE_SOURCE_DESCRIPTION}"),
        ),
    );

    let mut engine = PrivateDspEngine::new(PRIVATE_DSP_SAMPLE_RATE_HZ as f32);
    let mut input_bytes = [0_u8; PRIVATE_DSP_BLOCK_FRAMES * 4];
    let mut output_bytes = [0_u8; PRIVATE_DSP_BLOCK_FRAMES * 4];
    let mut samples = [0.0_f32; PRIVATE_DSP_BLOCK_FRAMES];

    while !stop.load(Ordering::Acquire) {
        while let Ok(next_profile) = config_rx.try_recv() {
            profile = next_profile;
        }

        match capture_stdout.read_exact(&mut input_bytes) {
            Ok(()) => {}
            Err(_) if stop.load(Ordering::Acquire) => break,
            Err(error) => return Err(format!("private BEACN capture stopped: {error}")),
        }

        for (index, chunk) in input_bytes.as_chunks::<4>().0.iter().enumerate() {
            samples[index] = f32::from_le_bytes(*chunk);
        }
        engine.process_mono_block(&mut samples, &profile);
        for (index, sample) in samples.iter().copied().enumerate() {
            let bytes = sample.to_le_bytes();
            let start = index * 4;
            output_bytes[start..start + 4].copy_from_slice(&bytes);
        }
        fifo.write_all(&output_bytes)
            .map_err(|error| format!("write processed BEACN audio to private source: {error}"))?;
    }

    kill_capture_child(child_slot);
    Ok(())
}

fn kill_capture_child(child_slot: &Arc<Mutex<Option<Child>>>) {
    if let Ok(mut slot) = child_slot.lock() {
        if let Some(child) = slot.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        *slot = None;
    }
}

fn set_status(status: &Arc<Mutex<PrivateDspRuntimeStatus>>, value: PrivateDspRuntimeStatus) {
    if let Ok(mut current) = status.lock() {
        *current = value;
    }
}

fn status_state(status: &Arc<Mutex<PrivateDspRuntimeStatus>>) -> PrivateDspRuntimeState {
    status
        .lock()
        .map_or(PrivateDspRuntimeState::Error, |value| value.state)
}
