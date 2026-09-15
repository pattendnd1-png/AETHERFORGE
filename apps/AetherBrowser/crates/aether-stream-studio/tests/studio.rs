use aether_stream_studio::{StandaloneGuiPolicy, StreamStudioState, StudioAuthority};

#[test]
fn browser_is_authoritative_and_standalone_gui_is_retired() {
    let studio = StreamStudioState::canonical();
    assert_eq!(studio.authority, StudioAuthority::BrowserAuthoritative);
    assert_eq!(studio.standalone_gui, StandaloneGuiPolicy::Retired);
}

#[test]
fn disconnecting_ui_does_not_clear_backend_live_state() {
    let mut studio = StreamStudioState::canonical();
    studio.service.connected = true;
    studio.service.live = true;
    studio.service.recording = true;
    studio.disconnect_ui();
    assert!(!studio.service.connected);
    assert!(studio.service.live);
    assert!(studio.service.recording);
}

#[test]
fn supervisor_socket_contract_matches_aetherstream_runtime() {
    let path = aether_stream_studio::supervisor_socket_path();
    assert!(path.ends_with("aetherstream/supervisor.sock"));
}
