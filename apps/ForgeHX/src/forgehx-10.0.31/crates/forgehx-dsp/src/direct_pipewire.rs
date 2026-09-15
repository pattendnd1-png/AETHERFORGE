//! Direct PipeWire microphone path for ForgeHX.
//!
//! This module deliberately does not build a sink/monitor/loopback chain. One
//! input stream targets the physical HyperX capture node by stable `node.name`;
//! post-DSP frames are sent nonblocking to AetherStream, which alone publishes the system microphone.

use crate::aetherstream_bridge::AetherStreamMicBridge;
use crate::runtime::RuntimeControl;
use crate::{wait_for_source_node_id, VoiceProcessingEngine, FRAME_SAMPLES};
use forgehx_core::{
    MicrophoneDspConfig, NoiseSceneTelemetry, VoiceIsolationTelemetry, VoicePilotTelemetry,
};
use pipewire as pw;
use pw::{properties::properties, spa};
use spa::pod::Pod;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

const SAMPLE_BYTES: usize = std::mem::size_of::<f32>();

pub struct DirectRuntimeShared {
    pub config: Arc<RwLock<MicrophoneDspConfig>>,
    pub telemetry: Arc<Mutex<VoicePilotTelemetry>>,
    pub isolation_telemetry: Arc<Mutex<VoiceIsolationTelemetry>>,
    pub noise_scene_telemetry: Arc<Mutex<NoiseSceneTelemetry>>,
    pub(crate) control: Arc<Mutex<RuntimeControl>>,
    pub completed_voiceprint: Arc<Mutex<Option<Vec<f32>>>>,
    pub latest_reference: Arc<Mutex<Option<[f32; FRAME_SAMPLES]>>>,
    pub stop: Arc<AtomicBool>,
}

struct CaptureState {
    engine: VoiceProcessingEngine,
    active_config: MicrophoneDspConfig,
    pending_input: VecDeque<f32>,
    bridge: AetherStreamMicBridge,
    shared: DirectRuntimeShared,
}

pub fn run_direct_pipewire(
    raw_source: &str,
    processed_source: &str,
    shared: DirectRuntimeShared,
) -> Result<(), String> {
    pw::init();

    let initial = shared
        .config
        .read()
        .map_err(|_| "ForgeHX DSP config lock poisoned".to_owned())?
        .clone();
    let engine = VoiceProcessingEngine::new(initial.clone()).map_err(|error| error.to_string())?;

    let mainloop = pw::main_loop::MainLoopRc::new(None)
        .map_err(|error| format!("cannot create ForgeHX PipeWire main loop: {error}"))?;
    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|error| format!("cannot create ForgeHX PipeWire context: {error}"))?;
    let core = context
        .connect_rc(None)
        .map_err(|error| format!("cannot connect ForgeHX directly to PipeWire: {error}"))?;

    let bridge = AetherStreamMicBridge::new()?;

    // Resolve the current registry ID from the stable physical node.name on every
    // connection attempt. PipeWire object IDs may change after unplug/replug, while
    // the persisted HyperX node.name remains the reconnect identity.
    let raw_source_node_id =
        wait_for_source_node_id(raw_source).map_err(|error| error.to_string())?;

    // Physical capture is targeted directly by PipeWire object ID. PipeWire performs
    // any device-format conversion before these F32LE mono frames reach DSP. Give the
    // client-side capture stream its own stable diagnostic node name so the host
    // verifier can prove the hardware -> ForgeHX edge without creating another
    // application-facing source or sink.
    let capture_node_name = format!("{processed_source}_direct_capture");
    let capture = pw::stream::StreamRc::new(
        core,
        "ForgeHX HyperX direct capture",
        properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Communication",
            *pw::keys::NODE_NAME => capture_node_name.as_str(),
            *pw::keys::NODE_DESCRIPTION => "ForgeHX Direct HyperX Capture",
            *pw::keys::AUDIO_RATE => "48000",
            *pw::keys::AUDIO_CHANNELS => "1",
        },
    )
    .map_err(|error| format!("cannot create direct HyperX capture stream: {error}"))?;

    let stop_for_timer = Arc::clone(&shared.stop);
    let capture_state = CaptureState {
        engine,
        active_config: initial,
        pending_input: VecDeque::with_capacity(FRAME_SAMPLES * 2),
        bridge,
        shared,
    };
    let _capture_listener = capture
        .add_local_listener_with_user_data(capture_state)
        .process(move |stream, state| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            let Some(data) = datas.first_mut() else {
                return;
            };
            let chunk = data.chunk();
            let offset = chunk.offset() as usize;
            let size = chunk.size() as usize;
            let Some(bytes) = data.data() else { return };
            let end = offset.saturating_add(size).min(bytes.len());
            if offset >= end {
                return;
            }

            let (samples, _) = bytes[offset..end].as_chunks::<SAMPLE_BYTES>();
            for sample in samples {
                state.pending_input.push_back(f32::from_le_bytes(*sample));
            }

            while state.pending_input.len() >= FRAME_SAMPLES {
                let mut frame = [0.0f32; FRAME_SAMPLES];
                for sample in &mut frame {
                    *sample = state.pending_input.pop_front().unwrap_or(0.0);
                }
                process_frame(state, &frame);
            }
        })
        .register()
        .map_err(|error| format!("cannot register direct HyperX capture listener: {error}"))?;

    let capture_format = audio_format_param()?;
    let mut capture_params = [Pod::from_bytes(&capture_format)
        .ok_or_else(|| "invalid direct HyperX capture format".to_owned())?];
    capture
        .connect(
            spa::utils::Direction::Input,
            Some(raw_source_node_id),
            pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
            &mut capture_params,
        )
        .map_err(|error| {
            format!("cannot connect ForgeHX directly to HyperX mic {raw_source}: {error}")
        })?;

    tracing::info!(
        raw_source,
        processed_source,
        raw_source_node_id,
        capture_node = %capture_node_name,
        "ForgeHX direct mic path connected"
    );

    let loop_for_stop = mainloop.clone();
    let _stop_timer = mainloop.loop_().add_timer(move |_| {
        if stop_for_timer.load(Ordering::Acquire) {
            loop_for_stop.quit();
        }
    });
    let _ = _stop_timer.update_timer(
        Some(Duration::from_millis(50)),
        Some(Duration::from_millis(50)),
    );

    mainloop.run();
    Ok(())
}

