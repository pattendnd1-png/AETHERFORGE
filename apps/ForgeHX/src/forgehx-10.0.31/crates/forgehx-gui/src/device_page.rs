use crate::{equalizer, firmware, hardware, keyboard, lighting, microphone, mouse, theme, widgets};
use eframe::egui::{self, Color32, Pos2, RichText, Sense, Stroke, Vec2};
use forgehx_core::{
    AudioNode, Capability, DeviceClass, DeviceId, DeviceInfo, DpiConfig, EqConfig,
    FirmwareIdentity, FirmwarePackageInfo, KeyboardModelInfo, LightingConfig,
    LightingControllerMetadata, MicrophoneDspConfig, MicrophoneDspState, MouseDeviceState,
    MouseModelInfo,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceTab {
    Overview,
    Keyboard,
    Hardware,
    Lights,
    Keys,
    Assignments,
    Performance,
    Dpi,
    Audio,
    Microphone,
    InputDsp,
    Firmware,
    Eq,
    Profiles,
    Doctor,
}

impl DeviceTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Keyboard => "Keyboard",
            Self::Hardware => "Hardware",
            Self::Lights => "Lights",
            Self::Keys => "Keys",
            Self::Assignments => "Assignments",
            Self::Performance => "Performance",
            Self::Dpi => "DPI",
            Self::Audio => "Audio",
            Self::Microphone => "Microphone",
            Self::InputDsp => "Input DSP",
            Self::Firmware => "Firmware",
            Self::Eq => "EQ",
            Self::Profiles => "Profiles",
            Self::Doctor => "Doctor",
        }
    }
}

#[derive(Debug, Clone)]
pub enum DeviceAction {
    RunDoctor(DeviceId),
    SetLighting(DeviceId, LightingConfig),
    SetDpi(DeviceId, DpiConfig),
    SetPollingRate(DeviceId, u16),
    SetMouseProfile(DeviceId, u8),
    SetButtonAssignment(DeviceId, u32, String),
    SetLiftOffDistance(DeviceId, u8),
    SetAudioVolume(u32, f32),
    SetAudioMute(u32, bool),
    EqLoad(String),
    EqSave(EqConfig),
    EqApply(String),
    EqDelete(String),
    EqBypass,
    MicDspSave(DeviceId, MicrophoneDspConfig),
    MicDspLiveUpdate(DeviceId, MicrophoneDspConfig),
    MicDspApply(DeviceId, MicrophoneDspConfig),
    MicVoiceForget(DeviceId),
    MicMonitor(DeviceId, bool, f32),
    MicFirmwareRefresh(DeviceId),
    MicFirmwareStage(DeviceId, String),
    MicFirmwareValidate(DeviceId, String),
    MicFirmwareBegin(DeviceId, String),
    MicFirmwareForget(String),
}

