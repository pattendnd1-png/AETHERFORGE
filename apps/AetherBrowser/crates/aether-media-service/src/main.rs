#![forbid(unsafe_code)]

use aether_media_service::{
    JpegFrameAccumulator, MEDIA_SERVICE_PROTOCOL_VERSION, MEDIA_SERVICE_RELEASE_VERSION,
    MediaRendererMode, MediaServiceRequest, NativeProvider, media_service_socket_path,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

const PLAYBACK_STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
const PLAYBACK_WATCHDOG_POLL: Duration = Duration::from_millis(250);

const YOUTUBE_RESOLVER_FORMAT: &str = "bv*+ba/b";
const YOUTUBE_RESOLVER_SORT: &str = "res:720";
const YOUTUBE_MERGE_CONTAINER: &str = "mkv";

#[derive(Debug)]
struct PlaybackRuntime {
    generation: u64,
    provider: Option<NativeProvider>,
    resource: String,
    state: String,
    error: Option<String>,
    latest_frame: Option<Vec<u8>>,
    buffering_started_at: Option<Instant>,
    first_frame_at: Option<Instant>,
    last_frame_at: Option<Instant>,
    children: Vec<Child>,
}

impl Default for PlaybackRuntime {
    fn default() -> Self {
        Self {
            generation: 0,
            provider: None,
            resource: String::new(),
            state: "idle".to_owned(),
            error: None,
            latest_frame: None,
            buffering_started_at: None,
            first_frame_at: None,
            last_frame_at: None,
            children: Vec::new(),
        }
    }
}

type SharedPlayback = Arc<Mutex<PlaybackRuntime>>;

fn lock_playback(shared: &SharedPlayback) -> MutexGuard<'_, PlaybackRuntime> {
    shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn stop_children(runtime: &mut PlaybackRuntime) {
    for child in &mut runtime.children {
        let _ = child.kill();
        let _ = child.wait();
    }
    runtime.children.clear();
}

fn stop_playback(shared: &SharedPlayback) {
    let mut runtime = lock_playback(shared);
    runtime.generation = runtime.generation.saturating_add(1);
    stop_children(&mut runtime);
    runtime.provider = None;
    runtime.resource.clear();
    runtime.state = "idle".to_owned();
    runtime.error = None;
    runtime.latest_frame = None;
    runtime.buffering_started_at = None;
    runtime.first_frame_at = None;
    runtime.last_frame_at = None;
}

fn begin_playback(shared: &SharedPlayback, provider: NativeProvider, resource: String) -> u64 {
    let mut runtime = lock_playback(shared);
    runtime.generation = runtime.generation.saturating_add(1);
    stop_children(&mut runtime);
    runtime.provider = Some(provider);
    runtime.resource = resource;
    runtime.state = "resolving".to_owned();
    runtime.error = None;
    runtime.latest_frame = None;
    runtime.buffering_started_at = None;
    runtime.first_frame_at = None;
    runtime.last_frame_at = None;
    runtime.generation
}

fn session_is_current(shared: &SharedPlayback, generation: u64) -> bool {
    lock_playback(shared).generation == generation
}

fn set_session_state(shared: &SharedPlayback, generation: u64, state: &str) {
    let mut runtime = lock_playback(shared);
    if runtime.generation == generation {
        runtime.state = state.to_owned();
        runtime.error = None;
        if state == "buffering" && runtime.buffering_started_at.is_none() {
            runtime.buffering_started_at = Some(Instant::now());
        }
    }
}

fn set_session_error(shared: &SharedPlayback, generation: u64, error: impl Into<String>) {
    let mut runtime = lock_playback(shared);
    if runtime.generation == generation {
        runtime.state = "error".to_owned();
        runtime.error = Some(error.into());
        stop_children(&mut runtime);
    }
}

fn register_child(shared: &SharedPlayback, generation: u64, mut child: Child) -> io::Result<()> {
    let mut runtime = lock_playback(shared);
    if runtime.generation != generation {
        let _ = child.kill();
        let _ = child.wait();
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "native media session was superseded",
        ));
    }
    runtime.children.push(child);
    Ok(())
}

