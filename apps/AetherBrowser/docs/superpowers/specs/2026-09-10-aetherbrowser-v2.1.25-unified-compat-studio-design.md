# AetherBrowser v2.1.25 Unified Compatibility + Aether Studio Design

## Intent

AetherBrowser v2.1.25 stops treating modern provider web apps and creator production tooling as external or loosely connected features. The browser remains the single user-facing application. Aether-native pages, lightweight browsing, provider compatibility surfaces, media fallback, OBS/Streamlabs/StreamElements workflows, AetherAI, OpenDeck, and creator-service controls all live behind the same tab/session/profile model and DragonGlass chrome.

## Non-negotiable session rule

Normal-window close is never logout. Existing persistent Servo profile data remains in place and is never wiped, rotated, or replaced during upgrade. Compatibility surfaces use their own stable persistent data directory and reuse it across restarts and upgrades. Private mode receives an isolated ephemeral compatibility directory. Closing a normal tab or browser window must only flush and close the engine; it must not clear cookies, local storage, IndexedDB, service-worker data, permissions, or provider auth state.

The installer may create new compatibility-state directories, but must never delete the existing normal profile. Any cross-engine migration is copy/transform only when safe; failed migration leaves the source untouched.

## Browser engine architecture

AetherBrowser becomes a provider-aware multi-surface browser:

1. **Aether Native** — Rust/egui/native pages and Aether control surfaces.
2. **Servo** — ordinary/lightweight web content and the existing persistent Servo profile.
3. **Compatibility WebView** — an in-window system Chromium child surface on Linux X11/XWayland, using one persistent Chromium user-data directory for all normal compatibility tabs. Chromium windows are reparented into the Aether web-content rectangle so the provider remains inside the same AetherBrowser window. The compatibility classifier routes YouTube, YouTube Music, Twitch, Streamlabs, StreamElements, Google Accounts, and configured compatibility-required origins to this surface.
4. **Native media fallback** — yt-dlp/FFmpeg/GStreamer for YouTube-family media and Streamlink/GStreamer for Twitch when provider-page media playback is degraded.

The compatibility surface is not a separate application window. It occupies the same web-content rectangle used by Servo and participates in AetherBrowser tab activation, hide/show, resize, focus, navigation, and close behavior.

## Provider routing

Compatibility-required hostnames include at minimum:

- `youtube.com`, `www.youtube.com`, `music.youtube.com`, `youtu.be`
- `twitch.tv`, `www.twitch.tv`
- `accounts.google.com`
- `streamlabs.com`, `www.streamlabs.com`
- `streamelements.com`, `www.streamelements.com`

Native Aether URLs never leave the native renderer. Other HTTP(S) URLs default to Servo unless explicitly classified compatibility-required.

Provider links opened from Aether pages preserve same-window separate-tab behavior. YouTube and Twitch tabs remain independent tabs but share the same persistent compatibility profile.

## Compatibility profile

The normal compatibility profile root is:

`$XDG_DATA_HOME/aetherforge/aether-browser/profiles/compat` or `$HOME/.local/share/aetherforge/aether-browser/profiles/compat`.

Private mode uses a unique temporary compatibility directory removed on shutdown. All normal Chromium compatibility surfaces launch against the same stable `--user-data-dir`, so cookies/site storage/service workers remain shared and persistent. Private compatibility surfaces use the private profile’s temporary compatibility directory and are destroyed with that profile.

## Provider media fallback

The compatibility web app is the primary provider experience. Native media fallback remains available and must not destroy the provider tab/session. Provider fallback state is per tab. The fallback transport does not become a login mechanism and never receives raw browser cookies through logs or command-line arguments.

## Aether Studio authority

AetherBrowser becomes the authoritative creator-production frontend. Standalone OBS/Streamlabs/StreamElements GUIs are no longer required for normal operation.

### OBS compatibility/backend

The current OBS WebSocket v5 client remains a supported backend and is expanded into a generic request transport for the complete OBS WebSocket request surface. Aether Studio exposes scene, source/input, filter, transition, mixer, stream, record, replay, virtual-camera, profile/collection, output/settings, and stats operations through typed browser routes.

A managed OBS backend may be used when libobs embedding is not available on the host. The user interacts with AetherBrowser, not the standalone OBS GUI. This preserves compatibility with OBS plugins and existing scene collections while Aether Studio owns the user-facing workflow.

### Native libobs path

The studio reports whether libobs is available. Native libobs embedding is an engine capability, not a hard preinstall blocker. The v2.1.25 control plane and migration/import layer must not depend on unsafe cookie/session handling or on deleting existing OBS data. When a host provides libobs, future native backend work can replace the managed compatibility backend without changing Aether Studio's command model.

## Streamlabs integration

Streamlabs integration includes:

- persistent compatibility tabs for account/dashboard/widget pages;
- existing official OAuth/API/socket descriptors;
- alerts, donations/tips, media share, stream-health and realtime event routing where provider authorization exists;
- import/migration descriptors for Streamlabs scene/overlay/widget configuration when local files are available;
- no requirement to run Streamlabs Desktop for normal Aether Studio use.

Cloud-only or proprietary account services remain provider-backed; AetherBrowser does not counterfeit them.

## StreamElements integration

StreamElements integration includes:

- persistent compatibility tabs for dashboard/overlay/widget pages;
- existing Astro realtime topics and account/API descriptors;
- overlay, alert, tip, chat, chatbot and realtime-event surfaces where authorization exists;
- browser-source URLs can be used directly as Aether Studio sources;
- no requirement for SE.Live as a separate UI.

## OpenDeck / Stream Deck

Aether Studio exposes stable command IDs for scene changes, source visibility, mute, recording, streaming, replay, transitions, studio mode, virtual camera, and provider actions so OpenDeck can bind without screen scraping or OBS-window focus.

## Import/export and non-lock-in

AetherBrowser never destroys existing OBS, Streamlabs, or StreamElements configuration during import. OBS scene/profile/collection compatibility remains importable/exportable. Provider web account state remains provider-owned. Aether Studio stores its own normalized control-plane state separately.

## Acceptance gates

v2.1.25 must add static/runtime gates for:

- compatibility URL routing;
- persistent normal compatibility profile;
- ephemeral private compatibility profile;
- normal window close does not delete either persistent profile;
- same-window separate YouTube/Twitch compatibility tabs;
- compatibility surface geometry follows the Aether web-content rectangle;
- YouTube/YouTube Music/Twitch route to compatibility surface, not Servo;
- Streamlabs/StreamElements route to compatibility surface;
- native media fallback remains available;
- Aether Studio is browser-authoritative;
- generic OBS WebSocket request transport exists;
- OBS scene/source/input/filter/transition/mixer/output control categories are represented;
- Streamlabs and StreamElements integration descriptors remain available;
- OpenDeck command IDs are stable;
- install/upgrade preserves the existing normal profile tree.

Host verification must distinguish compile/install success, compatibility-engine startup, provider UI load, provider media playback, and user-observed login persistence. It must never log cookies, OAuth tokens, passwords, or provider secrets.
