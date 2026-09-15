use ksni::blocking::TrayMethods;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrayCommand {
    Open,
    Quit,
}

struct ForgeHxTray {
    sender: Sender<TrayCommand>,
}

impl ForgeHxTray {
    fn send(&self, command: TrayCommand) {
        let _ = self.sender.send(command);
    }
}

impl ksni::Tray for ForgeHxTray {
    fn id(&self) -> String {
        "io.forgehx.ForgeHX".into()
    }
    fn category(&self) -> ksni::Category {
        ksni::Category::Hardware
    }
    fn title(&self) -> String {
        "ForgeHX".into()
    }
    fn status(&self) -> ksni::Status {
        ksni::Status::Active
    }
    fn icon_name(&self) -> String {
        "preferences-system".into()
    }
    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "ForgeHX".into(),
            description: "HyperX control, processed microphone, DSP, and communication routing"
                .into(),
            ..Default::default()
        }
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        self.send(TrayCommand::Open);
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        vec![
            StandardItem {
                label: "Open ForgeHX".into(),
                icon_name: "window-new".into(),
                activate: Box::new(|tray: &mut Self| tray.send(TrayCommand::Open)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "ForgeHX DSP + chat routing: managed by daemon".into(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "Quit ForgeHX tray".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|tray: &mut Self| tray.send(TrayCommand::Quit)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn start_gui() {
    let _ = Command::new("systemctl")
        .args(["--user", "start", "forgehx-gui.service"])
        .status();
}

fn stop_gui() {
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "forgehx-gui.service"])
        .status();
}

fn event_loop(receiver: Receiver<TrayCommand>) {
    while let Ok(command) = receiver.recv() {
        match command {
            TrayCommand::Open => start_gui(),
            TrayCommand::Quit => {
                stop_gui();
                break;
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (sender, receiver) = mpsc::channel();
    let handle = ForgeHxTray { sender }.spawn()?;
    event_loop(receiver);
    handle.shutdown().wait();
    Ok(())
}
