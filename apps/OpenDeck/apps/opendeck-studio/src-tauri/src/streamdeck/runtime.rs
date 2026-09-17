use super::protocol::{
    ELGATO_VID, INPUT_SIZE, ParsedInputReport, STREAM_DECK_PLUS_PID, TouchEvent, brightness_report,
    button_image_reports, parse_input_report, show_logo_report, window_image_reports,
};
use super::render::{RenderedDeck, render_workspace};
use crate::editor::Workspace;
use crate::qualification;
use hidapi::{HidApi, HidDevice};
use serde::Serialize;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const SCAN_INTERVAL: Duration = Duration::from_secs(1);
const IDLE_WAIT: Duration = Duration::from_millis(75);
const READ_TIMEOUT_MS: i32 = 50;
const DEFAULT_BRIGHTNESS: u8 = 50;

pub(crate) const HARDWARE_INPUT_EVENT: &str = "opendeck://hardware-input";
pub(crate) const HARDWARE_STATUS_EVENT: &str = "opendeck://hardware-status";

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StreamDeckStatus {
    pub state: String,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub message: String,
}

impl StreamDeckStatus {
    fn disconnected(message: impl Into<String>) -> Self {
        Self {
            state: "disconnected".into(),
            model: None,
            serial: None,
            message: message.into(),
        }
    }

    fn connecting() -> Self {
        Self {
            state: "connecting".into(),
            model: Some("Stream Deck +".into()),
            serial: None,
            message: "Opening Stream Deck +…".into(),
        }
    }

