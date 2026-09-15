#![forbid(unsafe_code)]

use aether_browser::{
    LaunchMode, NATIVE_DARK_MODE_STATUS, NATIVE_MEDIA_PROVIDER_TWITCH_STATUS,
    NATIVE_MEDIA_PROVIDER_YOUTUBE_STATUS, WEB_THEME_STATUS, parse_launch_args,
};
use aether_engine_servo::{ServoEngineAdapter, run_live_browser};
use aether_media_service::{MediaRendererMode, ensure_media_service, probe_media_service};
use aether_storage::LibraryStore;
use aether_stream_providers::built_in_providers;
use aether_ui::DragonGlassTokens;

const VERSION_LINE: &str = "Aether Browser 2.1.60";
const ARCHITECTURE_LINE: &str = "AETHER_BROWSER_ARCHITECTURE=rust-native";

fn print_media_status() {
    let service = probe_media_service();
    println!("AETHER_BROWSER_MEDIA_BACKEND=GSTREAMER");
    println!("AETHER_BROWSER_MEDIA_GSTREAMER_SERVO_FEATURE=ENABLED");
    println!("AETHER_BROWSER_MEDIA_SAFE_RENDERER=CPU_BGRA");
    println!(
        "AETHER_BROWSER_MEDIA_RENDERER_ACTIVE={}",
        MediaRendererMode::from_env().as_str()
    );
    println!("AETHER_BROWSER_MEDIA_SERVICE=READY");
    println!("AETHER_BROWSER_MEDIA_SERVICE_PROTOCOL=2");
    println!(
        "AETHER_BROWSER_MEDIA_SERVICE_ONLINE={}",
        if service.online { "YES" } else { "NO" }
    );
    println!(
        "AETHER_BROWSER_MEDIA_SERVICE_SOCKET={}",
        service.socket_path.display()
    );
    println!("AETHER_BROWSER_MEDIA_DECODE_ISOLATION=ACTIVE_FOR_NATIVE_PROVIDERS");
    println!("AETHER_BROWSER_MEDIA_CURRENT_SERVO_DECODE=BYPASSED_FOR_NATIVE_PROVIDERS");
    println!("{NATIVE_MEDIA_PROVIDER_YOUTUBE_STATUS}");
    println!("{NATIVE_MEDIA_PROVIDER_TWITCH_STATUS}");
    println!("{NATIVE_DARK_MODE_STATUS}");
    println!("{WEB_THEME_STATUS}");
    println!("AETHER_BROWSER_MEDIA_AUDIO=BACKEND_READY");
    println!("AETHER_BROWSER_MEDIA_VIDEO=BACKEND_READY");
    println!("AETHER_BROWSER_WEBRTC=BACKEND_READY");
    println!("AETHER_BROWSER_MEDIA_FULLSCREEN=MODEL_READY");
    println!("AETHER_BROWSER_MEDIA_PIP=MODEL_READY");
    println!("AETHER_BROWSER_MEDIA_MSE=UNVERIFIED");
    println!("AETHER_BROWSER_YOUTUBE_PLAYBACK=UNVERIFIED");
    println!("AETHER_BROWSER_TWITCH_PLAYBACK=UNVERIFIED");
}

fn print_library_status() {
    match LibraryStore::open_default() {
        Ok(store) => {
            let schema = store.schema_version().map(|version| version.0).unwrap_or(0);
            let snapshot = store.snapshot(None, 500);
            let self_test = store.self_test();
            println!("AETHER_BROWSER_LIBRARY=READY");
            println!("AETHER_BROWSER_LIBRARY_SCHEMA={schema}");
            match snapshot {
                Ok(snapshot) => {
                    println!("AETHER_BROWSER_LIBRARY_HISTORY={}", snapshot.history.len());
                    println!(
                        "AETHER_BROWSER_LIBRARY_BOOKMARKS={}",
                        snapshot.bookmarks.len()
                    );
                    println!(
                        "AETHER_BROWSER_LIBRARY_DOWNLOADS={}",
                        snapshot.downloads.len()
                    );
                    println!(
                        "AETHER_BROWSER_LIBRARY_RECENTLY_CLOSED={}",
                        snapshot.recently_closed.len()
                    );
                }
                Err(error) => println!("AETHER_BROWSER_LIBRARY_SNAPSHOT=FAIL:{error}"),
            }
            println!(
                "AETHER_BROWSER_LIBRARY_SELF_TEST={}",
                if self_test.is_ok() { "PASS" } else { "FAIL" }
            );
        }
        Err(error) => {
            println!("AETHER_BROWSER_LIBRARY=FAIL:{error}");
            println!("AETHER_BROWSER_LIBRARY_SELF_TEST=FAIL");
        }
    }
}

