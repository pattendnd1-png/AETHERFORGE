#![forbid(unsafe_code)]
//! First-party Aether Browser pages rendered from Rust-owned state.

use aether_comms::{
    CapturePrivacy, CommsAccount, CommsProviderRegistry, PROTON_BRIDGE_URL, PROTON_MAIL_WEB_URL,
    SecretRef as CommsSecretRef, UnifiedInbox, ZOHO_MAIL_WEB_URL, proton_mail_account,
    zoho_mail_account,
};
use aether_creator_integrations::vendor_packages::{
    authoritative_vendor_packages, ground_control_alert_actions, package_reference_status,
};
use aether_creator_integrations::{
    BLUESKY_LEXICONS, CreatorCapability, STREAMELEMENTS_TOPICS, bluesky_descriptor,
    stream_elements_descriptor, streamlabs_descriptor, twitch_descriptor,
};
use aether_identity::IdentityProvider;
use aether_media::MediaSupportMatrix;
use aether_storage::LibrarySnapshot;
use aether_stream_providers::{ProviderManifest, built_in_providers};
use aether_stream_studio::{
    ObsStatusSnapshot, ObsWebSocketConfig, StreamStudioState, StudioControlIntent,
    execute_obs_intent, query_obs_status_with_autostart, supervisor_socket_path,
    supervisor_socket_present,
};
use aether_vault::VaultRecordKind;

const PERSISTENT_SCROLLBAR: &str = "PERSISTENT_SCROLLBAR=ON";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativePageRoute {
    Home,
    Library,
    StreamStudio,
    Creator,
    Comms,
    Vault,
    Accounts,
    Providers,
    Media,
    Settings,
    Watch,
}

impl NativePageRoute {
    #[must_use]
    pub fn from_url(url: &str) -> Option<Self> {
        let parsed = url::Url::parse(url.trim()).ok()?;
        if parsed.scheme() != "aether" {
            return None;
        }
        match parsed.host_str()?.to_ascii_lowercase().as_str() {
            "home" => Some(Self::Home),
            "library" => Some(Self::Library),
            "stream" => Some(Self::StreamStudio),
            "creator" => Some(Self::Creator),
            "comms" => Some(Self::Comms),
            "vault" => Some(Self::Vault),
            "accounts" => Some(Self::Accounts),
            "providers" => Some(Self::Providers),
            "media" => Some(Self::Media),
            "settings" => Some(Self::Settings),
            "watch" => Some(Self::Watch),
            _ => None,
        }
    }

    #[must_use]
    pub const fn logical_url(self) -> &'static str {
        match self {
            Self::Home => "aether://home",
            Self::Library => "aether://library",
            Self::StreamStudio => "aether://stream",
            Self::Creator => "aether://creator",
            Self::Comms => "aether://comms",
            Self::Vault => "aether://vault",
            Self::Accounts => "aether://accounts",
            Self::Providers => "aether://providers",
            Self::Media => "aether://media",
            Self::Settings => "aether://settings",
            Self::Watch => "aether://watch",
        }
    }

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Home => "Welcome to Aether",
            Self::Library => "Aether Library",
            Self::StreamStudio => "Aether Stream Studio",
            Self::Creator => "Aether Creator Hub",
            Self::Comms => "Aether Comms",
            Self::Vault => "Aether Vault",
            Self::Accounts => "Aether Accounts",
            Self::Providers => "Streaming Providers",
            Self::Media => "Media Compatibility",
            Self::Settings => "Aether Settings",
            Self::Watch => "Aether Native Player",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativePageDocument {
    pub route: NativePageRoute,
    pub logical_url: String,
    pub title: String,
    pub html: String,
}

#[must_use]
pub fn native_page_for_url(url: &str) -> Option<NativePageDocument> {
    let route = NativePageRoute::from_url(url)?;
    if route == NativePageRoute::Watch {
        return Some(render_native_watch(url));
    }
    if route == NativePageRoute::StreamStudio {
        return Some(render_stream_studio_document(url));
    }
    Some(render_native_page(route))
}

#[must_use]
pub fn native_page_for_url_with_library(
    url: &str,
    snapshot: &LibrarySnapshot,
) -> Option<NativePageDocument> {
    let route = NativePageRoute::from_url(url)?;
    if route == NativePageRoute::Watch {
        return Some(render_native_watch(url));
    }
    if route == NativePageRoute::StreamStudio {
        return Some(render_stream_studio_document(url));
    }
    if route == NativePageRoute::Library {
        let parsed = url::Url::parse(url).ok()?;
        let query = parsed
            .query_pairs()
            .find_map(|(key, value)| (key == "q").then(|| value.into_owned()));
        let mut document = NativePageDocument {
            route,
            logical_url: url.to_owned(),
            title: route.title().to_owned(),
            html: String::new(),
        };
        document.html = wrap(
            route,
            route.title(),
            &render_library(snapshot, query.as_deref()),
        );
        return Some(document);
    }
    Some(render_native_page(route))
}

#[must_use]
pub fn render_native_page(route: NativePageRoute) -> NativePageDocument {
    let body = match route {
        NativePageRoute::Home => render_unified_home_marker(),
        NativePageRoute::Library => render_library(&LibrarySnapshot::default(), None),
        NativePageRoute::StreamStudio => render_stream_studio("aether://stream"),
        NativePageRoute::Creator => render_creator_hub(),
        NativePageRoute::Comms => render_comms_hub(),
        NativePageRoute::Vault => render_vault(),
        NativePageRoute::Accounts => render_accounts(),
        NativePageRoute::Providers => render_providers(),
        NativePageRoute::Media => render_media(),
        NativePageRoute::Settings => render_settings_body(),
        NativePageRoute::Watch => render_native_watch_body("unknown", "", ""),
    };
    NativePageDocument {
        route,
        logical_url: route.logical_url().to_owned(),
        title: route.title().to_owned(),
        html: wrap(route, route.title(), &body),
    }
}