pub fn tabs_for_device(info: &DeviceInfo) -> Vec<DeviceTab> {
    let mut tabs = vec![DeviceTab::Overview];
    if info.device_class == DeviceClass::Keyboard {
        tabs.push(DeviceTab::Keyboard);
    }
    if matches!(
        info.device_class,
        DeviceClass::Headset
            | DeviceClass::Controller
            | DeviceClass::Webcam
            | DeviceClass::Mousepad
            | DeviceClass::Monitor
            | DeviceClass::AudioInterface
            | DeviceClass::UsbAudio
    ) {
        tabs.push(DeviceTab::Hardware);
    }
    if info.supports(Capability::Lighting) {
        tabs.push(DeviceTab::Lights);
    }
    if info.supports(Capability::Bindings) {
        tabs.push(DeviceTab::Keys);
        tabs.push(DeviceTab::Assignments);
    }
    if info.supports(Capability::Dpi)
        || info.supports(Capability::PollingRate)
        || info.supports(Capability::Profiles)
    {
        tabs.push(DeviceTab::Performance);
    }
    if info.supports(Capability::Dpi) {
        tabs.push(DeviceTab::Dpi);
    }
    if info.supports(Capability::Audio) {
        tabs.push(DeviceTab::Audio);
    }
    if info.supports(Capability::Microphone) {
        tabs.push(DeviceTab::Microphone);
    }
    if info.supports(Capability::MicDsp) {
        tabs.push(DeviceTab::InputDsp);
    }
    if info.supports(Capability::MicFirmwareInventory) {
        tabs.push(DeviceTab::Firmware);
    }
    if info.supports(Capability::Eq) {
        tabs.push(DeviceTab::Eq);
    }
    tabs.push(DeviceTab::Profiles);
    tabs.push(DeviceTab::Doctor);
    tabs
}

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    device: &DeviceInfo,
    selected_tab: &mut DeviceTab,
    pane_changed: &mut bool,
    doctor_text: &mut String,
    lighting_config: &mut LightingConfig,
    lighting_metadata: Option<&LightingControllerMetadata>,
    dpi_stages: &mut [u16],
    dpi_active: &mut usize,
    mouse_state: Option<&MouseDeviceState>,
    mouse_model: Option<&MouseModelInfo>,
    keyboard_model: Option<&KeyboardModelInfo>,
    polling_rate: &mut u16,
    mouse_profile: &mut u8,
    button_number: &mut u32,
    button_action: &mut String,
    mouse_lift_off_distance: &mut u8,
    audio_nodes: &[AudioNode],
    mic_dsp_config: &mut MicrophoneDspConfig,
    mic_dsp_state: Option<&MicrophoneDspState>,
    live_mic_dsp_edits: bool,
    mic_monitor_enabled: &mut bool,
    mic_monitor_level_percent: &mut f32,
    firmware_identity: Option<&FirmwareIdentity>,
    firmware_package: Option<&FirmwarePackageInfo>,
    firmware_path: &mut String,
    profiles: &[String],
    eq_config: &mut EqConfig,
    eq_profiles: &[String],
    selected_eq_profile: &mut String,
) -> Option<DeviceAction> {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.heading(RichText::new(&device.name).size(28.0).strong());
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} • {}",
                    device.vendor_family, device.device_class
                ));
                ui.separator();
                widgets::support_label(ui, device.support_level);
            });
        });
    });
    ui.add_space(12.0);

    presentation(ui, device);
    ui.add_space(10.0);
    widgets::capability_chips(ui, device);
    ui.add_space(12.0);

    let tabs = tabs_for_device(device);
    let tab_before = *selected_tab;
    if !tabs.contains(selected_tab) {
        *selected_tab = DeviceTab::Overview;
    }
    ui.horizontal_wrapped(|ui| {
        for tab in &tabs {
            if ui
                .selectable_label(*selected_tab == *tab, tab.label())
                .clicked()
            {
                *selected_tab = *tab;
            }
        }
    });
    *pane_changed = *selected_tab != tab_before;
    ui.separator();
    ui.add_space(8.0);

    match *selected_tab {
        DeviceTab::Overview => show_overview(ui, device),
        DeviceTab::Keyboard => keyboard::show(ui, device, keyboard_model),
        DeviceTab::Hardware => {
            if let Some(control) = hardware::show(ui, device, audio_nodes) {
                return Some(match control {
                    hardware::HardwareControl::SetVolume(node, volume) => {
                        DeviceAction::SetAudioVolume(node, volume)
                    }
                    hardware::HardwareControl::SetMute(node, muted) => {
                        DeviceAction::SetAudioMute(node, muted)
                    }
                });
            }
        }
        DeviceTab::Lights => {
            if lighting::show(ui, device, lighting_config, lighting_metadata) {
                return Some(DeviceAction::SetLighting(
                    device.id.clone(),
                    lighting_config.clone(),
                ));
            }
        }
        DeviceTab::Keys | DeviceTab::Assignments => {
            if device.device_class == DeviceClass::Mouse {
                if let Some(control) = mouse::show(
                    ui,
                    device,
                    mouse_state,
                    mouse_model.map(|model| &model.limits),
                    dpi_stages,
                    dpi_active,
                    polling_rate,
                    mouse_profile,
                    button_number,
                    button_action,
                    mouse_lift_off_distance,
                ) {
                    return Some(mouse_action(device, control));
                }
            } else {
                ui.label("No verified onboard key-assignment backend is available for this keyboard yet.");
            }
        }
        DeviceTab::Performance | DeviceTab::Dpi => {
            if let Some(percent) = device.battery_percent {
                ui.add(
                    egui::ProgressBar::new(percent as f32 / 100.0)
                        .text(format!("Battery {percent}%")),
                );
            }
            if let Some(control) = mouse::show(
                ui,
                device,
                mouse_state,
                mouse_model.map(|model| &model.limits),
                dpi_stages,
                dpi_active,
                polling_rate,
                mouse_profile,
                button_number,
                button_action,
                mouse_lift_off_distance,
            ) {
                return Some(mouse_action(device, control));
            }
        }
        DeviceTab::Audio => return show_audio(ui, device, audio_nodes, "sink"),
        DeviceTab::Microphone => return show_audio(ui, device, audio_nodes, "source"),
        DeviceTab::InputDsp => {
            if let Some(control) = microphone::show(
                ui,
                device,
                mic_dsp_config,
                mic_dsp_state,
                live_mic_dsp_edits,
                mic_monitor_enabled,
                mic_monitor_level_percent,
            ) {
                return Some(mic_action(device, control));
            }
        }
        DeviceTab::Firmware => {
            if let Some(control) =
                firmware::show(ui, firmware_identity, firmware_package, firmware_path)
            {
                return Some(firmware_action(device, control));
            }
        }
        DeviceTab::Eq => {
            if let Some(control) =
                equalizer::show(ui, device, eq_config, eq_profiles, selected_eq_profile)
            {
                return Some(eq_action(control));
            }
        }
        DeviceTab::Profiles => {
            ui.heading("Profiles");
            if profiles.is_empty() {
                ui.label("No saved profiles.");
            }
            for profile in profiles {
                ui.label(format!("• {profile}"));
            }
        }
        DeviceTab::Doctor => {
            if ui.button("Run read-only Device Doctor").clicked() {
                return Some(DeviceAction::RunDoctor(device.id.clone()));
            }
            ui.label(
                RichText::new("Grouped HID, USB, and audio interfaces. No vendor writes are sent.")
                    .color(theme::text_secondary()),
            );
            ui.add(
                egui::TextEdit::multiline(doctor_text)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(16),
            );
        }
    }
    None
}

