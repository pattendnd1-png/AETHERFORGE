const LIVE_SOURCE: &str = include_str!("../src/live.rs");
const INSTALL_SOURCE: &str = include_str!("../../../scripts/install-current-tree.sh");

#[test]
fn native_runtime_is_released_in_winit_exiting_callback() {
    assert!(LIVE_SOURCE.contains("fn exiting(&mut self, _event_loop: &ActiveEventLoop)"));
    assert!(LIVE_SOURCE.contains("std::mem::replace("));
    assert!(LIVE_SOURCE.contains("Self::Stopped {"));
    assert!(LIVE_SOURCE.contains("AETHER_BROWSER_RUNTIME_SHUTDOWN=EARLY_DROP"));
}

#[test]
fn webviews_release_before_gl_painter_destroy() {
    let webview = LIVE_SOURCE
        .find("tab.webview.take();")
        .expect("webview teardown");
    let context = LIVE_SOURCE
        .find("tab.rendering_context.take();")
        .expect("offscreen-context teardown");
    let painter = LIVE_SOURCE
        .find("self.egui_painter.destroy();")
        .expect("egui painter teardown");
    assert!(webview < context && context < painter);
}

#[test]
fn installed_live_frame_probe_keeps_nonzero_exit_fatal() {
    let start = INSTALL_SOURCE
        .find("installed_web_rc=$?")
        .expect("installed web probe result capture");
    let end = INSTALL_SOURCE[start..]
        .find("rm -f /tmp/aether-browser-installed-web-probe.log")
        .map(|offset| start + offset)
        .expect("installed web probe cleanup");
    let probe_block = &INSTALL_SOURCE[start..end];

    assert!(probe_block.contains("if (( installed_web_rc == 0 ))"));
    assert!(probe_block.contains("AETHER_BROWSER_POSTINSTALL_VISUAL=FAIL"));
    assert!(probe_block.contains("hard_fail=1"));
    assert!(!probe_block.contains("PASS_WITH_139"));
    assert!(!probe_block.contains("installed_web_rc == 139"));
    assert!(!probe_block.contains("installed_web_rc==139"));
}
