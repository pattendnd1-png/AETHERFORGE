use crate::aetherstream_bridge::AETHERSTREAM_SYSTEM_SOURCE_NAME;
use crate::direct_pipewire::{run_direct_pipewire, DirectRuntimeShared};
use crate::FRAME_SAMPLES;
use forgehx_core::{
    MicrophoneDspConfig, MicrophoneMonitorState, NoiseSceneTelemetry, VoiceIsolationTelemetry,
    VoicePilotTelemetry,
};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use tracing::warn;

const FRAME_BYTES: usize = FRAME_SAMPLES * std::mem::size_of::<f32>();
const FORGEHX_DSP_RECONNECT_MS: u64 = 250;

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorRuntimeConfig {
    pub enabled: bool,
    pub monitor_target: Option<String>,
    pub level_percent: f32,
}

impl Default for MonitorRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            monitor_target: None,
            level_percent: 50.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum RuntimeControl {
    #[default]
    None,
    StartEnrollment,
    CancelEnrollment,
    ForgetVoice,
}

#[derive(Debug, Clone, Default)]
pub struct DspRuntimeRegistry {
    inner: Arc<Mutex<HashMap<String, RuntimeHandle>>>,
}

#[derive(Debug)]
struct RuntimeHandle {
    config: Arc<RwLock<MicrophoneDspConfig>>,
    telemetry: Arc<Mutex<VoicePilotTelemetry>>,
    isolation_telemetry: Arc<Mutex<VoiceIsolationTelemetry>>,
    noise_scene_telemetry: Arc<Mutex<NoiseSceneTelemetry>>,
    control: Arc<Mutex<RuntimeControl>>,
    completed_voiceprint: Arc<Mutex<Option<Vec<f32>>>>,
    monitor_config: Arc<RwLock<MonitorRuntimeConfig>>,
    monitor_state: Arc<Mutex<MicrophoneMonitorState>>,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    monitor_join: Option<JoinHandle<()>>,
}

impl DspRuntimeRegistry {
    pub fn start(
        &self,
        key: &str,
        raw_source: &str,
        processed_source: &str,
        config: MicrophoneDspConfig,
    ) -> Result<(), String> {
        let previous_monitor = self
            .inner
            .lock()
            .ok()
            .and_then(|registry| {
                registry
                    .get(key)
                    .and_then(|handle| handle.monitor_config.read().ok().map(|value| value.clone()))
            })
            .unwrap_or_default();
        self.stop(key);

        let config_slot = Arc::new(RwLock::new(config));
        let telemetry = Arc::new(Mutex::new(VoicePilotTelemetry::default()));
        let isolation_telemetry = Arc::new(Mutex::new(VoiceIsolationTelemetry::default()));
        let noise_scene_telemetry = Arc::new(Mutex::new(NoiseSceneTelemetry::default()));
        let control = Arc::new(Mutex::new(RuntimeControl::None));
        let completed_voiceprint = Arc::new(Mutex::new(None));
        let monitor_config = Arc::new(RwLock::new(previous_monitor.clone()));
        let monitor_state = Arc::new(Mutex::new(MicrophoneMonitorState {
            enabled: previous_monitor.enabled,
            active: false,
            level_percent: previous_monitor.level_percent,
            sink_node_name: previous_monitor.monitor_target.clone(),
            error: None,
        }));
        let stop = Arc::new(AtomicBool::new(false));

        let monitor_processed_source = AETHERSTREAM_SYSTEM_SOURCE_NAME.to_owned();
        let monitor_thread_config = Arc::clone(&monitor_config);
        let monitor_thread_state = Arc::clone(&monitor_state);
        let monitor_thread_stop = Arc::clone(&stop);
        let monitor_join = thread::Builder::new()
            .name(format!("forgehx-mic-monitor-{key}"))
            .spawn(move || {
                run_monitor_worker(
                    &monitor_processed_source,
                    monitor_thread_config,
                    monitor_thread_state,
                    monitor_thread_stop,
                )
            })
            .map_err(|e| format!("cannot start ForgeHX mic monitor worker: {e}"))?;

        let raw_source = raw_source.to_owned();
        let processed_source = processed_source.to_owned();
        let worker_shared = RuntimeWorkerShared {
            config: Arc::clone(&config_slot),
            telemetry: Arc::clone(&telemetry),
            isolation_telemetry: Arc::clone(&isolation_telemetry),
            noise_scene_telemetry: Arc::clone(&noise_scene_telemetry),
            control: Arc::clone(&control),
            completed_voiceprint: Arc::clone(&completed_voiceprint),
            stop: Arc::clone(&stop),
        };
        let join = thread::Builder::new()
            .name(format!("forgehx-dsp-{key}"))
            .spawn(move || {
                run_worker(&raw_source, &processed_source, worker_shared);
            })
            .map_err(|e| format!("cannot start ForgeHX direct mic DSP worker: {e}"))?;

        let handle = RuntimeHandle {
            config: config_slot,
            telemetry,
            isolation_telemetry,
            noise_scene_telemetry,
            control,
            completed_voiceprint,
            monitor_config,
            monitor_state,
            stop,
            join: Some(join),
            monitor_join: Some(monitor_join),
        };
        self.inner
            .lock()
            .map_err(|_| "ForgeHX DSP runtime registry poisoned".to_owned())?
            .insert(key.to_owned(), handle);
        Ok(())
    }