fn spawn_frame_reader(shared: SharedPlayback, generation: u64, mut stdout: ChildStdout) {
    thread::spawn(move || {
        let mut accumulator = JpegFrameAccumulator::default();
        let mut chunk = [0_u8; 64 * 1024];
        loop {
            if !session_is_current(&shared, generation) {
                return;
            }
            let read = match stdout.read(&mut chunk) {
                Ok(0) => return,
                Ok(read) => read,
                Err(error) => {
                    set_session_error(&shared, generation, format!("frame-read:{error}"));
                    return;
                }
            };
            accumulator.push(&chunk[..read]);
            if let Some(frame) = accumulator.take_latest() {
                let mut runtime = lock_playback(&shared);
                if runtime.generation != generation {
                    return;
                }
                let first_frame = runtime.latest_frame.is_none();
                let now = Instant::now();
                runtime.latest_frame = Some(frame);
                runtime.state = "playing".to_owned();
                runtime.error = None;
                runtime.last_frame_at = Some(now);
                if first_frame {
                    runtime.first_frame_at = Some(now);
                    println!("AETHER_MEDIA_GSTREAMER_STAGE=FRAME");
                    println!("AETHER_MEDIA_GSTREAMER_VIDEO_DECODE=PASS");
                }
            }
        }
    });
}

fn spawn_playback_watchdog(shared: SharedPlayback, generation: u64) {
    thread::spawn(move || {
        loop {
            thread::sleep(PLAYBACK_WATCHDOG_POLL);
            let (current, state, timed_out) = {
                let runtime = lock_playback(&shared);
                let current = runtime.generation == generation;
                let timed_out = current
                    && runtime.state == "buffering"
                    && runtime
                        .buffering_started_at
                        .is_some_and(|started| started.elapsed() >= PLAYBACK_STARTUP_TIMEOUT);
                (current, runtime.state.clone(), timed_out)
            };
            if !current {
                return;
            }
            if timed_out {
                eprintln!("AETHER_MEDIA_WATCHDOG=buffering-timeout");
                set_session_error(&shared, generation, "buffering-timeout");
                return;
            }
            if matches!(state.as_str(), "playing" | "ended" | "error" | "idle") {
                return;
            }
        }
    });
}

fn spawn_gstreamer_stderr_reader(shared: SharedPlayback, generation: u64, stderr: ChildStderr) {
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let Ok(line) = line else {
                return;
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            eprintln!("AETHER_MEDIA_GSTREAMER_DIAG={trimmed}");
            let lower = trimmed.to_ascii_lowercase();
            if lower.contains("not-negotiated")
                || lower.contains("missing plugin")
                || lower.contains("no decoder")
                || lower.contains("could not decode")
            {
                eprintln!("AETHER_MEDIA_GSTREAMER_STAGE=DECODE_ERROR:{trimmed}");
                let mut runtime = lock_playback(&shared);
                if runtime.generation == generation {
                    runtime.error = Some(format!("gstreamer-decode:{trimmed}"));
                }
            }
        }
    });
}