    fn connected(
        model: Option<String>,
        serial: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            state: "connected".into(),
            model: Some(model.unwrap_or_else(|| "Stream Deck +".into())),
            serial,
            message: message.into(),
        }
    }

    fn permission_denied(message: impl Into<String>) -> Self {
        Self {
            state: "permissionDenied".into(),
            model: Some("Stream Deck +".into()),
            serial: None,
            message: message.into(),
        }
    }

    fn io_error(message: impl Into<String>) -> Self {
        Self {
            state: "ioError".into(),
            model: Some("Stream Deck +".into()),
            serial: None,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub(crate) enum StreamDeckInputEvent {
    KeyDown {
        index: u8,
    },
    KeyUp {
        index: u8,
    },
    DialDown {
        index: u8,
    },
    DialUp {
        index: u8,
    },
    DialRotate {
        index: u8,
        ticks: i8,
        pressed: bool,
    },
    TouchTap {
        x: u16,
        y: u16,
        region: u8,
    },
    TouchPress {
        x: u16,
        y: u16,
        region: u8,
    },
    TouchFlick {
        #[serde(rename = "startX")]
        start_x: u16,
        #[serde(rename = "startY")]
        start_y: u16,
        #[serde(rename = "endX")]
        end_x: u16,
        #[serde(rename = "endY")]
        end_y: u16,
        region: u8,
    },
}

#[derive(Default)]
struct InputState {
    keys: [bool; 8],
    dials: [bool; 4],
}

impl InputState {
    fn apply(&mut self, report: ParsedInputReport) -> Vec<StreamDeckInputEvent> {
        match report {
            ParsedInputReport::Keys(next) => {
                let mut events = Vec::new();
                for (index, (&previous, &current)) in self.keys.iter().zip(next.iter()).enumerate()
                {
                    if previous != current {
                        events.push(if current {
                            StreamDeckInputEvent::KeyDown { index: index as u8 }
                        } else {
                            StreamDeckInputEvent::KeyUp { index: index as u8 }
                        });
                    }
                }
                self.keys = next;
                events
            }
            ParsedInputReport::DialButtons(next) => {
                let mut events = Vec::new();
                for (index, (&previous, &current)) in self.dials.iter().zip(next.iter()).enumerate()
                {
                    if previous != current {
                        events.push(if current {
                            StreamDeckInputEvent::DialDown { index: index as u8 }
                        } else {
                            StreamDeckInputEvent::DialUp { index: index as u8 }
                        });
                    }
                }
                self.dials = next;
                events
            }
            ParsedInputReport::DialRotate(ticks) => ticks
                .into_iter()
                .enumerate()
                .filter_map(|(index, ticks)| {
                    (ticks != 0).then_some(StreamDeckInputEvent::DialRotate {
                        index: index as u8,
                        ticks,
                        pressed: self.dials[index],
                    })
                })
                .collect(),
            ParsedInputReport::Touch(touch) => vec![match touch {
                TouchEvent::Tap { x, y, region } => StreamDeckInputEvent::TouchTap { x, y, region },
                TouchEvent::Press { x, y, region } => {
                    StreamDeckInputEvent::TouchPress { x, y, region }
                }
                TouchEvent::Flick {
                    start_x,
                    start_y,
                    end_x,
                    end_y,
                    region,
                } => StreamDeckInputEvent::TouchFlick {
                    start_x,
                    start_y,
                    end_x,
                    end_y,
                    region,
                },
            }],
        }
    }
}

trait DeckTransport {
    fn read_timeout(&self, buffer: &mut [u8], timeout_ms: i32) -> Result<usize, String>;
    fn write(&self, report: &[u8]) -> Result<usize, String>;
    fn send_feature_report(&self, report: &[u8]) -> Result<(), String>;
}

struct HidTransport {
    device: HidDevice,
}

struct OpenedDevice {
    transport: HidTransport,
    model: Option<String>,
    serial: Option<String>,
}

impl DeckTransport for HidTransport {
    fn read_timeout(&self, buffer: &mut [u8], timeout_ms: i32) -> Result<usize, String> {
        self.device
            .read_timeout(buffer, timeout_ms)
            .map_err(|error| error.to_string())
    }

    fn write(&self, report: &[u8]) -> Result<usize, String> {
        self.device.write(report).map_err(|error| error.to_string())
    }

    fn send_feature_report(&self, report: &[u8]) -> Result<(), String> {
        self.device
            .send_feature_report(report)
            .map_err(|error| error.to_string())
    }
}

enum RuntimeCommand {
    SyncWorkspace {
        workspace: Box<Workspace>,
        active_app_id: Option<String>,
    },
    SetBrightness(u8),
    Stop,
}

pub(crate) struct StreamDeckService {
    sender: Sender<RuntimeCommand>,
    receiver: Mutex<Option<Receiver<RuntimeCommand>>>,
    status: Arc<Mutex<StreamDeckStatus>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl StreamDeckService {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
            status: Arc::new(Mutex::new(StreamDeckStatus::disconnected(
                "Stream Deck + not connected",
            ))),
            worker: Mutex::new(None),
        }
    }

    pub(crate) fn start(&self, app: AppHandle) -> Result<(), String> {
        let receiver = self
            .receiver
            .lock()
            .map_err(|_| "Stream Deck runtime receiver lock poisoned".to_string())?
            .take()
            .ok_or_else(|| "Stream Deck runtime is already started".to_string())?;
        let status = Arc::clone(&self.status);
        let worker = thread::Builder::new()
            .name("opendeck-streamdeck-plus".into())
            .spawn(move || worker_loop(app, receiver, status))
            .map_err(|error| format!("could not start Stream Deck runtime: {error}"))?;
        *self
            .worker
            .lock()
            .map_err(|_| "Stream Deck runtime worker lock poisoned".to_string())? = Some(worker);
        Ok(())
    }

    pub(crate) fn status(&self) -> StreamDeckStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| StreamDeckStatus::io_error("Stream Deck status lock poisoned"))
    }

    pub(crate) fn sync_workspace(
        &self,
        workspace: Workspace,
        active_app_id: Option<String>,
    ) -> Result<(), String> {
        self.sender
            .send(RuntimeCommand::SyncWorkspace {
                workspace: Box::new(workspace),
                active_app_id,
            })
            .map_err(|_| "Stream Deck runtime is not running".to_string())
    }

    pub(crate) fn set_brightness(&self, percent: u8) -> Result<(), String> {
        if percent > 100 {
            return Err("Stream Deck brightness must be between 0 and 100".into());
        }
        self.sender
            .send(RuntimeCommand::SetBrightness(percent))
            .map_err(|_| "Stream Deck runtime is not running".to_string())
    }

    pub(crate) fn stop(&self) {
        let _ = self.sender.send(RuntimeCommand::Stop);
        if let Ok(mut worker) = self.worker.lock()
            && let Some(handle) = worker.take()
        {
            let _ = handle.join();
        }
    }
}

