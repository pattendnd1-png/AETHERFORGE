# OpenDeck+ 2.0.15 — Production Tauri Preview Closure

2.0.14 proved the frontend, Rust candidate, and diagnostic capture path were working far enough to launch a window, but its visual preview opened the WebView `Connection refused` page. The qualification runner had built the preview with plain `cargo build --release`, while Tauri development configuration still pointed `devUrl` at `http://localhost:1420`.

2.0.15 fixes the qualification architecture: the visual candidate is built with the production Tauri path (`npm run tauri -- build --no-bundle`), which runs the configured frontend production build and embeds `frontendDist`. Visual qualification never starts a Vite/dev server and does not depend on localhost.

The direct qualification-context bootstrap, visible boot shell, React error boundary, and BOOT-DIAGNOSTIC fallback from 2.0.14 are preserved. The approved canonical render and production editor layout are unchanged.

The qualified/active baseline remains 2.0.8 until all frontend, Rust/Tauri, Stream Deck+, visual, performance, and human approval gates pass. Dial Stacks are deferred to 2.0.16.