fn presentation(ui: &mut egui::Ui, device: &DeviceInfo) {
    let desired = Vec2::new(ui.available_width(), 230.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, egui::CornerRadius::same(14), theme::bg_panel());
    let glow = Color32::from_rgba_unmultiplied(
        theme::accent().r(),
        theme::accent().g(),
        theme::accent().b(),
        35,
    );
    painter.circle_filled(rect.center(), 105.0, glow);
    draw_device(&painter, rect.center(), device.device_class);
    painter.text(
        Pos2::new(rect.left() + 18.0, rect.bottom() - 24.0),
        egui::Align2::LEFT_CENTER,
        format!(
            "{}  •  {}",
            device.device_class,
            device.connection_kind_label()
        ),
        egui::FontId::proportional(13.0),
        theme::text_secondary(),
    );
}

fn draw_device(painter: &egui::Painter, center: Pos2, class: DeviceClass) {
    let body = Color32::from_rgb(72, 73, 80);
    let detail = Color32::from_rgb(122, 124, 132);
    let accent = theme::accent();
    match class {
        DeviceClass::Keyboard => {
            let rect = egui::Rect::from_center_size(center, Vec2::new(270.0, 92.0));
            painter.rect_filled(rect, egui::CornerRadius::same(10), body);
            for row in 0..4 {
                for col in 0..12 {
                    let pos = Pos2::new(
                        rect.left() + 17.0 + col as f32 * 20.5,
                        rect.top() + 14.0 + row as f32 * 19.0,
                    );
                    painter.rect_filled(
                        egui::Rect::from_min_size(pos, Vec2::new(15.0, 12.0)),
                        egui::CornerRadius::same(2),
                        detail,
                    );
                }
            }
            painter.rect_filled(
                egui::Rect::from_center_size(
                    Pos2::new(center.x, rect.bottom() - 12.0),
                    Vec2::new(90.0, 5.0),
                ),
                egui::CornerRadius::same(2),
                accent,
            );
        }
        DeviceClass::Mouse => {
            painter.circle_filled(Pos2::new(center.x, center.y + 5.0), 58.0, body);
            painter.rect_filled(
                egui::Rect::from_center_size(
                    Pos2::new(center.x, center.y + 32.0),
                    Vec2::new(96.0, 72.0),
                ),
                egui::CornerRadius::same(34),
                body,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y - 50.0),
                    Pos2::new(center.x, center.y + 20.0),
                ],
                Stroke::new(3.0_f32, detail),
            );
            painter.circle_filled(Pos2::new(center.x, center.y - 20.0), 7.0, accent);
        }
        DeviceClass::Headset => {
            painter.line_segment(
                [
                    Pos2::new(center.x - 62.0, center.y),
                    Pos2::new(center.x - 45.0, center.y - 55.0),
                ],
                Stroke::new(13.0_f32, body),
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - 45.0, center.y - 55.0),
                    Pos2::new(center.x + 45.0, center.y - 55.0),
                ],
                Stroke::new(13.0_f32, body),
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + 45.0, center.y - 55.0),
                    Pos2::new(center.x + 62.0, center.y),
                ],
                Stroke::new(13.0_f32, body),
            );
            painter.circle_filled(Pos2::new(center.x - 66.0, center.y + 20.0), 34.0, detail);
            painter.circle_filled(Pos2::new(center.x + 66.0, center.y + 20.0), 34.0, detail);
            painter.line_segment(
                [
                    Pos2::new(center.x + 78.0, center.y + 34.0),
                    Pos2::new(center.x + 108.0, center.y + 58.0),
                ],
                Stroke::new(5.0_f32, accent),
            );
        }
        DeviceClass::Microphone => {
            painter.rect_filled(
                egui::Rect::from_center_size(
                    Pos2::new(center.x, center.y - 20.0),
                    Vec2::new(70.0, 120.0),
                ),
                egui::CornerRadius::same(30),
                body,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y + 40.0),
                    Pos2::new(center.x, center.y + 75.0),
                ],
                Stroke::new(8.0_f32, detail),
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - 42.0, center.y + 76.0),
                    Pos2::new(center.x + 42.0, center.y + 76.0),
                ],
                Stroke::new(8.0_f32, detail),
            );
            painter.rect_filled(
                egui::Rect::from_center_size(
                    Pos2::new(center.x, center.y - 54.0),
                    Vec2::new(54.0, 5.0),
                ),
                egui::CornerRadius::same(2),
                accent,
            );
        }
        DeviceClass::Controller => {
            painter.circle_filled(Pos2::new(center.x - 48.0, center.y + 12.0), 50.0, body);
            painter.circle_filled(Pos2::new(center.x + 48.0, center.y + 12.0), 50.0, body);
            painter.rect_filled(
                egui::Rect::from_center_size(center, Vec2::new(100.0, 70.0)),
                egui::CornerRadius::same(22),
                body,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - 58.0, center.y),
                    Pos2::new(center.x - 30.0, center.y),
                ],
                Stroke::new(6.0_f32, detail),
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - 44.0, center.y - 14.0),
                    Pos2::new(center.x - 44.0, center.y + 14.0),
                ],
                Stroke::new(6.0_f32, detail),
            );
            painter.circle_filled(Pos2::new(center.x + 47.0, center.y - 9.0), 7.0, accent);
            painter.circle_filled(Pos2::new(center.x + 65.0, center.y + 7.0), 7.0, detail);
        }
        DeviceClass::Webcam => {
            painter.rect_filled(
                egui::Rect::from_center_size(center, Vec2::new(160.0, 88.0)),
                egui::CornerRadius::same(18),
                body,
            );
            painter.circle_filled(center, 28.0, detail);
            painter.circle_filled(center, 14.0, accent);
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y + 45.0),
                    Pos2::new(center.x, center.y + 72.0),
                ],
                Stroke::new(7.0_f32, detail),
            );
        }
        _ => {
            painter.rect_filled(
                egui::Rect::from_center_size(center, Vec2::new(150.0, 110.0)),
                egui::CornerRadius::same(24),
                body,
            );
            painter.circle_filled(center, 27.0, accent);
        }
    }
}