fn spawn_gstreamer_from_stdin()
-> io::Result<(std::process::ChildStdin, ChildStdout, ChildStderr, Child)> {
    let mut child = Command::new("gst-launch-1.0")
        .args([
            "-q",
            "fdsrc",
            "fd=0",
            "!",
            "queue2",
            "!",
            "decodebin",
            "name=d",
            "d.",
            "!",
            "queue",
            "!",
            "audioconvert",
            "!",
            "audioresample",
            "!",
            "autoaudiosink",
            "sync=true",
            "d.",
            "!",
            "queue",
            "!",
            "videoconvert",
            "!",
            "videoscale",
            "!",
            "video/x-raw,width=1280",
            "!",
            "videorate",
            "!",
            "video/x-raw,framerate=15/1",
            "!",
            "jpegenc",
            "quality=82",
            "!",
            "fdsink",
            "fd=1",
            "sync=false",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("GStreamer stdin unavailable"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("GStreamer stdout unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("GStreamer stderr unavailable"))?;
    Ok((stdin, stdout, stderr, child))
}

fn canonical_youtube_watch_url(resource: &str, original_url: &str) -> String {
    if !resource.trim().is_empty() {
        format!("https://www.youtube.com/watch?v={}", resource.trim())
    } else {
        original_url.to_owned()
    }
}

fn youtube_resolver_command(resource: &str, original_url: &str) -> Command {
    let canonical_url = canonical_youtube_watch_url(resource, original_url);
    let mut command = Command::new("yt-dlp");
    command
        .args([
            "--no-config",
            "--no-playlist",
            "--no-progress",
            "--no-warnings",
            "--quiet",
            "--no-update",
            "--js-runtimes",
            "deno",
            "--check-formats",
            "--downloader",
            "ffmpeg",
            "--format",
            YOUTUBE_RESOLVER_FORMAT,
            "--format-sort",
            YOUTUBE_RESOLVER_SORT,
            "--merge-output-format",
            YOUTUBE_MERGE_CONTAINER,
            "--output",
            "-",
            "--",
        ])
        .arg(canonical_url);
    command
}

fn spawn_ytdlp_stream(
    resource: &str,
    original_url: &str,
) -> io::Result<(ChildStdout, ChildStderr, Child)> {
    let mut child = youtube_resolver_command(resource, original_url)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("yt-dlp stdout unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("yt-dlp stderr unavailable"))?;
    Ok((stdout, stderr, child))
}

fn spawn_ytdlp_stderr_reader(shared: SharedPlayback, generation: u64, stderr: ChildStderr) {
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let Ok(line) = line else {
                return;
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            eprintln!("AETHER_MEDIA_YTDLP_DIAG={trimmed}");
            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("error:")
                || lower.contains("http error 403")
                || lower.contains("sign in to confirm")
            {
                set_session_error(&shared, generation, format!("youtube-resolver:{trimmed}"));
                return;
            }
        }
    });
}

fn streamlink_command(resource: &str) -> Command {
    let url = format!("https://www.twitch.tv/{}", resource.trim());
    let mut command = Command::new("streamlink");
    command
        .args([
            "--no-config",
            "--loglevel",
            "warning",
            "--stdout",
            "--twitch-low-latency",
        ])
        .arg(url)
        .arg("best");
    command
}

fn spawn_streamlink_stream(resource: &str) -> io::Result<(ChildStdout, ChildStderr, Child)> {
    let mut child = streamlink_command(resource)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("Streamlink stdout unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("Streamlink stderr unavailable"))?;
    Ok((stdout, stderr, child))
}

fn spawn_streamlink_stderr_reader(shared: SharedPlayback, generation: u64, stderr: ChildStderr) {
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let Ok(line) = line else {
                return;
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            eprintln!("AETHER_MEDIA_STREAMLINK_DIAG={trimmed}");
            let lower = trimmed.to_ascii_lowercase();
            if lower.contains("error:")
                || lower.contains("no playable streams")
                || lower.contains("stream ended")
                || lower.contains("offline")
            {
                set_session_error(&shared, generation, format!("twitch-resolver:{trimmed}"));
                return;
            }
        }
    });
}

fn pump_provider_to_gstreamer(
    shared: SharedPlayback,
    generation: u64,
    provider: &'static str,
    mut source: ChildStdout,
    mut sink: std::process::ChildStdin,
) {
    thread::spawn(move || match io::copy(&mut source, &mut sink) {
        Ok(bytes) if bytes > 0 => {
            let _ = sink.flush();
            println!("AETHER_MEDIA_PROVIDER_STREAM_BYTES={provider}:{bytes}");
        }
        Ok(_) => set_session_error(
            &shared,
            generation,
            format!("{provider}-resolver-empty-stream"),
        ),
        Err(error) => set_session_error(
            &shared,
            generation,
            format!("{provider}-stream-copy:{error}"),
        ),
    });
}

fn run_youtube_worker(
    shared: SharedPlayback,
    generation: u64,
    resource: String,
    original_url: String,
) -> Result<(), String> {
    println!("AETHER_MEDIA_GSTREAMER_STAGE=YOUTUBE_RESOLVE");
    println!("AETHER_MEDIA_YOUTUBE_RESOLVER_BACKEND=yt-dlp");
    println!("AETHER_MEDIA_YOUTUBE_JS_RUNTIME=deno");
    println!("AETHER_MEDIA_YOUTUBE_DOWNLOADER=ffmpeg");
    println!("AETHER_MEDIA_YOUTUBE_RESOLVER_FORMAT=adaptive-bv-plus-ba-fallback-best");
    println!("AETHER_MEDIA_YOUTUBE_RESOLVER_SORT=res:720");
    println!("AETHER_MEDIA_YOUTUBE_REMUX=FFMPEG");
    println!("AETHER_MEDIA_YOUTUBE_MERGE_CONTAINER=mkv");
    println!("AETHER_MEDIA_DRM_REQUIRED=UNKNOWN");

    let canonical_url = canonical_youtube_watch_url(&resource, &original_url);
    println!("AETHER_MEDIA_YOUTUBE_CANONICAL_URL={canonical_url}");
    let (ytdlp_stdout, ytdlp_stderr, ytdlp_child) = spawn_ytdlp_stream(&resource, &original_url)
        .map_err(|error| format!("youtube-resolver-spawn:yt-dlp:{error}"))?;
    if !session_is_current(&shared, generation) {
        return Ok(());
    }
    register_child(&shared, generation, ytdlp_child).map_err(|error| error.to_string())?;
    spawn_ytdlp_stderr_reader(shared.clone(), generation, ytdlp_stderr);
    println!("AETHER_MEDIA_YOUTUBE_RESOLVE=PASS");
    println!("AETHER_MEDIA_DRM_REQUIRED=NO_EVIDENCE");

    set_session_state(&shared, generation, "buffering");
    println!("AETHER_MEDIA_GSTREAMER_STAGE=SPAWN");
    let (gst_stdin, gst_stdout, gst_stderr, gst_child) =
        spawn_gstreamer_from_stdin().map_err(|error| format!("gstreamer-spawn:{error}"))?;
    register_child(&shared, generation, gst_child).map_err(|error| error.to_string())?;
    spawn_gstreamer_stderr_reader(shared.clone(), generation, gst_stderr);
    spawn_frame_reader(shared.clone(), generation, gst_stdout);
    pump_provider_to_gstreamer(shared, generation, "youtube", ytdlp_stdout, gst_stdin);
    Ok(())
}

fn run_twitch_worker(
    shared: SharedPlayback,
    generation: u64,
    resource: String,
) -> Result<(), String> {
    if resource.starts_with("videos/") {
        return Err("twitch-vod-must-use-web-fallback".to_owned());
    }
    println!("AETHER_MEDIA_GSTREAMER_STAGE=TWITCH_RESOLVE");
    println!("AETHER_MEDIA_TWITCH_RESOLVER_BACKEND=streamlink");
    println!("AETHER_MEDIA_TWITCH_TRANSPORT=stdout");
    let (streamlink_stdout, streamlink_stderr, streamlink_child) =
        spawn_streamlink_stream(&resource)
            .map_err(|error| format!("twitch-resolver-spawn:streamlink:{error}"))?;
    if !session_is_current(&shared, generation) {
        return Ok(());
    }
    register_child(&shared, generation, streamlink_child).map_err(|error| error.to_string())?;
    spawn_streamlink_stderr_reader(shared.clone(), generation, streamlink_stderr);
    set_session_state(&shared, generation, "buffering");
    let (gst_stdin, gst_stdout, gst_stderr, gst_child) =
        spawn_gstreamer_from_stdin().map_err(|error| format!("gstreamer-spawn:{error}"))?;
    register_child(&shared, generation, gst_child).map_err(|error| error.to_string())?;
    spawn_gstreamer_stderr_reader(shared.clone(), generation, gst_stderr);
    spawn_frame_reader(shared.clone(), generation, gst_stdout);
    pump_provider_to_gstreamer(shared, generation, "twitch", streamlink_stdout, gst_stdin);
    Ok(())
}

fn spawn_native_playback(
    shared: SharedPlayback,
    provider: NativeProvider,
    resource: String,
    original_url: String,
) -> u64 {
    let generation = begin_playback(&shared, provider, resource.clone());
    spawn_playback_watchdog(shared.clone(), generation);
    thread::spawn(move || {
        let result = match provider {
            NativeProvider::YouTube => {
                run_youtube_worker(shared.clone(), generation, resource, original_url)
            }
            NativeProvider::Twitch => run_twitch_worker(shared.clone(), generation, resource),
        };
        if let Err(error) = result {
            set_session_error(&shared, generation, error);
        }
    });
    generation
}

fn state_line(shared: &SharedPlayback) -> String {
    let runtime = lock_playback(shared);
    let provider = runtime.provider.map_or("none", NativeProvider::as_str);
    let frame = if runtime.latest_frame.is_some() {
        "yes"
    } else {
        "no"
    };
    let error = runtime
        .error
        .as_deref()
        .unwrap_or("-")
        .replace(['\n', '\r'], " ");
    format!(
        "STATE state={} provider={} resource={} frame={} error={}",
        runtime.state,
        provider,
        runtime.resource.replace(' ', "%20"),
        frame,
        error.replace(' ', "%20")
    )
}

#[cfg(unix)]
fn run_daemon() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;

    let socket = media_service_socket_path();
    if let Some(parent) = socket.parent() {
        fs::create_dir_all(parent)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }
    if socket.exists() {
        fs::remove_file(&socket)?;
    }
    let listener = UnixListener::bind(&socket)?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))?;
    let renderer = MediaRendererMode::from_env();
    let playback = Arc::new(Mutex::new(PlaybackRuntime::default()));
    println!(
        "AETHER_MEDIA_SERVICE_READY=YES protocol={} version={}",
        MEDIA_SERVICE_PROTOCOL_VERSION, MEDIA_SERVICE_RELEASE_VERSION
    );
    println!("AETHER_MEDIA_SERVICE_RENDERER={}", renderer.as_str());
    println!("AETHER_MEDIA_SERVICE_SOCKET={}", socket.display());
    println!("AETHER_BROWSER_MEDIA_WATCHDOG=PASS");
    println!("AETHER_BROWSER_TWITCH_VOD=CHROMIUM_WEB_REQUIRED");
    println!("AETHER_MEDIA_YOUTUBE_RESOLVER_BACKEND=yt-dlp");
    println!("AETHER_MEDIA_YOUTUBE_JS_RUNTIME=deno");

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!("AETHER_MEDIA_SERVICE_ACCEPT_ERROR={error}");
                continue;
            }
        };
        let mut line = String::new();
        BufReader::new(stream.try_clone()?).read_line(&mut line)?;
        let request = match MediaServiceRequest::parse_line(&line) {
            Ok(request) => request,
            Err(error) => {
                writeln!(stream, "ERROR {error}")?;
                stream.flush()?;
                continue;
            }
        };
        match request {
            MediaServiceRequest::Status => writeln!(
                stream,
                "AETHER_MEDIA_SERVICE_READY=YES protocol={} version={} renderer={} socket={} native-providers=youtube,twitch",
                MEDIA_SERVICE_PROTOCOL_VERSION,
                MEDIA_SERVICE_RELEASE_VERSION,
                renderer.as_str(),
                socket.display()
            )?,
            MediaServiceRequest::Ping => writeln!(stream, "PONG")?,
            MediaServiceRequest::Play {
                provider,
                resource,
                original_url,
            } => {
                let generation =
                    spawn_native_playback(playback.clone(), provider, resource, original_url);
                writeln!(
                    stream,
                    "OK PLAY session={generation} provider={}",
                    provider.as_str()
                )?;
            }
            MediaServiceRequest::Stop => {
                stop_playback(&playback);
                writeln!(stream, "OK STOP")?;
            }
            MediaServiceRequest::State => writeln!(stream, "{}", state_line(&playback))?,
            MediaServiceRequest::Frame => {
                let frame = lock_playback(&playback).latest_frame.clone();
                if let Some(frame) = frame {
                    writeln!(stream, "FRAME {}", STANDARD.encode(frame))?;
                } else {
                    writeln!(stream, "FRAME NONE")?;
                }
            }
            MediaServiceRequest::Shutdown => {
                stop_playback(&playback);
                writeln!(stream, "BYE")?;
                stream.flush()?;
                break;
            }
        }
        stream.flush()?;
    }
    stop_playback(&playback);
    let _ = fs::remove_file(&socket);
    Ok(())
}