    pub fn update(&self, key: &str, config: MicrophoneDspConfig) -> Result<(), String> {
        let mut registry = self
            .inner
            .lock()
            .map_err(|_| "ForgeHX DSP runtime registry poisoned".to_owned())?;
        let handle = registry
            .get_mut(key)
            .ok_or_else(|| "ForgeHX mic DSP runtime is not active".to_owned())?;
        *handle
            .config
            .write()
            .map_err(|_| "ForgeHX DSP config lock poisoned".to_owned())? = config;
        Ok(())
    }

    pub fn telemetry(&self, key: &str) -> Option<VoicePilotTelemetry> {
        let registry = self.inner.lock().ok()?;
        let handle = registry.get(key)?;
        handle.telemetry.lock().ok().map(|value| value.clone())
    }

    pub fn voice_isolation_telemetry(&self, key: &str) -> Option<VoiceIsolationTelemetry> {
        let registry = self.inner.lock().ok()?;
        let handle = registry.get(key)?;
        handle
            .isolation_telemetry
            .lock()
            .ok()
            .map(|value| value.clone())
    }

    pub fn noise_scene_telemetry(&self, key: &str) -> Option<NoiseSceneTelemetry> {
        let registry = self.inner.lock().ok()?;
        let handle = registry.get(key)?;
        handle
            .noise_scene_telemetry
            .lock()
            .ok()
            .map(|value| value.clone())
    }

    pub fn start_enrollment(&self, key: &str) -> Result<(), String> {
        self.set_control(key, RuntimeControl::StartEnrollment)
    }

    pub fn cancel_enrollment(&self, key: &str) -> Result<(), String> {
        self.set_control(key, RuntimeControl::CancelEnrollment)
    }

    pub fn forget_voice(&self, key: &str) -> Result<(), String> {
        self.set_control(key, RuntimeControl::ForgetVoice)
    }

    pub fn take_completed_voiceprint(&self, key: &str) -> Option<Vec<f32>> {
        let completed_voiceprint = {
            let registry = self.inner.lock().ok()?;
            let handle = registry.get(key)?;
            Arc::clone(&handle.completed_voiceprint)
        };
        let voiceprint = completed_voiceprint.lock().ok()?.take();
        voiceprint
    }

    fn set_control(&self, key: &str, command: RuntimeControl) -> Result<(), String> {
        let registry = self
            .inner
            .lock()
            .map_err(|_| "ForgeHX DSP runtime registry poisoned".to_owned())?;
        let handle = registry
            .get(key)
            .ok_or_else(|| "ForgeHX mic DSP runtime is not active".to_owned())?;
        *handle
            .control
            .lock()
            .map_err(|_| "ForgeHX DSP runtime control poisoned".to_owned())? = command;
        Ok(())
    }

