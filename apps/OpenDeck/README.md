# OpenDeck v2.0.0 Clean Baseline

A clean-room Linux Stream Deck controller baseline rebuilt from an empty tree. The first release prioritizes a Windows Stream Deck-style editor shell and usable OBS, Twitch, and Elgato Marketplace connections. No code from the retired v1.x implementation is copied into this tree.

## Scope

- Windows-style editor hierarchy: device/profile header, central Stream Deck Plus canvas, right Action List, bottom Property Inspector.
- OBS WebSocket 5.x connection plus scenes, stream toggle, record toggle, and input mute toggle.
- Twitch OAuth Device Code Flow with token validation and refresh.
- Elgato Marketplace browser handoff and local download discovery.
- Local icon-pack discovery from the user's existing OpenDeck icon-pack folder and Downloads.
- No background daemon, no autostart, and no DragonGlass in this baseline.

Hardware/HID runtime is intentionally deferred until the interface and service connections are proven usable.
