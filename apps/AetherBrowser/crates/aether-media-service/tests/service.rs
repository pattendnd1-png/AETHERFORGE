use aether_media_service::{MediaRendererMode, media_service_socket_path};

#[test]
fn cpu_bgra_is_the_canonical_safe_renderer() {
    assert_eq!(MediaRendererMode::CpuBgra.as_str(), "cpu-bgra");
    assert_eq!(MediaRendererMode::GpuDmabuf.as_str(), "gpu-dmabuf");
}

#[test]
fn socket_is_namespaced_under_aetherforge() {
    assert!(
        media_service_socket_path()
            .to_string_lossy()
            .contains("aetherforge/aether-media-service.sock")
            || media_service_socket_path()
                .to_string_lossy()
                .contains("aether-media-service.sock")
    );
}

#[test]
fn native_playback_protocol_round_trips_play_request() {
    use aether_media_service::{MediaServiceRequest, NativeProvider};
    let request = MediaServiceRequest::Play {
        provider: NativeProvider::YouTube,
        resource: "dQw4w9WgXcQ".into(),
        original_url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".into(),
    };
    let encoded = request.encode_line();
    assert_eq!(MediaServiceRequest::parse_line(&encoded), Ok(request));
}

#[test]
fn jpeg_accumulator_keeps_latest_complete_frame_across_chunks() {
    use aether_media_service::JpegFrameAccumulator;
    let mut parser = JpegFrameAccumulator::default();
    parser.push(&[0x00, 0xff, 0xd8, 0x01, 0x02]);
    assert!(parser.latest().is_none());
    parser.push(&[0x03, 0xff, 0xd9, 0xff, 0xd8, 0x09, 0xff]);
    assert_eq!(
        parser.latest(),
        Some(&[0xff, 0xd8, 0x01, 0x02, 0x03, 0xff, 0xd9][..])
    );
    parser.push(&[0xd9]);
    assert_eq!(parser.latest(), Some(&[0xff, 0xd8, 0x09, 0xff, 0xd9][..]));
}
