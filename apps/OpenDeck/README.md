# OpenDeck+ 2.0.37 — Desktop Entry Activation Closure

OpenDeck+ 2.0.37 is the bounded activation correction after 2.0.36 passed the real desktop-ID menu-launch gate but failed before binary switch because `desktop-file-validate` was asked to validate staging filenames that did not end in `.desktop`.

The production runtime, Action Wheel, Dial Stacks, event-loop startup behavior, Tauri permissions, and menu-launch path are unchanged from 2.0.36. The only behavioral delta is activation staging: temporary canonical and compatibility desktop entries now retain the required `.desktop` filename extension while being validated, then move atomically into their final locations.

The host qualifier adds a desktop-stage-extension regression gate before the inherited frontend, Rust, Tauri, Stream Deck+, visual, menu-launch, and activation gates. Activation remains last, so the current qualified installation is preserved unless every 2.0.37 gate passes.
