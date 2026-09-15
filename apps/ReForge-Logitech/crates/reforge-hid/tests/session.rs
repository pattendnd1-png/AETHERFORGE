use reforge_hid::{classify_packet, classify_response, HidppPacketKind};
use reforge_protocol::{HidppRequest, HidppResponse};

#[test]
fn matching_response_is_a_reply_and_unmatched_packet_is_notification() {
    let request = HidppRequest::new(0xff, 2, 1, vec![0, 0, 0]).unwrap();
    let matching = HidppResponse {
        report_id: 0x11,
        device_index: 0xff,
        feature_index: 2,
        function_swid: request.function_swid(),
        params: vec![1, 2, 3],
    };
    assert_eq!(classify_response(Some(&request), &matching), HidppPacketKind::Reply);

    let mut other = matching.clone();
    other.function_swid ^= 0x10;
    assert_eq!(classify_response(Some(&request), &other), HidppPacketKind::Notification);
}

#[test]
fn malformed_packet_is_unknown() {
    assert_eq!(classify_packet(None, &[0x01, 0x02]), HidppPacketKind::Unknown);
}