    pub fn set_monitor(
        &self,
        key: &str,
        enabled: bool,
        monitor_target: Option<String>,
        level_percent: f32,
    ) -> Result<(), String> {
        if !(0.0..=100.0).contains(&level_percent) {
            return Err("microphone monitor level must be 0..=100 percent".into());
        }
        if enabled && monitor_target.is_none() {
            return Err("wired headphone monitor requires a wired PipeWire sink".into());
        }
        let registry = self
            .inner
            .lock()
            .map_err(|_| "ForgeHX DSP runtime registry poisoned".to_owned())?;
        let handle = registry
            .get(key)
            .ok_or_else(|| "ForgeHX mic DSP runtime is not active".to_owned())?;
        let next = MonitorRuntimeConfig {
            enabled,
            monitor_target: monitor_target.clone(),
            level_percent,
        };
        *handle
            .monitor_config
            .write()
            .map_err(|_| "ForgeHX monitor config lock poisoned".to_owned())? = next;
        if let Ok(mut state) = handle.monitor_state.lock() {
            state.enabled = enabled;
            state.level_percent = level_percent;
            state.sink_node_name = monitor_target;
            if !enabled {
                state.active = false;
                state.error = None;
            }
        }
        Ok(())
    }

    pub fn monitor_state(&self, key: &str) -> Option<MicrophoneMonitorState> {
        let registry = self.inner.lock().ok()?;
        let handle = registry.get(key)?;
        let state = handle.monitor_state.lock().ok()?.clone();
        Some(state)
    }

    pub fn is_active(&self, key: &str) -> bool {
        self.inner
            .lock()
            .map(|registry| registry.contains_key(key))
            .unwrap_or(false)
    }

    pub fn stop(&self, key: &str) {
        let handle = self
            .inner
            .lock()
            .ok()
            .and_then(|mut registry| registry.remove(key));
        if let Some(mut handle) = handle {
            handle.stop.store(true, Ordering::Release);
            // Do not block the caller while pw-cat may be waiting on a capture read.
            // Dropping the JoinHandle detaches the worker; it observes `stop` on the
            // next frame and tears down its child processes itself.
            let _ = handle.join.take();
            let _ = handle.monitor_join.take();
        }
    }
}

struct RuntimeWorkerShared {
    config: Arc<RwLock<MicrophoneDspConfig>>,
    telemetry: Arc<Mutex<VoicePilotTelemetry>>,
    isolation_telemetry: Arc<Mutex<VoiceIsolationTelemetry>>,
    noise_scene_telemetry: Arc<Mutex<NoiseSceneTelemetry>>,
    control: Arc<Mutex<RuntimeControl>>,
    completed_voiceprint: Arc<Mutex<Option<Vec<f32>>>>,
    stop: Arc<AtomicBool>,
}

fn run_worker(raw_source: &str, processed_source: &str, shared: RuntimeWorkerShared) {
    let RuntimeWorkerShared {
        config,
        telemetry,
        isolation_telemetry,
        noise_scene_telemetry,
        control,
        completed_voiceprint,
        stop,
    } = shared;
    let latest_reference = Arc::new(Mutex::new(None::<[f32; FRAME_SAMPLES]>));
    let reference_stop = Arc::clone(&stop);
    let reference_slot = Arc::clone(&latest_reference);
    let reference_thread = thread::Builder::new()
        .name("forgehx-aec-reference".into())
        .spawn(move || {
            reconnect_reference_session(reference_stop, reference_slot);
        })
        .ok();

    while !stop.load(Ordering::Acquire) {
        let shared = DirectRuntimeShared {
            config: Arc::clone(&config),
            telemetry: Arc::clone(&telemetry),
            isolation_telemetry: Arc::clone(&isolation_telemetry),
            noise_scene_telemetry: Arc::clone(&noise_scene_telemetry),
            control: Arc::clone(&control),
            completed_voiceprint: Arc::clone(&completed_voiceprint),
            latest_reference: Arc::clone(&latest_reference),
            stop: Arc::clone(&stop),
        };
        let _ = run_direct_pipewire(raw_source, processed_source, shared);
        if !stop.load(Ordering::Acquire) {
            thread::sleep(std::time::Duration::from_millis(FORGEHX_DSP_RECONNECT_MS));
        }
    }

    let _ = reference_thread;
}

