#![forbid(unsafe_code)]

pub const NATIVE_MEDIA_PROVIDER_YOUTUBE_STATUS: &str =
    "AETHER_BROWSER_NATIVE_MEDIA_PROVIDER_YOUTUBE=ENABLED";
pub const NATIVE_MEDIA_PROVIDER_TWITCH_STATUS: &str =
    "AETHER_BROWSER_NATIVE_MEDIA_PROVIDER_TWITCH=ENABLED";
pub const NATIVE_DARK_MODE_STATUS: &str = "AETHER_BROWSER_NATIVE_DARK_MODE=FORCED";
pub const WEB_THEME_STATUS: &str = "AETHER_BROWSER_WEB_THEME=FORCED_DARK";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LaunchMode {
    Version,
    Status,
    MediaStatus,
    LibraryStatus,
    Browse(String),
}

pub fn parse_launch_args<I, S>(args: I) -> Result<LaunchMode, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    match args.as_slice() {
        [] => Ok(LaunchMode::Browse("aether://home".into())),
        [flag] if flag == "--version" => Ok(LaunchMode::Version),
        [flag] if flag == "--status" => Ok(LaunchMode::Status),
        [flag] if flag == "--media-status" => Ok(LaunchMode::MediaStatus),
        [flag] if flag == "--library-status" => Ok(LaunchMode::LibraryStatus),
        [flag] if flag == "--library" => Ok(LaunchMode::Browse("aether://library".into())),
        [flag] if flag == "--stream-studio" => Ok(LaunchMode::Browse("aether://stream".into())),
        [flag] if flag == "--vault" => Ok(LaunchMode::Browse("aether://vault".into())),
        [flag] if flag == "--accounts" => Ok(LaunchMode::Browse("aether://accounts".into())),
        [flag] if flag == "--providers" => Ok(LaunchMode::Browse("aether://providers".into())),
        [flag] if flag == "--media-center" => Ok(LaunchMode::Browse("aether://media".into())),
        [flag, url] if flag == "--url" => Ok(LaunchMode::Browse(url.clone())),
        [url] if !url.starts_with('-') => Ok(LaunchMode::Browse(url.clone())),
        _ => Err(
            "usage: aether-browser [--version|--status|--media-status|--library-status|--library|--stream-studio|--vault|--accounts|--providers|--media-center|--url <URL>|<URL>]"
                .into(),
        ),
    }
}