impl Default for StreamDeckService {
    fn default() -> Self {
        Self::new()
    }
}

fn set_status(app: &AppHandle, shared: &Arc<Mutex<StreamDeckStatus>>, status: StreamDeckStatus) {
    if let Ok(mut target) = shared.lock() {
        *target = status.clone();
    }
    let _ = app.emit(HARDWARE_STATUS_EVENT, status);
}

fn emit_inputs(app: &AppHandle, state: &mut InputState, report: ParsedInputReport) {
    let started = Instant::now();
    for event in state.apply(report) {
        let _ = app.emit(HARDWARE_INPUT_EVENT, event);
    }
    qualification::record_runtime_sample(
        "hidDecodeDispatchMs",
        started.elapsed().as_secs_f64() * 1000.0,
    );
}

fn classify_open_error(error: &str) -> StreamDeckStatus {
    let lower = error.to_ascii_lowercase();
    if lower.contains("permission") || lower.contains("access") {
        StreamDeckStatus::permission_denied(
            "Stream Deck + is visible but cannot be opened. Install the OpenDeck udev rule, reload rules, then reconnect the device.",
        )
    } else {
        StreamDeckStatus::io_error(format!("Could not open Stream Deck +: {error}"))
    }
}

fn open_device() -> Result<Option<OpenedDevice>, String> {
    let api = HidApi::new().map_err(|error| format!("HID initialization failed: {error}"))?;
    let Some(info) = api
        .device_list()
        .find(|info| info.vendor_id() == ELGATO_VID && info.product_id() == STREAM_DECK_PLUS_PID)
    else {
        return Ok(None);
    };
    let model = info.product_string().map(str::to_owned);
    let serial = info.serial_number().map(str::to_owned);
    let device = info.open_device(&api).map_err(|error| error.to_string())?;
    Ok(Some(OpenedDevice {
        transport: HidTransport { device },
        model,
        serial,
    }))
}

fn write_report<T: DeckTransport>(transport: &T, report: &[u8]) -> Result<(), String> {
    let written = transport.write(report)?;
    if written != report.len() {
        return Err(format!(
            "short Stream Deck HID write: {written}/{} bytes",
            report.len()
        ));
    }
    Ok(())
}

fn write_rendered<T: DeckTransport>(transport: &T, rendered: &RenderedDeck) -> Result<(), String> {
    if rendered.keys.len() != 8 {
        return Err(format!(
            "renderer produced {} key frames; expected 8",
            rendered.keys.len()
        ));
    }
    for (index, jpeg) in rendered.keys.iter().enumerate() {
        for report in button_image_reports(index as u8, jpeg)? {
            write_report(transport, &report)?;
        }
    }
    for report in window_image_reports(&rendered.window)? {
        write_report(transport, &report)?;
    }
    Ok(())
}

fn sync_workspace_to_device<T: DeckTransport>(
    transport: &T,
    workspace: &Workspace,
    active_app_id: Option<&str>,
) -> Result<Vec<String>, String> {
    let rendered = render_workspace(workspace, active_app_id)?;
    write_rendered(transport, &rendered)?;
    Ok(rendered.warnings)
}

#[cfg(test)]
fn apply_command<T: DeckTransport>(
    command: RuntimeCommand,
    transport: Option<&T>,
    last_workspace: &mut Option<(Workspace, Option<String>)>,
) -> Result<bool, String> {
    match command {
        RuntimeCommand::SyncWorkspace {
            workspace,
            active_app_id,
        } => {
            if let Some(transport) = transport {
                let _warnings =
                    sync_workspace_to_device(transport, &workspace, active_app_id.as_deref())?;
            }
            *last_workspace = Some((*workspace, active_app_id));
            Ok(true)
        }
        RuntimeCommand::SetBrightness(percent) => {
            if let Some(transport) = transport {
                transport.send_feature_report(&brightness_report(percent)?)?;
            }
            Ok(true)
        }
        RuntimeCommand::Stop => {
            if let Some(transport) = transport {
                let _ = transport.send_feature_report(&show_logo_report());
            }
            Ok(false)
        }
    }
}

