from pathlib import Path
root=Path(__file__).resolve().parents[1]
ws=(root/'Cargo.toml').read_text()
gui_main=(root/'crates/forgehx-gui/src/main.rs').read_text()
gui_app=(root/'crates/forgehx-gui/src/app.rs').read_text()
gui_cargo=(root/'crates/forgehx-gui/Cargo.toml').read_text()
tray_main=(root/'crates/forgehx-tray/src/main.rs').read_text() if (root/'crates/forgehx-tray/src/main.rs').exists() else ''
tray_cargo=(root/'crates/forgehx-tray/Cargo.toml').read_text() if (root/'crates/forgehx-tray/Cargo.toml').exists() else ''
desktop=(root/'packaging/desktop/io.forgehx.ForgeHX.desktop').read_text()
assert 'crates/forgehx-tray' in ws
assert 'ksni' in tray_cargo and 'ksni' not in gui_cargo
assert 'ViewportCommand::CancelClose' not in gui_app
assert 'ViewportCommand::Visible(false)' not in gui_app
assert 'mod tray;' not in gui_main
assert 'systemctl' in tray_main and 'forgehx-gui.service' in tray_main
assert (root/'packaging/systemd/forgehx-tray.service').exists()
assert (root/'packaging/systemd/forgehx-gui.service').exists()
assert 'forgehx-launch' in desktop
print('FORGEHX_10_0_6_TRAY_PROCESS=PASS')