fn show_overview(ui: &mut egui::Ui, device: &DeviceInfo) {
    ui.columns(2, |columns| {
        columns[0].group(|ui| {
            ui.heading("Device");
            widgets::info_row(ui, "Vendor", &device.vendor_family);
            widgets::info_row(ui, "Class", device.device_class);
            let usb = if device.vendor_id == 0 {
                "Not exposed".into()
            } else {
                format!("{:04x}:{:04x}", device.vendor_id, device.product_id)
            };
            widgets::info_row(ui, "USB ID", usb);
            widgets::info_row(
                ui,
                "Serial",
                device.serial.as_deref().unwrap_or("Not exposed"),
            );
        });
        columns[1].group(|ui| {
            ui.heading("ForgeHX support");
            widgets::support_label(ui, device.support_level);
            widgets::info_row(
                ui,
                "Driver",
                device
                    .protocol
                    .as_deref()
                    .unwrap_or("No verified vendor driver"),
            );
            widgets::info_row(ui, "Interfaces", device.interfaces.len());
            widgets::info_row(
                ui,
                "Vendor writes",
                if device.protocol.is_some() {
                    "Capability gated"
                } else {
                    "Blocked"
                },
            );
        });
    });
}

fn mouse_action(device: &DeviceInfo, control: mouse::MouseControl) -> DeviceAction {
    match control {
        mouse::MouseControl::Dpi(config) => DeviceAction::SetDpi(device.id.clone(), config),
        mouse::MouseControl::PollingRate(hz) => DeviceAction::SetPollingRate(device.id.clone(), hz),
        mouse::MouseControl::Profile(profile) => {
            DeviceAction::SetMouseProfile(device.id.clone(), profile)
        }
        mouse::MouseControl::Button { button, action } => {
            DeviceAction::SetButtonAssignment(device.id.clone(), button, action)
        }
        mouse::MouseControl::LiftOffDistance(mm) => {
            DeviceAction::SetLiftOffDistance(device.id.clone(), mm)
        }
    }
}