fn wrap(_route: NativePageRoute, title: &str, body: &str) -> String {
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta name="color-scheme" content="dark"><title>{title}</title><style>{css}</style></head><body class="native"><div class="scroll-contract">{scroll_contract}</div><div class="shell"><div class="top"><div><div class="eyebrow">AETHERFORGE // NATIVE WORKSPACE</div><h1>{title}</h1></div><div class="badge">DRAGONGLASS NIGHT</div></div><nav class="routebar"><div class="route-primary"><a class="button secondary" href="aether://home">Home</a><a class="button secondary" href="aether://library">Library</a><a class="button secondary" href="aether://stream">Stream Studio</a><a class="button secondary" href="aether://media">Media</a></div><details class="route-more"><summary>More</summary><div class="route-more-menu"><a href="aether://creator">Creator</a><a href="aether://comms">Comms</a><a href="aether://vault">Vault</a><a href="aether://accounts">Accounts</a><a href="aether://providers">Providers</a><a href="aether://settings">Settings</a></div></details></nav>{body}</div></body></html>"#,
        css = NATIVE_PAGE_CSS,
        scroll_contract = PERSISTENT_SCROLLBAR,
    )
}

const NATIVE_PAGE_CSS: &str = r#"
:root{color-scheme:dark;font-family:Inter,'AetherForge Sans',system-ui,sans-serif;background:#03040a;color:#f6f3ff}
*{box-sizing:border-box}
html{min-height:100%;overflow-y:scroll;scrollbar-gutter:stable;scrollbar-color:#7057ff #070812;scrollbar-width:thin;background:#03040a}
body{margin:0;min-height:100vh;color:#f6f3ff;background:radial-gradient(circle at 88% -10%,rgba(99,102,241,.15),transparent 34%),linear-gradient(145deg,#03040a,#090812 58%,#070711)}
body.native{padding:12px}
body::-webkit-scrollbar{width:10px}
body::-webkit-scrollbar-track{background:#070812}
body::-webkit-scrollbar-thumb{background:linear-gradient(#8b5cf6,#4338ca);border:2px solid #070812;border-radius:999px}
.shell{max-width:1440px;margin:auto}.top{display:flex;justify-content:space-between;align-items:flex-end;padding:3px 2px 12px;border-bottom:1px solid rgba(139,92,246,.12);margin-bottom:10px;backdrop-filter:blur(18px)}.eyebrow{font-size:8px;letter-spacing:.18em;color:#756a94;font-weight:800;margin-bottom:4px}h1{margin:0;font-size:22px;letter-spacing:-.02em}.badge{padding:5px 8px;border:1px solid rgba(167,139,250,.24);border-radius:8px;background:rgba(91,33,182,.12);color:#c4b5fd;font-size:8px;letter-spacing:.12em;font-weight:800}
.routebar{display:flex;gap:6px;align-items:center;margin:0 0 11px}.route-primary{display:flex;gap:4px;flex-wrap:wrap;min-width:0}.routebar a{text-decoration:none}.route-more{position:relative;margin-left:auto}.route-more summary{cursor:pointer;list-style:none;padding:7px 10px;border:1px solid rgba(167,139,250,.18);border-radius:8px;background:rgba(24,22,38,.60);color:#c9c0dd;font-size:10px;font-weight:700}.route-more summary::-webkit-details-marker{display:none}.route-more-menu{position:absolute;right:0;top:calc(100% + 6px);z-index:8;display:grid;min-width:148px;padding:6px;border:1px solid rgba(139,92,246,.18);border-radius:10px;background:rgba(8,8,18,.96);box-shadow:0 18px 40px rgba(0,0,0,.36);backdrop-filter:blur(20px)}.route-more-menu a{padding:8px 9px;border-radius:7px;color:#d8d0e9;font-size:10px}.route-more-menu a:hover{background:rgba(91,62,180,.24);color:#fff}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(250px,1fr));gap:8px}.card{position:relative;padding:12px;border-radius:12px;background:linear-gradient(145deg,rgba(18,17,31,.82),rgba(8,8,16,.86));border:1px solid rgba(139,92,246,.16);box-shadow:0 14px 34px rgba(0,0,0,.22),inset 0 1px 0 rgba(255,255,255,.025);backdrop-filter:blur(20px)}.card::before{content:'';position:absolute;left:0;top:10px;bottom:10px;width:2px;border-radius:4px;background:linear-gradient(#8b5cf6,#4338ca);opacity:.62}.card h2{margin:0 0 7px;font-size:13px;color:#ddd6fe}.muted{color:#9288a8;font-size:11px;line-height:1.48}.ok{color:#86efac}.warn{color:#fde68a}.row{display:flex;gap:6px;flex-wrap:wrap;margin-top:10px}.library-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(420px,1fr));gap:10px}.library-head{display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:10px}.library-head form{display:flex;gap:6px;min-width:min(520px,52vw)}.library-head input{min-width:0;flex:1;background:rgba(6,7,16,.78);border:1px solid rgba(139,92,246,.25);border-radius:8px;color:#f6f3ff;padding:8px 10px}.library-row{display:flex;align-items:center;justify-content:space-between;gap:10px;padding:9px 0;border-top:1px solid rgba(139,92,246,.10)}.library-row:first-of-type{border-top:0}.library-section-head{display:flex;justify-content:space-between;align-items:center;gap:10px}button,.button{appearance:none;border:1px solid rgba(167,139,250,.25);background:linear-gradient(135deg,rgba(111,62,190,.67),rgba(55,48,138,.60));color:white;padding:7px 10px;border-radius:8px;font-size:10px;font-weight:700;box-shadow:inset 0 1px 0 rgba(255,255,255,.04);text-decoration:none}.secondary{background:rgba(24,22,38,.67);color:#c9c0dd}ul{padding-left:17px;color:#b9b0c8;font-size:11px;line-height:1.58}code{color:#c4b5fd;font-size:10px}p{font-size:11px;line-height:1.48}b{color:#ddd6fe}
.scroll-contract{display:none}


.native-player{display:grid;grid-template-rows:auto minmax(300px,1fr) auto;gap:10px;min-height:calc(100vh - 110px)}.native-stage{position:relative;display:grid;place-items:center;overflow:hidden;border-radius:14px;background:#010207;border:1px solid rgba(139,92,246,.25);box-shadow:inset 0 0 80px rgba(67,56,202,.10)}#aether-native-video{display:block;width:100%;height:auto;max-height:calc(100vh - 220px);object-fit:contain;background:#000}.native-overlay{position:absolute;left:12px;top:12px;padding:6px 9px;border-radius:8px;background:rgba(3,4,10,.78);border:1px solid rgba(139,92,246,.22);font-size:10px;color:#ddd6fe}.native-controls{display:flex;gap:8px;align-items:center;flex-wrap:wrap}.native-source{font-size:10px;color:#8f86a4;overflow-wrap:anywhere}
@media(max-width:900px){.grid{grid-template-columns:1fr}}


"#;

fn render_unified_home_marker() -> String {
    r#"<div class="card"><h2>Unified browser home</h2><p class="muted">The interactive home composition is owned by the Aether Browser unified UI surface.</p></div>"#.to_owned()
}

fn render_stream_studio_document(url: &str) -> NativePageDocument {
    let route = NativePageRoute::StreamStudio;
    NativePageDocument {
        route,
        logical_url: route.logical_url().to_owned(),
        title: route.title().to_owned(),
        html: wrap(route, route.title(), &render_stream_studio(url)),
    }
}

fn render_stream_studio(url: &str) -> String {
    let service_state = StreamStudioState::canonical();
    let (intent, action_notice) = parse_stream_studio_intent(url);
    let config = ObsWebSocketConfig::from_env();
    let obs_result = match intent {
        Some(intent) => execute_obs_intent(config, intent),
        None => query_obs_status_with_autostart(config),
    };

    let (obs_card, scene_card, mixer_card) = match obs_result {
        Ok(snapshot) => render_obs_connected(&snapshot, action_notice.as_deref()),
        Err(error) => render_obs_offline(&error.to_string(), action_notice.as_deref()),
    };

    format!(
        r#"<div class="grid">
{obs_card}
{scene_card}
{mixer_card}
<div class="card"><h2>Aether Studio // BrowserAuthoritative</h2><p class="muted">Browser UI authority: <b>{:?}</b></p><p class="muted">Standalone GUI: <b>{:?}</b></p><p>Supervisor socket: <code>{}</code></p><p class="{}">{}</p><p class="muted">The Full OBS Workspace embeds the real installed OBS Studio window directly inside this AetherBrowser tab. That preserves OBS-native preview, Scenes, Sources, Audio Mixer, transitions, Studio Mode, filters, properties, docks/plugins, profiles, scene collections, recording, streaming, replay buffer, virtual camera and settings. OBS WebSocket v5 remains the typed compatibility/control plane for Aether integrations.</p><div class="row"><a class="button" href="aether://external-app/open?id=obs">Open Full OBS Workspace</a><a class="button secondary" href="aether://external-app/open?id=obs&amp;mode=detached">Open OBS Detached</a><a class="button secondary" href="aether://creator">Streamlabs / StreamElements</a></div><p class="muted">OBS is contained inside AetherBrowser by default. Explicit detach is session-only; the next AetherBrowser run adopts OBS back into the main window. Capture Safety is STRICT_NO_RECURSION: whole-monitor/self-window capture that would include AetherBrowser, OBS, or Streamlabs is blocked while embedded, and overflowing scene items are clamped to the configured OBS canvas.</p></div>
<div class="card"><h2>Provider compatibility</h2><p class="muted"><b>OBS Studio 32.2.2</b> is the authoritative vendor compatibility baseline for the Full OBS Workspace; the installed native Linux OBS remains the runtime engine. YouTube, YouTube Music, Twitch, Streamlabs and StreamElements account surfaces use the persistent Chromium Compatibility engine inside AetherBrowser. Closing the Aether window does not log out normal profiles. Streamlabs surfaces inherit STRICT_NO_RECURSION capture safety and viewport clipping so the browser/preview cannot feed itself back into the scene.</p></div>
</div>"#,
        service_state.authority,
        service_state.standalone_gui,
        supervisor_socket_path().display(),
        if supervisor_socket_present() {
            "ok"
        } else {
            "warn"
        },
        if supervisor_socket_present() {
            "Supervisor socket detected"
        } else {
            "Supervisor socket offline"
        },
    )
}

fn parse_stream_studio_intent(url: &str) -> (Option<StudioControlIntent>, Option<String>) {
    let Ok(parsed) = url::Url::parse(url) else {
        return (
            None,
            Some("Invalid Stream Studio URL; refreshed OBS status only.".to_owned()),
        );
    };
    let mut action = None::<String>;
    let mut scene = None::<String>;
    let mut channel = None::<String>;
    let mut millidb = None::<i32>;
    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "action" => action = Some(value.into_owned()),
            "name" => scene = Some(value.into_owned()),
            "channel" => channel = Some(value.into_owned()),
            "millidb" => millidb = value.parse::<i32>().ok(),
            _ => {}
        }
    }
    let Some(action) = action else {
        return (None, None);
    };
    match action.as_str() {
        "refresh" => (None, Some("OBS status refreshed.".to_owned())),
        "start-stream" => (
            Some(StudioControlIntent::StartStream),
            Some("Start stream requested.".to_owned()),
        ),
        "stop-stream" => (
            Some(StudioControlIntent::StopStream),
            Some("Stop stream requested.".to_owned()),
        ),
        "start-record" => (
            Some(StudioControlIntent::StartRecording),
            Some("Start recording requested.".to_owned()),
        ),
        "stop-record" => (
            Some(StudioControlIntent::StopRecording),
            Some("Stop recording requested.".to_owned()),
        ),
        "save-replay" => (
            Some(StudioControlIntent::SaveReplay),
            Some("Save replay requested.".to_owned()),
        ),
        "scene" => match scene.filter(|value| !value.trim().is_empty()) {
            Some(scene) => (
                Some(StudioControlIntent::ActivateScene(scene)),
                Some("Scene activation requested.".to_owned()),
            ),
            None => (
                None,
                Some("Scene action ignored because no scene name was supplied.".to_owned()),
            ),
        },
        "mixer-gain" => match (channel.filter(|value| !value.trim().is_empty()), millidb) {
            (Some(channel), Some(gain_millidb)) => (
                Some(StudioControlIntent::SetMixerGain {
                    channel,
                    gain_millidb,
                }),
                Some("Mixer gain update requested.".to_owned()),
            ),
            _ => (
                None,
                Some("Mixer action ignored because channel or millidb was missing.".to_owned()),
            ),
        },
        _ => (
            None,
            Some("Unknown Stream Studio action ignored; OBS status refreshed.".to_owned()),
        ),
    }
}

fn render_obs_connected(
    snapshot: &ObsStatusSnapshot,
    notice: Option<&str>,
) -> (String, String, String) {
    let notice = notice.map_or_else(String::new, |value| {
        format!(r#"<p class="muted">{}</p>"#, html_escape(value))
    });
    let stream_action = if snapshot.streaming {
        "stop-stream"
    } else {
        "start-stream"
    };
    let stream_label = if snapshot.streaming {
        "Stop Stream"
    } else {
        "Start Stream"
    };
    let record_action = if snapshot.recording {
        "stop-record"
    } else {
        "start-record"
    };
    let record_label = if snapshot.recording {
        "Stop Recording"
    } else {
        "Start Recording"
    };
    let current_scene = snapshot.current_scene.as_deref().unwrap_or("None");
    let scene_buttons = if snapshot.scenes.is_empty() {
        r#"<p class="muted">OBS returned no scenes.</p>"#.to_owned()
    } else {
        snapshot
            .scenes
            .iter()
            .map(|scene| {
                let href = stream_action_url(&[("action", "scene"), ("name", scene)]);
                let class = if snapshot.current_scene.as_deref() == Some(scene.as_str()) {
                    "button"
                } else {
                    "button secondary"
                };
                format!(
                    r#"<a class="{class}" href="{}">{}</a>"#,
                    html_escape(&href),
                    html_escape(scene)
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    let obs_card = format!(
        r#"<div class="card"><h2>OBS WebSocket</h2><p class="ok">CONNECTED</p><p class="muted">OBS {} · WebSocket {} · RPC {}</p><p>Current scene: <b>{}</b></p><p>Streaming: <b>{}</b> · Recording: <b>{}</b> · Replay buffer: <b>{}</b></p>{notice}<div class="row"><a class="button secondary" href="aether://stream?action=refresh">Refresh</a><a class="button" href="aether://stream?action={stream_action}">{stream_label}</a><a class="button secondary" href="aether://stream?action={record_action}">{record_label}</a><a class="button secondary" href="aether://stream?action=save-replay">Save Replay</a></div></div>"#,
        html_escape(&snapshot.obs_version),
        html_escape(&snapshot.websocket_version),
        snapshot.rpc_version,
        html_escape(current_scene),
        if snapshot.streaming { "LIVE" } else { "OFF" },
        if snapshot.recording { "ACTIVE" } else { "OFF" },
        if snapshot.replay_buffer {
            "ACTIVE"
        } else {
            "OFF"
        },
    );
    let scene_card = format!(
        r#"<div class="card"><h2>Scenes</h2><p class="muted">Current scene: <b>{}</b></p><div class="row">{scene_buttons}</div></div>"#,
        html_escape(current_scene),
    );
    let mixer_card = r#"<div class="card"><h2>Mixer</h2><p class="muted">Set an OBS input gain in millidecibels (for example -8000 = -8 dB).</p><form action="aether://stream" method="get"><input type="hidden" name="action" value="mixer-gain"><input name="channel" placeholder="OBS input name" required><input name="millidb" type="number" value="-8000" required><div class="row"><button type="submit">Set Input Gain</button></div></form></div>"#.to_owned();
    (obs_card, scene_card, mixer_card)
}

fn render_obs_offline(error: &str, notice: Option<&str>) -> (String, String, String) {
    let notice = notice.map_or_else(String::new, |value| {
        format!(r#"<p class="muted">{}</p>"#, html_escape(value))
    });
    let obs_card = format!(
        r#"<div class="card"><h2>OBS WebSocket</h2><p class="warn">OFFLINE</p><p class="muted">Default endpoint: <code>ws://127.0.0.1:4455</code>. Override with <code>AETHER_OBS_WEBSOCKET_URL</code>. If OBS authentication is enabled, provide <code>AETHER_OBS_WEBSOCKET_PASSWORD</code> to the browser process; the password is not rendered or logged.</p><p class="warn">{}</p>{notice}<div class="row"><a class="button secondary" href="aether://stream?action=refresh">Retry</a></div></div>"#,
        html_escape(error),
    );
    let scene_card = r#"<div class="card"><h2>Scenes</h2><p class="muted">Current scene: unavailable until OBS WebSocket connects.</p><div class="row"><a class="button secondary" href="aether://stream?action=scene&amp;name=Gaming">Gaming</a></div></div>"#.to_owned();
    let mixer_card = r#"<div class="card"><h2>Mixer</h2><p class="muted">OBS input control becomes available after connection.</p><form action="aether://stream" method="get"><input type="hidden" name="action" value="mixer-gain"><input name="channel" placeholder="OBS input name" required><input name="millidb" type="number" value="-8000" required><div class="row"><button type="submit">Set Input Gain</button></div></form></div>"#.to_owned();
    (obs_card, scene_card, mixer_card)
}

fn stream_action_url(params: &[(&str, &str)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in params {
        serializer.append_pair(key, value);
    }
    format!("aether://stream?{}", serializer.finish())
}

fn render_authoritative_vendor_baselines() -> String {
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/home/benji"));
    let packages = authoritative_vendor_packages()
        .iter()
        .map(|package| {
            let version = package.expected_version.unwrap_or("hash-pinned latest channel");
            let status = package_reference_status(&home, package);
            format!(
                "<li><b>{}</b> · baseline <code>{version}</code> · <code>{status}</code> · policy <code>{:?}</code></li>",
                package.display_name, package.execution_policy
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let ground_actions = ground_control_alert_actions().join(" · ");
    format!(
        r#"<div class="card"><h2>Authoritative Vendor Baselines</h2><p class="ok">PACKAGE-ALIGNED // REFERENCE-ONLY</p><p class="muted">AetherBrowser aligns its embedded creator workspaces to the user's exact vendor installers while keeping execution native to Linux/Rust. Vendor installers and platform binaries are never launched by the importer.</p><ul>{packages}</ul><p class="muted"><b>OBS Studio 32.2.2</b> pins Full OBS Workspace behavior. <b>Streamlabs Desktop 1.21.9</b> pins the Streamlabs integration baseline. <b>Ground Control 2.1.20</b> pins StreamElements Ground Control action naming.</p><p class="muted">Ground Control actions: {ground_actions}</p></div>"#
    )
}

fn render_creator_hub() -> String {
    let twitch = twitch_descriptor();
    let capability_list = twitch
        .capabilities
        .iter()
        .map(|capability| {
            let label = match capability {
                CreatorCapability::StreamInfo => "Stream Info",
                CreatorCapability::Chat => "Chat",
                CreatorCapability::Moderation => "Moderation",
                CreatorCapability::Clips => "Clips",
                CreatorCapability::Raids => "Raids",
                CreatorCapability::Polls => "Polls",
                CreatorCapability::Predictions => "Predictions",
                CreatorCapability::ChannelPoints => "Channel Points",
                CreatorCapability::Followers => "Followers",
                CreatorCapability::Subscribers => "Subscribers",
                CreatorCapability::Cheers => "Cheers",
                CreatorCapability::StreamHealth => "Stream Health",
                CreatorCapability::RealtimeEvents => "Realtime Events",
                CreatorCapability::Alerts => "Alerts",
                CreatorCapability::Donations => "Donations",
                CreatorCapability::MediaShare => "Media Share",
                CreatorCapability::BrowserSources => "Browser Sources",
                CreatorCapability::Overlays => "Overlays",
                CreatorCapability::Chatbot => "Chatbot",
                CreatorCapability::SocialPost => "Social Post",
                CreatorCapability::SocialFeed => "Social Feed",
                CreatorCapability::DirectMessages => "Direct Messages",
            };
            format!("<li>{label}</li>")
        })
        .collect::<Vec<_>>()
        .join("");
    let streamlabs = streamlabs_descriptor();
    let stream_elements = stream_elements_descriptor();
    let se_topics = STREAMELEMENTS_TOPICS.join(" · ");
    let bluesky = bluesky_descriptor();
    let bluesky_lexicons = BLUESKY_LEXICONS.join(" · ");
    let vendor_baselines = render_authoritative_vendor_baselines();
    format!(
        r#"<div class="grid"><div class="card"><h2>Twitch Creator</h2><p class="muted">Official OAuth + Helix API + EventSub integration. Tokens remain AetherForge secret references and are never serialized into profiles.</p><div class="row"><a class="button" href="aether://provider/open?id=twitch">Open Twitch in New Tab</a></div><ul>{capability_list}</ul></div><div class="card"><h2>Streamlabs Suite // BrowserAuthoritative</h2><p class="muted">Integrated import/account surface for Streamlabs Desktop scenes, browser sources, alerts, donations, Media Share, overlays, chatbot, realtime Socket events and cloud-only account services. The account dashboard stays signed in through the persistent Chromium Compatibility profile.</p><div class="row"><a class="button" href="{}">Open Integrated Streamlabs</a><a class="button secondary" href="aether://stream">Aether Studio</a></div><p class="muted">Realtime endpoint: <code>{}</code></p></div><div class="card"><h2>StreamElements Suite // BrowserAuthoritative</h2><p class="muted">Integrated overlay/import/account surface for AlertBox, custom HTML/CSS/JS widgets, browser sources, tips, activities, chat, chatbot workflows and Astro realtime events. ProviderCloud-only services remain on the provider account while the local production UI stays in AetherBrowser.</p><div class="row"><a class="button" href="{}">Open Integrated StreamElements</a><a class="button secondary" href="aether://stream">Aether Studio</a></div><p class="muted">Astro: <code>{}</code></p><p class="muted">Topics: {se_topics}</p></div><div class="card"><h2>Bluesky / AT Protocol</h2><p class="muted">OAuth-first AT Protocol integration with PKCE + PAR + DPoP, public feeds, posts, replies/reposts/likes, notifications, media, and chat.bsky direct-message capability when explicitly authorized.</p><div class="row"><a class="button" href="{}">Open Bluesky</a></div><p class="muted">Lexicons: {bluesky_lexicons}</p></div><div class="card"><h2>StreamElements Ground Control // Browser Integrated</h2><p class="muted">The user-owned Ground Control 2.1.20 MSI is a reference-only compatibility input. Its alert-control vocabulary is normalized into AetherBrowser naming instead of running the Windows application.</p><div class="row"><a class="button" href="{}">Open StreamElements</a></div></div><div class="card"><h2>Creator provider status</h2><p class="warn">Account authorization requires registered provider configuration or provider-discovered OAuth metadata.</p><p class="muted">Twitch realtime: <code>{}</code></p></div>{vendor_baselines}</div>"#,
        streamlabs.official_web_url,
        streamlabs.realtime_url.unwrap_or("N/A"),
        stream_elements.official_web_url,
        stream_elements.realtime_url.unwrap_or("N/A"),
        bluesky.official_web_url,
        stream_elements.official_web_url,
        twitch.realtime_url.unwrap_or("N/A")
    )
}

fn render_comms_hub() -> String {
    let mut inbox = UnifiedInbox::default();
    inbox.accounts.push(CommsAccount::generic_imap_smtp(
        "Generic IMAP/SMTP",
        CommsSecretRef::new("comms/generic/account"),
    ));
    inbox.accounts.push(zoho_mail_account("Zoho Mail"));
    inbox.accounts.push(proton_mail_account("Proton Mail"));

    let registry = CommsProviderRegistry::canonical();
    let provider_cards = registry
        .providers()
        .iter()
        .map(|provider| {
            let link = provider.official_web_url.map_or_else(
                || String::from("<span class=\"badge\">NATIVE ACCOUNT</span>"),
                |url| format!("<a class=\"button secondary\" href=\"{url}\">Open {}</a>", provider.display_name),
            );
            format!(
                "<div class=\"card\"><h2>{}</h2><p class=\"muted\">Transport: {:?}</p><p class=\"ok\">Privacy: {:?}</p><div class=\"row\">{link}</div></div>",
                provider.display_name, provider.transport, provider.privacy
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let privacy = CapturePrivacy::HidePrivateWhileLive;
    format!(
        r#"<div class="grid"><div class="card"><h2>Unified Communications Hub</h2><p class="muted">Aether Comms unifies mail, DMs, creator chat, alerts, and provider notifications while preserving each provider's authorization boundary.</p><div class="row"><a class="button" href="aether://comms?action=add-mail">Add Mail Account</a><a class="button secondary" href="aether://comms?action=connect-provider">Connect Provider</a><a class="button secondary" href="aether://creator">Creator Hub</a></div></div><div class="card"><h2>Unified Inbox</h2><p class="muted">{} configured mail account(s) + {} registered communication provider(s).</p><p class="muted">Generic mail uses IMAP/SMTP; Zoho uses OAuth/API with mail fallback; Proton uses the official local Bridge boundary.</p></div><div class="card"><h2>Zoho Mail</h2><p class="muted">Native OAuth/API account with secure IMAP/SMTP fallback. OAuth tokens and client credentials remain secret-store references.</p><div class="row"><a class="button" href="{ZOHO_MAIL_WEB_URL}">Open Zoho Mail / Sign In</a></div></div><div class="card"><h2>Proton Mail + Bridge</h2><p class="muted">Managed official Proton Mail Bridge boundary. Bridge performs encryption/decryption locally and exposes local IMAP/SMTP endpoints to Aether Comms.</p><div class="row"><a class="button" href="{PROTON_MAIL_WEB_URL}">Open Proton Mail</a><a class="button secondary" href="{PROTON_BRIDGE_URL}">Bridge</a></div><p class="warn">Bridge requires an eligible paid Proton Mail plan.</p></div><div class="card"><h2>Streamer Privacy</h2><p class="ok">{:?}</p><p class="muted">When LIVE, private subjects, message bodies, direct messages, and account identifiers stay out of capture-safe activity by default.</p></div></div><div class="grid">{provider_cards}</div>"#,
        inbox.accounts.len(),
        registry.provider_count(),
        privacy
    )
}

fn render_vault() -> String {
    let kinds = [
        VaultRecordKind::WebsitePassword,
        VaultRecordKind::PasskeyReference,
        VaultRecordKind::OAuthToken,
        VaultRecordKind::StreamKey,
        VaultRecordKind::RecoveryCode,
        VaultRecordKind::SecureNote,
    ];
    let items = kinds
        .iter()
        .map(|kind| format!("<li>{kind:?}</li>"))
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<div class="grid"><div class="card"><h2>Vault locked</h2><p class="muted">Argon2id key derivation + XChaCha20-Poly1305 authenticated encryption.</p><div class="row"><a class="button" href="aether://vault?action=unlock">Unlock Vault</a><a class="button secondary" href="aether://vault?action=add-login">Add Login</a><a class="button secondary" href="aether://vault?action=generate-password">Generate Password</a></div></div><div class="card"><h2>Protected record classes</h2><ul>{items}</ul></div><div class="card"><h2>Origin protection</h2><p class="ok">Website autofill requires exact origin matching.</p><p class="muted">Web pages never receive direct vault access.</p></div></div>"#
    )
}

fn render_accounts() -> String {
    let providers = [
        (IdentityProvider::Twitch, "Sign in with Twitch", "twitch"),
        (IdentityProvider::Google, "Sign in with Google", "google"),
        (IdentityProvider::Apple, "Sign in with Apple", "apple"),
        (IdentityProvider::Local, "Use local Aether profile", "local"),
    ];
    let cards = providers
        .iter()
        .map(|(provider, label, id)| {
            let note = if *provider == IdentityProvider::Local {
                "Ready locally"
            } else {
                "Official provider sign-in opens through the native Aether route dispatcher"
            };
            format!(
                "<div class=\"card\"><h2>{provider:?}</h2><p class=\"muted\">{note}</p><a class=\"button\" href=\"aether://auth/start?provider={id}\">{label}</a></div>"
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!("<div class=\"grid\">{cards}</div>")
}

fn render_provider_card(provider: &ProviderManifest) -> String {
    let id = html_escape(&provider.id);
    format!(
        "<div class=\"card\"><h2>{}</h2><p class=\"muted\">{:?} · {:?}</p><p>{} capabilities</p><div class=\"row\"><a class=\"button\" href=\"aether://provider/open?id={id}\">Open</a><a class=\"button secondary\" href=\"aether://provider/configure?id={id}\">Configure</a></div></div>",
        provider.name,
        provider.kind,
        provider.auth,
        provider.capabilities.len(),
    )
}

fn render_providers() -> String {
    let cards = built_in_providers()
        .iter()
        .map(render_provider_card)
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<div class="card"><h2>Production integrations</h2><p class="muted">OBS Studio, Streamlabs and StreamElements are BrowserAuthoritative Aether Studio integrations; provider account pages use the persistent Chromium Compatibility engine.</p></div><div class="grid">{cards}</div>"#
    )
}

fn render_library(snapshot: &LibrarySnapshot, query: Option<&str>) -> String {
    let search = html_escape(query.unwrap_or_default());
    let history = if snapshot.history.is_empty() {
        "<p class=\"muted\">No matching history yet.</p>".to_owned()
    } else {
        snapshot.history.iter().map(|row| {
            let label = if row.title.trim().is_empty() { &row.url } else { &row.title };
            format!(
                "<div class=\"library-row\"><div><b>{}</b><div class=\"muted\">{}</div></div><a class=\"button secondary\" href=\"{}\">OPEN</a></div>",
                html_escape(label), html_escape(&row.url), html_escape(&row.url)
            )
        }).collect::<Vec<_>>().join("")
    };
    let bookmarks = if snapshot.bookmarks.is_empty() {
        "<p class=\"muted\">No matching bookmarks yet. Use the star beside the omnibox.</p>"
            .to_owned()
    } else {
        snapshot.bookmarks.iter().map(|row| {
            let tags = if row.tags.is_empty() { String::new() } else { format!(" · {}", html_escape(&row.tags.join(", "))) };
            format!(
                "<div class=\"library-row\"><div><b>★ {}</b><div class=\"muted\">{} · {}{}</div></div><div class=\"row\"><a class=\"button secondary\" href=\"{}\">OPEN</a><a class=\"button secondary\" href=\"aether://library?action=remove-bookmark&amp;id={}\">REMOVE</a></div></div>",
                html_escape(&row.title), html_escape(&row.url), html_escape(&row.folder), tags, html_escape(&row.url), row.id
            )
        }).collect::<Vec<_>>().join("")
    };
    let downloads = if snapshot.downloads.is_empty() {
        "<p class=\"muted\">No persisted downloads yet.</p>".to_owned()
    } else {
        snapshot.downloads.iter().map(|row| {
            let progress = row.total_bytes.map_or_else(
                || format!("{} bytes", row.received_bytes),
                |total| format!("{} / {} bytes", row.received_bytes, total),
            );
            format!(
                "<div class=\"library-row\"><div><b>{}</b><div class=\"muted\">{} · {}</div></div><span class=\"badge\">{}</span></div>",
                html_escape(&row.destination), html_escape(&row.source_url), progress, html_escape(&row.state)
            )
        }).collect::<Vec<_>>().join("")
    };
    let recently_closed = if snapshot.recently_closed.is_empty() {
        "<p class=\"muted\">No recently closed tabs yet.</p>".to_owned()
    } else {
        snapshot.recently_closed.iter().map(|row| {
            let label = if row.title.trim().is_empty() { &row.url } else { &row.title };
            let encoded: String = url::form_urlencoded::byte_serialize(row.url.as_bytes()).collect();
            format!(
                "<div class=\"library-row\"><div><b>{}</b><div class=\"muted\">{}</div></div><a class=\"button secondary\" href=\"aether://library?action=reopen&amp;id={}&amp;url={}\">REOPEN</a></div>",
                html_escape(label), html_escape(&row.url), row.id, encoded
            )
        }).collect::<Vec<_>>().join("")
    };
    format!(
        r#"<section class="library-head card"><div><h2>Aether Library</h2><p class="muted">Persistent browser memory. Private browsing is excluded by contract.</p></div><form method="get" action="aether://library"><input name="q" value="{search}" placeholder="Search history, bookmarks, downloads, closed tabs"><button>SEARCH</button></form></section>
<div class="row"><a class="button secondary" href="aether://library?section=history">History</a><a class="button secondary" href="aether://library?section=bookmarks">Bookmarks</a><a class="button secondary" href="aether://library?section=downloads">Downloads</a><a class="button secondary" href="aether://library?section=recently-closed">Recently Closed</a></div><div class="library-grid">
<section id="history" class="card"><div class="library-section-head"><h2>History</h2><a class="button secondary" href="aether://library?action=clear-history">CLEAR</a></div>{history}</section>
<section id="bookmarks" class="card"><h2>Bookmarks</h2>{bookmarks}</section>
<section id="downloads" class="card"><h2>Downloads</h2>{downloads}</section>
<section id="recently-closed" class="card"><div class="library-section-head"><h2>Recently Closed</h2><a class="button secondary" href="aether://library?action=clear-recent">CLEAR</a></div>{recently_closed}</section>
</div>"#
    )
}

fn render_native_watch(url: &str) -> NativePageDocument {
    let parsed = url::Url::parse(url).expect("aether watch route was already parsed");
    let mut provider = String::from("unknown");
    let mut resource = String::new();
    let mut original_url = String::new();
    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "provider" => provider = value.into_owned(),
            "resource" => resource = value.into_owned(),
            "url" => original_url = value.into_owned(),
            _ => {}
        }
    }
    let title = match provider.as_str() {
        "youtube" => "YouTube // Aether Native Player",
        "twitch" => "Twitch // Aether Native Player",
        _ => "Aether Native Player",
    };
    NativePageDocument {
        route: NativePageRoute::Watch,
        logical_url: url.to_owned(),
        title: title.to_owned(),
        html: wrap(
            NativePageRoute::Watch,
            title,
            &render_native_watch_body(&provider, &resource, &original_url),
        ),
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_native_watch_body(provider: &str, resource: &str, original_url: &str) -> String {
    let servo_url = url::Url::parse(original_url).map_or_else(
        |_| original_url.to_owned(),
        |mut parsed| {
            parsed.query_pairs_mut().append_pair("aether_web", "1");
            parsed.to_string()
        },
    );
    format!(
        r#"<section class="native-player" data-provider="{}" data-resource="{}">
<div class="card"><h2>Aether Native Player</h2><p class="muted">{} playback is isolated from Servo media decode. The page engine renders this DragonGlass surface; aether-media-service owns the media session.</p></div>
<div class="native-stage"><img id="aether-native-video" alt="Native playback frame"/><div class="native-overlay" id="aether-native-state">Connecting to native media service…</div></div>
<div class="card native-controls"><a class="button" id="aether-native-play" href="aether://watch?action=play&amp;provider={}&amp;resource={}&amp;url={}">Play / Retry</a><a class="button secondary" id="aether-native-stop" href="aether://watch?action=stop&amp;provider={}&amp;resource={}&amp;url={}">Stop</a><a class="button secondary" href="{}">Open Source Page</a><span class="native-source">{}</span></div>
</section>"#,
        html_escape(provider),
        html_escape(resource),
        html_escape(provider),
        html_escape(provider),
        html_escape(resource),
        html_escape(original_url),
        html_escape(provider),
        html_escape(resource),
        html_escape(original_url),
        html_escape(&servo_url),
        html_escape(original_url),
    )
}

fn render_settings_body() -> String {
    r#"<div class="grid">
<div class="card"><h2>Browser</h2><p class="muted">Native Aether Browser configuration routes.</p><div class="row"><a class="button" href="aether://accounts">Accounts &amp; Sign-ins</a><a class="button secondary" href="aether://providers">Providers</a></div></div>
<div class="card"><h2>Privacy &amp; credentials</h2><div class="row"><a class="button" href="aether://vault">Vault</a><a class="button secondary" href="aether://library">Library</a></div></div>
<div class="card"><h2>Communications &amp; media</h2><div class="row"><a class="button" href="aether://comms">Comms</a><a class="button secondary" href="aether://media">Media</a><a class="button secondary" href="aether://youtube-music">YouTube Music</a></div></div>
<div class="card"><h2>Interface</h2><p class="ok">52 px launcher · 176/38 px utility rail · resizable DragonGlass shell.</p><p class="muted">Drag any outer edge or corner to resize. The Servo content surface follows the same authoritative viewport geometry.</p></div>
</div>"#.to_owned()
}

fn render_media() -> String {
    let matrix = MediaSupportMatrix::gstreamer_unverified();
    format!(
        r#"<div class="grid"><div class="card"><h2>Servo media backend</h2><p class="ok">{:?}</p><p class="muted">GStreamer-backed audio/video/WebRTC plumbing is compiled into Servo.</p></div><div class="card"><h2>YouTube</h2><p class="warn">{:?}</p><p>Native playback uses the current yt-dlp resolver and Aether/GStreamer player.</p><div class="row"><a class="button" href="aether://provider/open?id=youtube-live">Open YouTube in New Tab</a></div></div><div class="card"><h2>Twitch</h2><p class="warn">{:?}</p><p>Live channels use current Streamlink stdout into the native GStreamer player; VODs remain authenticated Servo web playback.</p><div class="row"><a class="button" href="aether://provider/open?id=twitch">Open Twitch in New Tab</a></div></div><div class="card"><h2>Runtime proof</h2><p class="muted">Ctrl+Shift+M probes the active media path while provider tabs keep independent navigation and authenticated profile state.</p></div></div>"#,
        matrix.backend, matrix.youtube, matrix.twitch,
    )
}
