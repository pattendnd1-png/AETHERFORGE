#![forbid(unsafe_code)]
//! Crash-isolated media-service boundary for Aether Browser.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub const MEDIA_SERVICE_PROTOCOL_VERSION: u32 = 2;
pub const MEDIA_SERVICE_RELEASE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaRendererMode {
    CpuBgra,
    GpuDmabuf,
}

impl MediaRendererMode {
    #[must_use]
    pub fn from_env() -> Self {
        match env::var("AETHER_BROWSER_MEDIA_RENDERER")
            .unwrap_or_else(|_| "cpu-bgra".to_owned())
            .to_ascii_lowercase()
            .as_str()
        {
            "gpu-dmabuf" | "gpu" => Self::GpuDmabuf,
            _ => Self::CpuBgra,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CpuBgra => "cpu-bgra",
            Self::GpuDmabuf => "gpu-dmabuf",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeProvider {
    YouTube,
    Twitch,
}

impl NativeProvider {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::YouTube => "youtube",
            Self::Twitch => "twitch",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "youtube" => Ok(Self::YouTube),
            "twitch" => Ok(Self::Twitch),
            other => Err(format!("unsupported native provider: {other}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MediaServiceRequest {
    Play {
        provider: NativeProvider,
        resource: String,
        original_url: String,
    },
    Stop,
    State,
    Frame,
    Status,
    Ping,
    Shutdown,
}

impl MediaServiceRequest {
    #[must_use]
    pub fn encode_line(&self) -> String {
        match self {
            Self::Play {
                provider,
                resource,
                original_url,
            } => {
                let query = url::form_urlencoded::Serializer::new(String::new())
                    .append_pair("provider", provider.as_str())
                    .append_pair("resource", resource)
                    .append_pair("url", original_url)
                    .finish();
                format!("PLAY?{query}")
            }
            Self::Stop => "STOP".to_owned(),
            Self::State => "STATE".to_owned(),
            Self::Frame => "FRAME".to_owned(),
            Self::Status => "STATUS".to_owned(),
            Self::Ping => "PING".to_owned(),
            Self::Shutdown => "SHUTDOWN".to_owned(),
        }
    }

    pub fn parse_line(line: &str) -> Result<Self, String> {
        let line = line.trim();
        if let Some(query) = line.strip_prefix("PLAY?") {
            let mut provider = None;
            let mut resource = None;
            let mut original_url = None;
            for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
                match key.as_ref() {
                    "provider" => provider = Some(NativeProvider::parse(&value)?),
                    "resource" => resource = Some(value.into_owned()),
                    "url" => original_url = Some(value.into_owned()),
                    _ => {}
                }
            }
            return Ok(Self::Play {
                provider: provider.ok_or_else(|| "PLAY missing provider".to_owned())?,
                resource: resource.ok_or_else(|| "PLAY missing resource".to_owned())?,
                original_url: original_url.ok_or_else(|| "PLAY missing url".to_owned())?,
            });
        }
        match line {
            "STOP" => Ok(Self::Stop),
            "STATE" => Ok(Self::State),
            "FRAME" => Ok(Self::Frame),
            "STATUS" => Ok(Self::Status),
            "PING" => Ok(Self::Ping),
            "SHUTDOWN" => Ok(Self::Shutdown),
            _ => Err(format!("unknown media-service request: {line}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaServiceStatus {
    pub online: bool,
    pub renderer: MediaRendererMode,
    pub socket_path: PathBuf,
}

#[derive(Debug, Default)]
pub struct JpegFrameAccumulator {
    buffer: Vec<u8>,
    latest: Option<Vec<u8>>,
}

impl JpegFrameAccumulator {
    pub fn push(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);
        loop {
            let Some(start) = find_pair(&self.buffer, 0xff, 0xd8, 0) else {
                if self.buffer.len() > 1 {
                    let keep_ff = self.buffer.last().copied() == Some(0xff);
                    self.buffer.clear();
                    if keep_ff {
                        self.buffer.push(0xff);
                    }
                }
                break;
            };
            if start > 0 {
                self.buffer.drain(..start);
            }
            let Some(end) = find_pair(&self.buffer, 0xff, 0xd9, 2) else {
                break;
            };
            let frame_end = end + 2;
            self.latest = Some(self.buffer[..frame_end].to_vec());
            self.buffer.drain(..frame_end);
        }
        const MAX_PENDING_BYTES: usize = 16 * 1024 * 1024;
        if self.buffer.len() > MAX_PENDING_BYTES {
            self.buffer.clear();
        }
    }

    #[must_use]
    pub fn latest(&self) -> Option<&[u8]> {
        self.latest.as_deref()
    }

    pub fn take_latest(&mut self) -> Option<Vec<u8>> {
        self.latest.take()
    }
}

fn find_pair(bytes: &[u8], first: u8, second: u8, from: usize) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(2)
        .position(|window| window == [first, second])
        .map(|position| position + from)
}

#[must_use]
pub fn media_service_socket_path() -> PathBuf {
    if let Some(dir) = env::var_os("XDG_RUNTIME_DIR") {
        PathBuf::from(dir).join("aetherforge/aether-media-service.sock")
    } else {
        PathBuf::from(format!(
            "/tmp/aether-media-service-{}/aether-media-service.sock",
            env::var("UID").unwrap_or_else(|_| "user".to_owned())
        ))
    }
}

#[cfg(unix)]
fn request_service(request: &MediaServiceRequest) -> io::Result<String> {
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(media_service_socket_path())?;
    writeln!(stream, "{}", request.encode_line())?;
    stream.flush()?;
    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response)?;
    Ok(response.trim_end().to_owned())
}

#[cfg(not(unix))]
fn request_service(_request: &MediaServiceRequest) -> io::Result<String> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "aether-media-service currently requires Unix sockets",
    ))
}

pub fn start_native_playback(
    provider: NativeProvider,
    resource: &str,
    original_url: &str,
) -> io::Result<String> {
    request_service(&MediaServiceRequest::Play {
        provider,
        resource: resource.to_owned(),
        original_url: original_url.to_owned(),
    })
}

pub fn stop_native_playback() -> io::Result<String> {
    request_service(&MediaServiceRequest::Stop)
}

pub fn native_playback_state() -> io::Result<String> {
    request_service(&MediaServiceRequest::State)
}

pub fn latest_native_frame() -> io::Result<Option<Vec<u8>>> {
    let response = request_service(&MediaServiceRequest::Frame)?;
    if response == "FRAME NONE" {
        return Ok(None);
    }
    let encoded = response
        .strip_prefix("FRAME ")
        .ok_or_else(|| io::Error::other(format!("invalid frame response: {response}")))?;
    STANDARD
        .decode(encoded)
        .map(Some)
        .map_err(|error| io::Error::other(format!("invalid frame base64: {error}")))
}

#[cfg(unix)]
#[must_use]
pub fn probe_media_service() -> MediaServiceStatus {
    let renderer = MediaRendererMode::from_env();
    let socket_path = media_service_socket_path();
    let online = request_service(&MediaServiceRequest::Status).is_ok_and(|response| {
        response.starts_with("AETHER_MEDIA_SERVICE_READY")
            && response.contains("protocol=2")
            && response.contains(&format!("version={MEDIA_SERVICE_RELEASE_VERSION}"))
    });
    MediaServiceStatus {
        online,
        renderer,
        socket_path,
    }
}

#[cfg(not(unix))]
#[must_use]
pub fn probe_media_service() -> MediaServiceStatus {
    MediaServiceStatus {
        online: false,
        renderer: MediaRendererMode::from_env(),
        socket_path: media_service_socket_path(),
    }
}

#[must_use]
pub fn media_service_binary_path() -> Option<PathBuf> {
    let current = env::current_exe().ok()?;
    let parent = current.parent()?;
    Some(parent.join("aether-media-service"))
}

fn wait_for_service_exit() {
    for _ in 0..20 {
        if !media_service_socket_path().exists() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

pub fn ensure_media_service() -> io::Result<MediaServiceStatus> {
    let current = probe_media_service();
    if current.online {
        return Ok(current);
    }

    // Clear an unhealthy endpoint before starting the current service.
    if media_service_socket_path().exists() {
        let _ = request_service(&MediaServiceRequest::Shutdown);
        wait_for_service_exit();
        let _ = std::fs::remove_file(media_service_socket_path());
    }

    let binary = media_service_binary_path()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "media-service binary path"))?;
    if !binary.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("media-service binary not found: {}", binary.display()),
        ));
    }
    Command::new(binary)
        .arg("--daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    for _ in 0..40 {
        thread::sleep(Duration::from_millis(50));
        let status = probe_media_service();
        if status.online {
            return Ok(status);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "aether-media-service protocol-v2 did not become ready",
    ))
}
