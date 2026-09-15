#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
checks = {
    'crates/forgehx-gui/src/equalizer.rs': [
        ('widgets::info_row(ui, "EQ backend", &owner.backend);', False),
        ('widgets::info_row(ui, "EQ backend", owner.backend);', True),
    ],
    'crates/forgehx-gui/src/lighting.rs': [
        ('widgets::info_row(ui, "Control backend", &owner.backend);', False),
        ('widgets::info_row(ui, "Control backend", owner.backend);', True),
    ],
    'crates/forgehx-gui/src/microphone.rs': [
        ('widgets::info_row(ui, "Processing backend", &owner.backend);', False),
        ('widgets::info_row(ui, "Processing backend", owner.backend);', True),
    ],
    'crates/forgehx-gui/src/mouse.rs': [
        ('stages: &mut Vec<u16>,', False),
        ('stages: &mut [u16],', True),
        ('stages: stages.clone(), active_stage: *active', False),
        ('stages: stages.to_owned(), active_stage: *active', True),
        ('widgets::info_row(ui, "Mouse backend", &owner.backend);', False),
        ('widgets::info_row(ui, "Mouse backend", owner.backend);', True),
    ],
}
errors = []
for rel, rules in checks.items():
    text = (root / rel).read_text()
    for needle, should_exist in rules:
        present = needle in text
        if present != should_exist:
            errors.append(f"{rel}: {'missing' if should_exist else 'still contains'} {needle!r}")
if errors:
    print('\n'.join(errors))
    raise SystemExit(1)
print('PASS: ForgeHX 10.0.24 GUI clippy cleanup contract')