fn mic_action(device: &DeviceInfo, control: microphone::MicControl) -> DeviceAction {
    match control {
        microphone::MicControl::Save(config) => DeviceAction::MicDspSave(device.id.clone(), config),
        microphone::MicControl::LiveUpdate(config) => {
            DeviceAction::MicDspLiveUpdate(device.id.clone(), config)
        }
        microphone::MicControl::Apply(config) => {
            DeviceAction::MicDspApply(device.id.clone(), config)
        }
        microphone::MicControl::ForgetVoice => DeviceAction::MicVoiceForget(device.id.clone()),
        microphone::MicControl::Monitor {
            enabled,
            level_percent,
        } => DeviceAction::MicMonitor(device.id.clone(), enabled, level_percent),
    }
}

fn firmware_action(device: &DeviceInfo, control: firmware::FirmwareControl) -> DeviceAction {
    match control {
        firmware::FirmwareControl::Refresh => DeviceAction::MicFirmwareRefresh(device.id.clone()),
        firmware::FirmwareControl::Stage(path) => {
            DeviceAction::MicFirmwareStage(device.id.clone(), path)
        }
        firmware::FirmwareControl::Validate(staged_id) => {
            DeviceAction::MicFirmwareValidate(device.id.clone(), staged_id)
        }
        firmware::FirmwareControl::Begin(staged_id) => {
            DeviceAction::MicFirmwareBegin(device.id.clone(), staged_id)
        }
        firmware::FirmwareControl::Forget(staged_id) => DeviceAction::MicFirmwareForget(staged_id),
    }
}

fn eq_action(control: equalizer::EqControl) -> DeviceAction {
    match control {
        equalizer::EqControl::Load(name) => DeviceAction::EqLoad(name),
        equalizer::EqControl::Save(config) => DeviceAction::EqSave(config),
        equalizer::EqControl::Apply(name) => DeviceAction::EqApply(name),
        equalizer::EqControl::Delete(name) => DeviceAction::EqDelete(name),
        equalizer::EqControl::Bypass => DeviceAction::EqBypass,
    }
}

