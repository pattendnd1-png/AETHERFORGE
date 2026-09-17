# OpenDeck+ 2.0.14 — Bootstrap/Readiness Diagnostic Closure

2.0.13 built successfully on the host but opened as a blank DragonGlass window and failed at PREVIEW_VISUAL_READY before screenshot capture. The root cause is bootstrap ordering: App.tsx decided qualification mode at module import time before main.tsx obtained the Rust qualification context.

2.0.14 passes qualification context directly into the React app, shows an immediate visible boot shell, catches React render failures visibly, and captures a BOOT-DIAGNOSTIC PNG if visual readiness is not reached. The approved canonical render and production editor layout are unchanged.

The qualified/active baseline remains 2.0.8 until all frontend, Rust/Tauri, Stream Deck+, visual, performance, and human approval gates pass. Dial Stacks are deferred to 2.0.15.
