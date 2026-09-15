use aether_browser::{LaunchMode, parse_launch_args};

#[test]
fn current_status_mode_is_parsed() {
    assert_eq!(parse_launch_args(["--status"]), Ok(LaunchMode::Status));
}

#[test]
fn media_status_mode_is_parsed() {
    assert_eq!(
        parse_launch_args(["--media-status"]),
        Ok(LaunchMode::MediaStatus)
    );
}

#[test]
fn explicit_url_mode_is_parsed() {
    assert_eq!(
        parse_launch_args(["--url", "example.com"]),
        Ok(LaunchMode::Browse("example.com".into()))
    );
}

#[test]
fn positional_url_launches_browser() {
    assert_eq!(
        parse_launch_args(["https://servo.org/"]),
        Ok(LaunchMode::Browse("https://servo.org/".into()))
    );
}

#[test]
fn default_launch_uses_aether_start_page() {
    assert_eq!(
        parse_launch_args(std::iter::empty::<&str>()),
        Ok(LaunchMode::Browse("aether://home".into()))
    );
}

#[test]
fn library_modes_are_current() {
    assert_eq!(
        parse_launch_args(["--library-status"]).unwrap(),
        LaunchMode::LibraryStatus
    );
    assert_eq!(
        parse_launch_args(["--library"]).unwrap(),
        LaunchMode::Browse("aether://library".into())
    );
}
