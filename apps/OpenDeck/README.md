# OpenDeck+ 2.0.19 — Canonical Canvas Fit Scaling

OpenDeck+ 2.0.19 preserves the approved 1536×1024 DragonGlass editor as one authoritative canvas and adapts it to smaller displays only by applying a single uniform fit scale. It does not reflow, independently resize, hide, or rearrange canonical panels and controls.

Normal mode maximizes the Tauri window, then computes `min(viewportWidth / 1536, viewportHeight / 1024, 1)` and scales the entire canonical canvas around its center. Resizing the window recomputes the fit scale. Qualification mode is locked to 1.0 scale so canonical 1536×1024 screenshot geometry remains unchanged.

The legacy max-width media-query reflow rules are removed because they would mutate canonical geometry on smaller screens. Key, dial, touch-strip, profile, action-library, Twitch, OBS, persistence, HID, encoder, candidate-bound screenshot, and qualification behavior remain preserved. Stream Deck controls remain fully remappable.

Active baseline remains OpenDeck+ 2.0.8 until 2.0.19 completes full host qualification and human visual approval. Dial Stacks are deferred to 2.0.20.