fn worker_loop(
    app: AppHandle,
    receiver: Receiver<RuntimeCommand>,
    status: Arc<Mutex<StreamDeckStatus>>,
) {
    let mut transport: Option<HidTransport> = None;
    let mut input_state = InputState::default();
    let mut last_workspace: Option<(Workspace, Option<String>)> = None;
    let mut next_scan = Instant::now();
    let mut brightness = DEFAULT_BRIGHTNESS;

    loop {
        loop {
            match receiver.try_recv() {
                Ok(RuntimeCommand::SetBrightness(percent)) => {
                    brightness = percent;
                    if let Some(device) = transport.as_ref() {
                        match brightness_report(percent) {
                            Ok(report) => {
                                if let Err(error) = device.send_feature_report(&report) {
                                    set_status(&app, &status, StreamDeckStatus::io_error(error));
                                    transport = None;
                                    next_scan = Instant::now();
                                }
                            }
                            Err(error) => {
                                set_status(&app, &status, StreamDeckStatus::io_error(error));
                            }
                        }
                    }
                }
                Ok(RuntimeCommand::SyncWorkspace {
                    workspace,
                    active_app_id,
                }) => {
                    last_workspace = Some((*workspace, active_app_id));
                    if let (Some(device), Some((workspace, active_app_id))) =
                        (transport.as_ref(), last_workspace.as_ref())
                        && let Err(error) =
                            sync_workspace_to_device(device, workspace, active_app_id.as_deref())
                    {
                        set_status(&app, &status, StreamDeckStatus::io_error(error));
                        transport = None;
                        next_scan = Instant::now();
                    }
                }
                Ok(RuntimeCommand::Stop) => {
                    if let Some(device) = transport.as_ref() {
                        let _ = device.send_feature_report(&show_logo_report());
                    }
                    set_status(
                        &app,
                        &status,
                        StreamDeckStatus::disconnected("OpenDeck hardware runtime stopped"),
                    );
                    return;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        if transport.is_none() {
            if Instant::now() < next_scan {
                match receiver.recv_timeout(IDLE_WAIT) {
                    Ok(command) => match command {
                        RuntimeCommand::SetBrightness(percent) => brightness = percent,
                        RuntimeCommand::SyncWorkspace {
                            workspace,
                            active_app_id,
                        } => last_workspace = Some((*workspace, active_app_id)),
                        RuntimeCommand::Stop => return,
                    },
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
                continue;
            }
            next_scan = Instant::now() + SCAN_INTERVAL;
            set_status(&app, &status, StreamDeckStatus::connecting());
            match open_device() {
                Ok(Some(OpenedDevice {
                    transport: device,
                    model,
                    serial,
                })) => {
                    let brightness_feature = match brightness_report(brightness) {
                        Ok(report) => report,
                        Err(error) => {
                            set_status(&app, &status, StreamDeckStatus::io_error(error));
                            continue;
                        }
                    };
                    if let Err(error) = device.send_feature_report(&brightness_feature) {
                        set_status(&app, &status, StreamDeckStatus::io_error(error));
                        continue;
                    }
                    let mut message = "Stream Deck + connected".to_string();
                    if let Some((workspace, active_app_id)) = last_workspace.as_ref() {
                        match sync_workspace_to_device(&device, workspace, active_app_id.as_deref())
                        {
                            Ok(warnings) if !warnings.is_empty() => {
                                message = format!(
                                    "Stream Deck + connected ({} render warning{})",
                                    warnings.len(),
                                    if warnings.len() == 1 { "" } else { "s" }
                                );
                            }
                            Ok(_) => {}
                            Err(error) => {
                                set_status(&app, &status, StreamDeckStatus::io_error(error));
                                continue;
                            }
                        }
                    }
                    set_status(
                        &app,
                        &status,
                        StreamDeckStatus::connected(model, serial, message),
                    );
                    input_state = InputState::default();
                    transport = Some(device);
                }
                Ok(None) => set_status(
                    &app,
                    &status,
                    StreamDeckStatus::disconnected("Stream Deck + not connected"),
                ),
                Err(error) => set_status(&app, &status, classify_open_error(&error)),
            }
            continue;
        }

        let mut buffer = [0u8; INPUT_SIZE];
        let Some(device) = transport.as_ref() else {
            continue;
        };
        let read_result = device.read_timeout(&mut buffer, READ_TIMEOUT_MS);
        match read_result {
            Ok(0) => {}
            Ok(count) => match parse_input_report(&buffer[..count]) {
                Ok(Some(report)) => emit_inputs(&app, &mut input_state, report),
                Ok(None) => {}
                Err(error) => {
                    set_status(
                        &app,
                        &status,
                        StreamDeckStatus::io_error(format!("Invalid Stream Deck input: {error}")),
                    );
                }
            },
            Err(error) => {
                set_status(
                    &app,
                    &status,
                    StreamDeckStatus::io_error(format!("Stream Deck disconnected: {error}")),
                );
                transport = None;
                next_scan = Instant::now();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeTransport {
        writes: RefCell<Vec<Vec<u8>>>,
        features: RefCell<Vec<Vec<u8>>>,
    }

    impl DeckTransport for FakeTransport {
        fn read_timeout(&self, _buffer: &mut [u8], _timeout_ms: i32) -> Result<usize, String> {
            Ok(0)
        }

        fn write(&self, report: &[u8]) -> Result<usize, String> {
            self.writes.borrow_mut().push(report.to_vec());
            Ok(report.len())
        }

        fn send_feature_report(&self, report: &[u8]) -> Result<(), String> {
            self.features.borrow_mut().push(report.to_vec());
            Ok(())
        }
    }

    #[test]
    fn diffs_button_states_into_edge_events() {
        let mut state = InputState::default();
        assert_eq!(
            state.apply(ParsedInputReport::Keys([
                true, false, false, false, false, false, false, false,
            ])),
            vec![StreamDeckInputEvent::KeyDown { index: 0 }]
        );
        assert_eq!(
            state.apply(ParsedInputReport::Keys([
                false, false, false, false, false, false, false, false,
            ])),
            vec![StreamDeckInputEvent::KeyUp { index: 0 }]
        );
    }

    #[test]
    fn expands_nonzero_dial_reports_without_synthesizing_button_events() {
        let mut state = InputState::default();
        assert_eq!(
            state.apply(ParsedInputReport::DialRotate([2, -3, 0, 0])),
            vec![
                StreamDeckInputEvent::DialRotate {
                    index: 0,
                    ticks: 2,
                    pressed: false,
                },
                StreamDeckInputEvent::DialRotate {
                    index: 1,
                    ticks: -3,
                    pressed: false,
                },
            ]
        );
    }

    #[test]
    fn marks_rotation_as_pressed_when_encoder_button_is_held() {
        let mut state = InputState::default();
        assert_eq!(
            state.apply(ParsedInputReport::DialButtons([true, false, false, false])),
            vec![StreamDeckInputEvent::DialDown { index: 0 }]
        );
        assert_eq!(
            state.apply(ParsedInputReport::DialRotate([-2, 1, 0, 0])),
            vec![
                StreamDeckInputEvent::DialRotate {
                    index: 0,
                    ticks: -2,
                    pressed: true,
                },
                StreamDeckInputEvent::DialRotate {
                    index: 1,
                    ticks: 1,
                    pressed: false,
                },
            ]
        );
    }

    #[test]
    fn serializes_hardware_events_with_frontend_field_names() {
        let value = serde_json::to_value(StreamDeckInputEvent::TouchFlick {
            start_x: 10,
            start_y: 20,
            end_x: 700,
            end_y: 30,
            region: 0,
        })
        .unwrap();
        assert_eq!(value["kind"], "touchFlick");
        assert_eq!(value["startX"], 10);
        assert_eq!(value["startY"], 20);
        assert_eq!(value["endX"], 700);
        assert_eq!(value["endY"], 30);
        assert!(value.get("start_x").is_none());
    }

    #[test]
    fn brightness_command_uses_feature_report() {
        let transport = FakeTransport::default();
        let mut workspace = None;
        assert!(
            apply_command(
                RuntimeCommand::SetBrightness(64),
                Some(&transport),
                &mut workspace,
            )
            .unwrap()
        );
        assert_eq!(transport.features.borrow()[0][0..3], [0x03, 0x08, 64]);
    }
}
