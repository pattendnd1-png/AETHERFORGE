# ForgeHX 10.0.6 Tray + Communication Routing Design

## Goal
ForgeHX must close its visible GUI process when the user presses X while leaving a persistent system tray host and the background daemon/DSP active. ForgeHX Processed Mic must also become the preferred system capture source for communication applications.

## Tray architecture
- Add a standalone `forgehx-tray` Rust binary using StatusNotifierItem (`ksni`).
- The GUI no longer owns a tray or hides its viewport; normal close exits `forgehx-gui`.
- Add user services `forgehx-tray.service` and `forgehx-gui.service`.
- The tray host opens the GUI with `systemctl --user start forgehx-gui.service`; systemd prevents duplicate managed GUI instances.
- The desktop launcher uses a helper that starts both tray and GUI services.
- Tray Quit stops the GUI service and exits the tray host normally; the daemon remains independent.

## Communication routing
- `ForgeHX Processed Mic` remains the single post-DSP virtual source used by OBS and communications apps.
- When the processed source is active, the daemon resolves its current PipeWire node id and runs `wpctl set-default <id>`.
- ForgeHX also ensures WirePlumber `linking.allow-moving-streams=true` and `linking.follow-default-target=true` at runtime so default-following capture streams can move when the default changes.
- Explicit per-app microphone selections are not continuously overridden.
- The physical SoloCast remains the DSP input and is never exposed as the preferred chat source while ForgeHX routing is active.

## Safety / lifecycle
- No PipeWire or WirePlumber restart is performed.
- Live wired headphone monitoring remains a separate post-DSP tee and never uses Bluetooth.
- Closing the GUI does not stop the daemon, DSP, VoicePilot, chat routing, or monitor.
