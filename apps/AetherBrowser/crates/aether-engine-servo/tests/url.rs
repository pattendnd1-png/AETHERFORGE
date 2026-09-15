use aether_engine_servo::normalize_startup_url;

#[test]
fn preserves_absolute_https_url() {
    assert_eq!(
        normalize_startup_url("https://servo.org/").expect("url"),
        "https://servo.org/"
    );
}

#[test]
fn upgrades_bare_host_to_https() {
    assert_eq!(
        normalize_startup_url("example.com/docs").expect("url"),
        "https://example.com/docs"
    );
}

#[test]
fn rejects_empty_startup_url() {
    assert!(normalize_startup_url("   ").is_err());
}