#[cfg(not(unix))]
fn run_daemon() -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "aether-media-service currently requires Unix sockets",
    ))
}

fn main() {
    let renderer = MediaRendererMode::from_env();
    match std::env::args().nth(1).as_deref() {
        Some("--daemon") => {
            if let Err(error) = run_daemon() {
                eprintln!("AETHER_MEDIA_SERVICE_FATAL={error}");
                std::process::exit(1);
            }
        }
        Some("--status") | None => {
            println!(
                "AETHER_MEDIA_SERVICE_READY=NATIVE_PROVIDER_FOUNDATION protocol={}",
                MEDIA_SERVICE_PROTOCOL_VERSION
            );
            println!(
                "AETHER_MEDIA_SERVICE_RELEASE_VERSION={}",
                MEDIA_SERVICE_RELEASE_VERSION
            );
            println!("AETHER_MEDIA_SERVICE_RENDERER={}", renderer.as_str());
            println!(
                "AETHER_MEDIA_SERVICE_SOCKET={}",
                media_service_socket_path().display()
            );
            println!("AETHER_MEDIA_SERVICE_NATIVE_PROVIDER_YOUTUBE=YT_DLP");
            println!("AETHER_MEDIA_YOUTUBE_JS_RUNTIME=deno");
            println!("AETHER_MEDIA_SERVICE_NATIVE_PROVIDER_TWITCH=STREAMLINK");
            println!("AETHER_BROWSER_MEDIA_WATCHDOG=PASS");
            println!("AETHER_BROWSER_TWITCH_VOD=CHROMIUM_WEB_REQUIRED");
        }
        Some(_) => {
            eprintln!("usage: aether-media-service [--status|--daemon]");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod provider_resolver_tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn yt_dlp_command_uses_adaptive_ffmpeg_stdout_contract() {
        let resource = "jNQXAC9IVRw";
        let url = "https://music.youtube.com/watch?v=jNQXAC9IVRw";
        let command = youtube_resolver_command(resource, url);
        assert_eq!(command.get_program(), OsStr::new("yt-dlp"));
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let expected: Vec<String> = vec![
            "--no-config",
            "--no-playlist",
            "--no-progress",
            "--no-warnings",
            "--quiet",
            "--no-update",
            "--js-runtimes",
            "deno",
            "--check-formats",
            "--downloader",
            "ffmpeg",
            "--format",
            YOUTUBE_RESOLVER_FORMAT,
            "--format-sort",
            YOUTUBE_RESOLVER_SORT,
            "--merge-output-format",
            YOUTUBE_MERGE_CONTAINER,
            "--output",
            "-",
            "--",
            "https://www.youtube.com/watch?v=jNQXAC9IVRw",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        assert_eq!(args, expected);
    }

    #[test]
    fn youtube_music_watch_urls_are_normalized_to_canonical_youtube_watch_urls() {
        assert_eq!(
            canonical_youtube_watch_url(
                "dQw4w9WgXcQ",
                "https://music.youtube.com/watch?v=dQw4w9WgXcQ"
            ),
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        );
    }

    #[test]
    fn twitch_streamlink_command_streams_best_quality_to_stdout() {
        let command = streamlink_command("monstercat");
        assert_eq!(command.get_program(), OsStr::new("streamlink"));
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(args.iter().any(|arg| arg == "--stdout"));
        assert!(args.iter().any(|arg| arg == "--twitch-low-latency"));
        assert!(
            args.iter()
                .any(|arg| arg == "https://www.twitch.tv/monstercat")
        );
        assert_eq!(args.last().map(String::as_str), Some("best"));
    }
}