fn run_monitor_worker(
    processed_source: &str,
    config_slot: Arc<RwLock<MonitorRuntimeConfig>>,
    state_slot: Arc<Mutex<MicrophoneMonitorState>>,
    stop: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Acquire) {
        let config = match config_slot.read() {
            Ok(value) => value.clone(),
            Err(_) => return,
        };
        if !config.enabled {
            if let Ok(mut state) = state_slot.lock() {
                state.enabled = false;
                state.active = false;
                state.level_percent = config.level_percent;
                state.sink_node_name = config.monitor_target.clone();
                state.error = None;
            }
            thread::sleep(std::time::Duration::from_millis(FORGEHX_DSP_RECONNECT_MS));
            continue;
        }
        let Some(monitor_target) = config.monitor_target.clone() else {
            if let Ok(mut state) = state_slot.lock() {
                state.enabled = true;
                state.active = false;
                state.error = Some("wired headphone monitor has no eligible wired sink".into());
            }
            thread::sleep(std::time::Duration::from_millis(FORGEHX_DSP_RECONNECT_MS));
            continue;
        };

        let mut capture = match spawn_record(processed_source) {
            Ok(child) => child,
            Err(error) => {
                if let Ok(mut state) = state_slot.lock() {
                    state.active = false;
                    state.error = Some(error);
                }
                thread::sleep(std::time::Duration::from_millis(FORGEHX_DSP_RECONNECT_MS));
                continue;
            }
        };
        let mut playback = match spawn_playback(&monitor_target) {
            Ok(child) => child,
            Err(error) => {
                let _ = capture.kill();
                if let Ok(mut state) = state_slot.lock() {
                    state.active = false;
                    state.error = Some(error);
                }
                thread::sleep(std::time::Duration::from_millis(FORGEHX_DSP_RECONNECT_MS));
                continue;
            }
        };
        let Some(mut capture_stdout) = capture.stdout.take() else {
            let _ = capture.kill();
            let _ = playback.kill();
            if let Ok(mut state) = state_slot.lock() {
                state.active = false;
                state.error = Some("ForgeHX monitor capture has no stdout".into());
            }
            continue;
        };
        let Some(mut playback_stdin) = playback.stdin.take() else {
            let _ = capture.kill();
            let _ = playback.kill();
            if let Ok(mut state) = state_slot.lock() {
                state.active = false;
                state.error = Some("ForgeHX monitor playback has no stdin".into());
            }
            continue;
        };
        if let Ok(mut state) = state_slot.lock() {
            state.enabled = true;
            state.active = true;
            state.level_percent = config.level_percent;
            state.sink_node_name = Some(monitor_target.clone());
            state.error = None;
        }

        while !stop.load(Ordering::Acquire) {
            let current = match config_slot.read() {
                Ok(value) => value.clone(),
                Err(_) => break,
            };
            if current != config {
                break;
            }
            let mut frame = match read_frame(&mut capture_stdout) {
                Ok(frame) => frame,
                Err(error) => {
                    if let Ok(mut state) = state_slot.lock() {
                        state.active = false;
                        state.error = Some(error);
                    }
                    break;
                }
            };
            let gain = (config.level_percent / 100.0).clamp(0.0, 1.0);
            for sample in &mut frame {
                *sample *= gain;
            }
            if let Err(error) = write_frame(&mut playback_stdin, &frame) {
                if let Ok(mut state) = state_slot.lock() {
                    state.active = false;
                    state.error = Some(error);
                }
                break;
            }
        }
        let _ = capture.kill();
        let _ = playback.kill();
        if let Ok(mut state) = state_slot.lock() {
            state.active = false;
        }
        if !stop.load(Ordering::Acquire) {
            thread::sleep(std::time::Duration::from_millis(FORGEHX_DSP_RECONNECT_MS));
        }
    }
}