fn print_status() {
    let engine = ServoEngineAdapter::new();
    let theme = DragonGlassTokens::canonical();
    println!("AETHER_BROWSER_ENGINE=chromium-x11");
    println!("AETHER_BROWSER_INTERNAL_ENGINE={}", engine.backend_name());
    println!("{ARCHITECTURE_LINE}");
    println!("AETHER_BROWSER_SERVO_API=0.5.0");
    println!("AETHER_BROWSER_LIVE_WINDOW=ENABLED");
    println!("AETHER_BROWSER_DRAGONGLASS_STYLE=ENABLED");
    println!("AETHER_BROWSER_APPROVED_RENDER=ENABLED");
    println!("AETHER_BROWSER_RENDER_LOCK=AETHER_BROWSER_COSMIC_INTERFACE");
    println!("AETHER_BROWSER_COSMIC_INTERFACE=AETHER_BROWSER_COSMIC_INTERFACE_V2");
    println!("AETHER_BROWSER_HOME_RENDER=COSMIC_OPEN_WORKSPACE");
    println!("AETHER_BROWSER_CHROME_CLEAR=OPAQUE_DARK");
    println!("AETHER_BROWSER_FRAME=FRAMELESS_AETHER_OWNED");
    println!("AETHER_BROWSER_INTEGRATED_LAUNCHER=ENABLED");
    println!("AETHER_BROWSER_UTILITY_DOCK=ENABLED");
    println!("AETHER_BROWSER_STATUS_BAR=ENABLED");
    println!("AETHER_BROWSER_SURFACE_LAYOUT=NATIVE_CHROME_WITH_WEB_VIEWPORT");
    println!("AETHER_BROWSER_LAUNCHER_WIDTH_PX=52");
    println!("AETHER_BROWSER_UTILITY_DOCK_WIDTH_PX=176");
    println!("AETHER_BROWSER_UTILITY_DOCK_COLLAPSED_WIDTH_PX=38");
    println!("AETHER_BROWSER_STATUS_BAR_HEIGHT_PX=30");
    println!("AETHER_BROWSER_UI_OWNERSHIP=NATIVE_RUST");
    println!("AETHER_BROWSER_UI_RENDERER=NATIVE_EGUI_GLOW");
    println!("AETHER_BROWSER_EXTERNAL_WEB_ENGINE=CHROMIUM_X11");
    println!("AETHER_BROWSER_SERVO_SURFACES=INTERNAL_NON_HTTP_ONLY");
    println!("AETHER_BROWSER_CHROME_WEBVIEW=NONE");
    println!("AETHER_BROWSER_PERSISTENT_SCROLLBAR=ON");
    println!("AETHER_BROWSER_NATIVE_HOME_WALLPAPER=EMBEDDED");
    println!("AETHER_BROWSER_DEFAULT_HOME=aether://home");
    println!("AETHER_BROWSER_HOME_SURFACE=NATIVE_RUST");
    println!("AETHER_BROWSER_HOME_BACKING_WEBVIEW=NONE");
    println!("AETHER_BROWSER_EXTERNAL_WEB_COMPOSITOR=CONTENT_VIEWPORT_ONLY");
    println!("AETHER_BROWSER_COMPOSITOR_ORDER=WEB_CONTENT_THEN_NATIVE_CHROME");
    println!("AETHER_BROWSER_NATIVE_FRAME_GATE=LIVE_CAPTURE_REQUIRED");
    println!("AETHER_BROWSER_CONTENT_TOP_PX=116");
    println!("AETHER_BROWSER_CONTENT_GEOMETRY=AUTHORITATIVE_SHARED_RECT");
    println!("AETHER_BROWSER_WINDOW_RESIZABLE=YES");
    println!("AETHER_BROWSER_RESIZE_BORDER_PX=7");
    println!("AETHER_BROWSER_MIN_WINDOW=760x520");
    println!("AETHER_BROWSER_TAB_ISLAND_VISUAL=ENABLED");
    println!("AETHER_BROWSER_OMNIBOX=ENABLED");
    println!("AETHER_BROWSER_TABS=ENABLED");
    println!("AETHER_BROWSER_WEB_INPUT=ENABLED");
    println!("AETHER_BROWSER_RESOURCE_POLICY=ENABLED");
    println!("AETHER_BROWSER_RESOURCE_BUDGET_MODEL=READY");
    println!("AETHER_BROWSER_SYSTEM_TELEMETRY=NATIVE_PROC_SYSFS");
    println!("AETHER_BROWSER_TELEMETRY_CPU=LIVE");
    println!("AETHER_BROWSER_TELEMETRY_RAM=LIVE");
    println!("AETHER_BROWSER_TELEMETRY_GPU=BEST_EFFORT_DRM_SYSFS");
    println!("AETHER_BROWSER_TELEMETRY_NETWORK=LIVE");
    println!("AETHER_BROWSER_TELEMETRY_BROWSER_FPS=LIVE");
    println!("AETHER_BROWSER_STREAM_TELEMETRY=FPS|RENDER_SKIPPED|OUTPUT_SKIPPED|NETWORK_DROPPED");
    println!("AETHER_BROWSER_BACKGROUND_THROTTLING=ENABLED");
    println!("AETHER_BROWSER_NATIVE_LAUNCHER=INTEGRATED");
    println!("AETHER_BROWSER_SETTINGS_ROUTE=aether://settings");
    println!("AETHER_BROWSER_LOGIN_ROUTE_DISPATCH=ENABLED");
    println!("AETHER_BROWSER_YOUTUBE_MUSIC_SURFACE=ACTIVE_CONTENT");
    println!("AETHER_BROWSER_YOUTUBE_MUSIC_ROUTE=aether://youtube-music");
    println!("AETHER_BROWSER_YOUTUBE_MUSIC_LAUNCHER=CONTENT|SEARCH|LIBRARY");
    println!("AETHER_BROWSER_UI_SNAPSHOT=CTRL+SHIFT+X|INTEGRATED_LAUNCHER");
    println!("AETHER_BROWSER_ANIMATED_SNAPSHOT=CTRL+SHIFT+A|INTEGRATED_LAUNCHER");
    println!("AETHER_BROWSER_ANIMATED_SNAPSHOT_OUTPUT=GIF|WEBP|MP4");
    println!("AETHER_BROWSER_ANIMATED_NATIVE_SURFACE=NATIVE_BROWSER_SURFACE");
    println!(
        "AETHER_BROWSER_ANIMATED_SNAPSHOT_FRAMES=$HOME/Downloads/Aether-Browser-v2.1.60-NATIVE-SURFACE-FRAMES"
    );

    println!(
        "AETHER_BROWSER_UI_SNAPSHOT_OUTPUT=$HOME/Downloads/Aether-Browser-v2.1.60-UI-SNAPSHOT.png"
    );
    println!("AETHER_BROWSER_WORKSPACE_STATE=READY");
    println!("AETHER_BROWSER_TAB_ISLAND_STATE=READY");
    println!("AETHER_BROWSER_SPLIT_VIEW_STATE=READY");
    println!("AETHER_BROWSER_GAMING_HUB_MODEL=READY");
    println!("AETHER_BROWSER_AETHER_IDENTITY=READY");
    println!("AETHER_BROWSER_LOGIN_TWITCH=REQUIRES_CLIENT_CONFIG");
    println!("AETHER_BROWSER_LOGIN_GOOGLE=REQUIRES_CLIENT_CONFIG");
    println!("AETHER_BROWSER_LOGIN_APPLE=REQUIRES_CLIENT_CONFIG");
    println!("AETHER_BROWSER_LOGIN_LOCAL=READY");
    println!("AETHER_BROWSER_IDENTITY_EMAIL_AUTOMERGE=DISABLED");
    println!("AETHER_BROWSER_VAULT=READY");
    println!("AETHER_BROWSER_VAULT_CRYPTO=ARGON2ID+XCHACHA20POLY1305");
    println!("AETHER_BROWSER_PROVIDER_REGISTRY=READY");
    println!(
        "AETHER_BROWSER_PROVIDER_COUNT={}",
        built_in_providers().len()
    );
    println!("AETHER_BROWSER_AETHERSTREAM_BROWSER_UI=AUTHORITATIVE");
    println!("AETHER_BROWSER_AETHERSTREAM_STANDALONE_GUI=RETIRED");
    println!("AETHER_BROWSER_AETHERSTREAM_SERVICE=RETAINED");
    println!("AETHER_BROWSER_NATIVE_STREAM_STUDIO=ENABLED");
    println!("AETHER_BROWSER_DECK_STUDIO=ENABLED");
    println!("AETHER_BROWSER_TWITCH_CREATOR=OAUTH+HELIX+EVENTSUB");
    println!("AETHER_BROWSER_STREAMLABS=OAUTH+API+SOCKET+ALERTS");
    println!("AETHER_BROWSER_STREAMELEMENTS=ASTRO+OVERLAYS+CHATBOT");
    println!("AETHER_BROWSER_CREATOR_CONTROL=UNIFIED_TYPED_BUS");
    println!("AETHER_BROWSER_STREAM_TELEMETRY_OWNER=AETHER_STREAM_STUDIO");
    println!("AETHER_BROWSER_EXTERNAL_OBS=COMPATIBILITY_OPTION");
    println!("AETHER_BROWSER_OBS_WEBSOCKET=ENABLED");
    println!("AETHER_BROWSER_OBS_WEBSOCKET_DEFAULT=ws://127.0.0.1:4455");
    println!("AETHER_BROWSER_BLUESKY=ATPROTO_OAUTH+APP_BSKY+CHAT_BSKY");
    println!("AETHER_BROWSER_COMMS_CORE=UNIFIED_INBOX+ACTIVITY");
    println!("AETHER_BROWSER_COMMS_GENERIC_MAIL=IMAP+SMTP");
    println!("AETHER_BROWSER_STREAMER_PRIVACY=HIDE_PRIVATE_WHILE_LIVE");
    println!("AETHER_BROWSER_ZOHO_MAIL=OAUTH_API+IMAP_SMTP");
    println!("AETHER_BROWSER_PROTON_MAIL=MANAGED_OFFICIAL_BRIDGE+IMAP_SMTP");
    println!("AETHER_BROWSER_PROTON_BRIDGE_PLAN=PAID_PLAN_REQUIRED");
    println!("AETHER_BROWSER_COMMS_HUB=UNIFIED");
    println!(
        "AETHER_BROWSER_COMMS_PROVIDERS=GENERIC_MAIL|ZOHO|PROTON|TWITCH|BLUESKY|STREAMLABS|STREAMELEMENTS"
    );
    println!("AETHER_BROWSER_COMMS_CAPTURE_PRIVACY=ENFORCED");
    println!("AETHER_BROWSER_CREATOR_TOKENS=AETHERFORGE_SECRET_REFS");
    println!("AETHER_BROWSER_NATIVE_VAULT_UI=ENABLED");
    println!("AETHER_BROWSER_NATIVE_ACCOUNTS_UI=ENABLED");
    println!("AETHER_BROWSER_NATIVE_PROVIDER_UI=ENABLED");
    println!("AETHER_BROWSER_NATIVE_MEDIA_UI=ENABLED");
    println!("AETHER_BROWSER_AETHER_LIBRARY=SQLITE_HISTORY|BOOKMARKS|DOWNLOADS|RECENTLY_CLOSED");
    println!("AETHER_BROWSER_LIBRARY_ROUTE=aether://library");
    println!("AETHER_BROWSER_LIBRARY_PRIVATE_PERSISTENCE=DISABLED");
    println!("AETHER_BROWSER_BROWSER_TAKEOVER=STABILITY_GATED");
    println!("AETHER_BROWSER_MEDIA_RUNTIME_PROBE=ENABLED");
    println!("AETHER_BROWSER_MEDIA_RUNTIME_SHORTCUT=CTRL+SHIFT+M");
    println!(
        "AETHER_BROWSER_AETHERSTREAM_SUPERVISOR_SOCKET={}",
        aether_stream_studio::supervisor_socket_path().display()
    );
    println!(
        "AETHER_BROWSER_AETHERSTREAM_SUPERVISOR_SOCKET_PRESENT={}",
        if aether_stream_studio::supervisor_socket_present() {
            "YES"
        } else {
            "NO"
        }
    );
    println!(
        "AETHER_BROWSER_DRAGONGLASS_TRANSPARENCY={}pct",
        theme.surface_transparency_percent
    );
    print_media_status();
}

fn main() {
    let mode = match parse_launch_args(std::env::args().skip(1)) {
        Ok(mode) => mode,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

    match mode {
        LaunchMode::Version => println!("{VERSION_LINE}"),
        LaunchMode::Status => print_status(),
        LaunchMode::MediaStatus => print_media_status(),
        LaunchMode::LibraryStatus => print_library_status(),
        LaunchMode::Browse(url) => {
            match ensure_media_service() {
                Ok(status) => println!(
                    "AETHER_BROWSER_MEDIA_SERVICE_RUNTIME=ONLINE:{}",
                    status.socket_path.display()
                ),
                Err(error) => eprintln!("AETHER_BROWSER_MEDIA_SERVICE_RUNTIME=DEGRADED:{error}"),
            }
            if let Err(error) = run_live_browser(&url) {
                eprintln!("Aether Browser failed to start: {error}");
                std::process::exit(1);
            }
        }
    }
}