fn process_frame(state: &mut CaptureState, frame: &[f32; FRAME_SAMPLES]) {
    if let Ok(mut pending) = state.shared.control.lock() {
        match std::mem::take(&mut *pending) {
            RuntimeControl::None => {}
            RuntimeControl::StartEnrollment => state.engine.begin_speaker_enrollment(),
            RuntimeControl::CancelEnrollment => state.engine.cancel_speaker_enrollment(),
            RuntimeControl::ForgetVoice => {
                state.engine.forget_speaker_voiceprint();
                state.active_config.speaker_lock.voiceprint.clear();
                if let Ok(mut config) = state.shared.config.write() {
                    config.speaker_lock.voiceprint.clear();
                }
                if let Ok(mut completed) = state.shared.completed_voiceprint.lock() {
                    *completed = Some(Vec::new());
                }
            }
        }
    }

    if let Ok(config) = state.shared.config.read() {
        if *config != state.active_config && state.engine.update_config(config.clone()).is_ok() {
            state.active_config = config.clone();
        }
    }

    let reference = state
        .shared
        .latest_reference
        .lock()
        .ok()
        .and_then(|slot| *slot);
    let processed = match state.engine.process_10ms(frame, reference.as_ref()) {
        Ok(processed) => processed,
        Err(error) => {
            tracing::error!(
                %error,
                "ForgeHX DSP frame rejected; raw microphone bypass forbidden"
            );
            [0.0f32; FRAME_SAMPLES]
        }
    };

    if let Some(voiceprint) = state.engine.take_completed_voiceprint() {
        state.active_config.speaker_lock.voiceprint = voiceprint.clone();
        if let Ok(mut config) = state.shared.config.write() {
            config.speaker_lock.voiceprint = voiceprint.clone();
        }
        if let Ok(mut completed) = state.shared.completed_voiceprint.lock() {
            *completed = Some(voiceprint);
        }
    }

    state.bridge.send_processed_frame(&processed);

    if let Ok(mut telemetry) = state.shared.telemetry.lock() {
        *telemetry = state.engine.telemetry();
    }
    if let Ok(mut telemetry) = state.shared.isolation_telemetry.lock() {
        *telemetry = state.engine.voice_isolation_telemetry();
    }
    if let Ok(mut telemetry) = state.shared.noise_scene_telemetry.lock() {
        *telemetry = state.engine.noise_scene_telemetry();
    }
}

fn audio_format_param() -> Result<Vec<u8>, String> {
    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    audio_info.set_rate(48_000);
    audio_info.set_channels(1);

    let object = pw::spa::pod::Object {
        type_: pw::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: pw::spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };
    pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(object),
    )
    .map(|serialized| serialized.0.into_inner())
    .map_err(|error| format!("cannot serialize ForgeHX direct audio format: {error:?}"))
}