fn show_audio(
    ui: &mut egui::Ui,
    device: &DeviceInfo,
    nodes: &[AudioNode],
    wanted_kind: &str,
) -> Option<DeviceAction> {
    ui.heading(if wanted_kind == "source" {
        "Microphone"
    } else {
        "Audio"
    });
    let ids = device
        .interfaces
        .iter()
        .filter_map(|interface| interface.audio_node_id)
        .collect::<Vec<_>>();
    let relevant = nodes
        .iter()
        .filter(|node| ids.contains(&node.id) && node.kind == wanted_kind)
        .collect::<Vec<_>>();
    if relevant.is_empty() {
        ui.label("No currently associated PipeWire node was returned for this device.");
        return None;
    }
    for node in relevant {
        let mut volume = node.volume.unwrap_or(1.0);
        let mut muted = node.muted.unwrap_or(false);
        let mut action = None;
        ui.group(|ui| {
            ui.label(RichText::new(&node.name).strong());
            ui.small(format!("{} • PipeWire node {}", node.kind, node.id));
            if ui
                .add(egui::Slider::new(&mut volume, 0.0..=1.5).text("Volume"))
                .changed()
            {
                action = Some(DeviceAction::SetAudioVolume(node.id, volume));
            }
            if ui.checkbox(&mut muted, "Muted").changed() {
                action = Some(DeviceAction::SetAudioMute(node.id, muted));
            }
        });
        if action.is_some() {
            return action;
        }
        ui.add_space(6.0);
    }
    None
}

trait DeviceConnectionLabel {
    fn connection_kind_label(&self) -> &'static str;
}

impl DeviceConnectionLabel for DeviceInfo {
    fn connection_kind_label(&self) -> &'static str {
        match self.connection_kind {
            forgehx_core::ConnectionKind::Usb => "USB",
            forgehx_core::ConnectionKind::Hid => "HID",
            forgehx_core::ConnectionKind::Audio => "Audio",
            forgehx_core::ConnectionKind::Composite => "Composite",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forgehx_core::{
        ConnectionKind, DeviceInterface, InterfaceSource, SupportLevel, VendorFamily,
    };

    fn info(capabilities: Vec<Capability>) -> DeviceInfo {
        DeviceInfo {
            id: DeviceId("x".into()),
            name: "Test Mouse".into(),
            manufacturer: Some("HyperX".into()),
            vendor_id: 0x0951,
            product_id: 2,
            serial: None,
            vendor_family: VendorFamily::HyperX,
            device_class: DeviceClass::Mouse,
            support_level: SupportLevel::PartiallySupported,
            connection_kind: ConnectionKind::Hid,
            interfaces: vec![DeviceInterface {
                source: InterfaceSource::Hid,
                path: "x".into(),
                vendor_id: Some(0x0951),
                product_id: Some(2),
                interface_number: None,
                usage_page: None,
                usage: None,
                audio_node_id: None,
                usb_parent: None,
            }],
            capabilities,
            generic_capabilities: Vec::new(),
            capability_owners: Vec::new(),
            battery_percent: None,
            battery_state: None,
            protocol: Some("test".into()),
        }
    }

    #[test]
    fn capability_driven_tabs_include_mouse_controls() {
        let tabs = tabs_for_device(&info(vec![
            Capability::Diagnostics,
            Capability::Lighting,
            Capability::Dpi,
        ]));
        assert!(tabs.contains(&DeviceTab::Overview));
        assert!(tabs.contains(&DeviceTab::Lights));
        assert!(tabs.contains(&DeviceTab::Dpi));
        assert!(tabs.contains(&DeviceTab::Profiles));
        assert!(tabs.contains(&DeviceTab::Doctor));
        assert!(!tabs.contains(&DeviceTab::Audio));
    }

    #[test]
    fn keyboard_class_always_gets_keyboard_tab() {
        let mut device = info(vec![Capability::Diagnostics]);
        device.name = "HyperX Alloy Rise 75 Wireless".into();
        device.device_class = DeviceClass::Keyboard;
        let tabs = tabs_for_device(&device);
        assert!(tabs.contains(&DeviceTab::Keyboard));
    }

    #[test]
    fn microphone_dsp_capability_adds_input_dsp_tab() {
        let mut device = info(vec![Capability::Diagnostics, Capability::MicDsp]);
        device.device_class = DeviceClass::Microphone;
        let tabs = tabs_for_device(&device);
        assert!(tabs.contains(&DeviceTab::InputDsp));
    }
}