fn reconnect_reference_session(
    stop: Arc<AtomicBool>,
    latest_reference: Arc<Mutex<Option<[f32; FRAME_SAMPLES]>>>,
) {
    while !stop.load(Ordering::Acquire) {
        if let Ok(mut slot) = latest_reference.lock() {
            *slot = None;
        }
        match spawn_reference_record() {
            Ok(mut reference) => {
                if let Some(mut stdout) = reference.stdout.take() {
                    while !stop.load(Ordering::Acquire) {
                        match read_frame(&mut stdout) {
                            Ok(frame) => {
                                if let Ok(mut slot) = latest_reference.lock() {
                                    *slot = Some(frame);
                                }
                            }
                            Err(error) => {
                                if let Ok(mut slot) = latest_reference.lock() {
                                    *slot = None;
                                }
                                warn!(%error, "ForgeHX AEC reference capture ended; reconnecting to default sink monitor");
                                break;
                            }
                        }
                    }
                } else {
                    warn!("ForgeHX AEC reference capture has no stdout; reconnecting");
                }
                let _ = reference.kill();
            }
            Err(error) => {
                warn!(%error, "ForgeHX AEC reference capture unavailable; retrying default sink monitor");
            }
        }
        if !stop.load(Ordering::Acquire) {
            thread::sleep(std::time::Duration::from_millis(FORGEHX_DSP_RECONNECT_MS));
        }
    }
}

fn monitor_source_name(sink: &str) -> Result<String, String> {
    let sink = sink.trim();
    if sink.is_empty() {
        return Err("default audio sink name is empty".into());
    }
    Ok(format!("{sink}.monitor"))
}

fn default_sink_monitor_source() -> Result<String, String> {
    let output = Command::new("pactl")
        .arg("get-default-sink")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("cannot query default audio sink with pactl: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(format!("pactl get-default-sink failed: {stderr}"));
    }
    let sink = String::from_utf8_lossy(&output.stdout);
    monitor_source_name(&sink)
}

fn spawn_reference_record() -> Result<Child, String> {
    let target = default_sink_monitor_source()?;
    Command::new("parec")
        .arg(format!("--device={target}"))
        .args(["--rate=48000", "--channels=1", "--format=float32le"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("cannot start ForgeHX AEC reference capture for {target}: {e}"))
}

fn spawn_record(target: &str) -> Result<Child, String> {
    Command::new("pw-cat")
        .args([
            "--record",
            "--target",
            target,
            "--rate",
            "48000",
            "--channels",
            "1",
            "--format",
            "f32",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot start pw-cat record for {target}: {e}"))
}

fn spawn_playback(target: &str) -> Result<Child, String> {
    Command::new("pw-cat")
        .args([
            "--playback",
            "--target",
            target,
            "--rate",
            "48000",
            "--channels",
            "1",
            "--format",
            "f32",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot start pw-cat playback for {target}: {e}"))
}

fn read_frame(stdout: &mut ChildStdout) -> Result<[f32; FRAME_SAMPLES], String> {
    let mut bytes = [0u8; FRAME_BYTES];
    stdout.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    let mut frame = [0.0f32; FRAME_SAMPLES];
    for (index, chunk) in bytes.as_chunks::<4>().0.iter().enumerate() {
        frame[index] = f32::from_le_bytes(*chunk);
    }
    Ok(frame)
}

fn write_frame(writer: &mut impl Write, frame: &[f32; FRAME_SAMPLES]) -> Result<(), String> {
    let mut bytes = [0u8; FRAME_BYTES];
    for (index, sample) in frame.iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&sample.to_le_bytes());
    }
    writer.write_all(&bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod noise_runtime_tests {
    use super::*;

    #[test]
    fn noise_scene_telemetry_defaults_absent_without_runtime() {
        let registry = DspRuntimeRegistry::default();
        assert!(registry.noise_scene_telemetry("missing").is_none());
    }

    #[test]
    fn default_sink_monitor_name_targets_monitor_source() {
        assert_eq!(
            monitor_source_name("bluez_output.50_5E_5C_6E_16_17.1\n").unwrap(),
            "bluez_output.50_5E_5C_6E_16_17.1.monitor"
        );
    }

    #[test]
    fn empty_default_sink_name_is_rejected() {
        assert!(monitor_source_name("  \n").is_err());
    }
}
