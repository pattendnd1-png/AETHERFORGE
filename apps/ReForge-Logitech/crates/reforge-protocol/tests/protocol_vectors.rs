use reforge_protocol::{
    HidppRequest, HidppResponse, SOFTWARE_ID,
    dpi::{build_set_dpi_request, parse_dpi_range, parse_dpi_state, parse_dpi_values, validate_dpi, validate_dpi_value},
    feature::{parse_feature_index, parse_feature_set_entry},
};

#[test]
fn root_feature_query_encodes_as_short_report() {
    let request = HidppRequest::new(0xff, 0x00, 0x0, vec![0x22, 0x01, 0x00]).unwrap();
    assert_eq!(SOFTWARE_ID, 0x08);
    assert_eq!(request.encode(), vec![0x10, 0xff, 0x00, 0x08, 0x22, 0x01, 0x00]);
}

#[test]
fn payload_over_three_bytes_uses_long_report_and_zero_padding() {
    let request = HidppRequest::new(0x01, 0x0a, 0x3, vec![0, 0x06, 0x40, 0xaa]).unwrap();
    let bytes = request.encode();
    assert_eq!(bytes.len(), 20);
    assert_eq!(&bytes[..8], &[0x11, 0x01, 0x0a, 0x38, 0, 0x06, 0x40, 0xaa]);
    assert!(bytes[8..].iter().all(|byte| *byte == 0));
}

#[test]
fn captured_root_response_resolves_adjustable_dpi_index() {
    let raw = [
        0x11, 0xff, 0x00, 0x08, 0x0a, 0x00, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let response = HidppResponse::parse(&raw).unwrap();
    assert_eq!(parse_feature_index(&response).unwrap(), Some(0x0a));
}

#[test]
fn feature_set_entry_parses_feature_id_and_metadata() {
    let raw = [
        0x11, 0xff, 0x01, 0x18, 0x22, 0x01, 0x03, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let response = HidppResponse::parse(&raw).unwrap();
    let entry = parse_feature_set_entry(7, &response).unwrap();
    assert_eq!(entry.index, 7);
    assert_eq!(entry.feature_id, 0x2201);
    assert_eq!(entry.metadata, 0x03);
}

#[test]
fn captured_adjustable_dpi_range_is_parsed() {
    let raw = [
        0x11, 0xff, 0x0a, 0x18, 0x00, 0x00, 0x32, 0xe0, 0x32, 0x2e, 0xe0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let response = HidppResponse::parse(&raw).unwrap();
    let range = parse_dpi_range(&response).unwrap();
    assert_eq!(range.sensor, 0);
    assert_eq!(range.min, 50);
    assert_eq!(range.max, 12_000);
    assert_eq!(range.step, 50);
}

#[test]
fn captured_current_dpi_is_parsed() {
    let raw = [
        0x11, 0xff, 0x0a, 0x28, 0x00, 0x06, 0x40, 0x03, 0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let response = HidppResponse::parse(&raw).unwrap();
    let state = parse_dpi_state(&response).unwrap();
    assert_eq!(state.sensor, 0);
    assert_eq!(state.current, 1600);
    assert_eq!(state.default, 800);
}

#[test]
fn dpi_validation_rejects_out_of_range_and_misaligned_values() {
    let range = reforge_protocol::dpi::DpiRange { sensor: 0, min: 50, max: 12_000, step: 50 };
    assert!(validate_dpi(&range, 1600).is_ok());
    assert!(validate_dpi(&range, 49).is_err());
    assert!(validate_dpi(&range, 12_050).is_err());
    assert!(validate_dpi(&range, 1625).is_err());
}

#[test]
fn set_dpi_frame_matches_hidpp_function_three_layout() {
    let request = build_set_dpi_request(0xff, 0x0a, 0, 1600).unwrap();
    assert_eq!(request.encode(), vec![0x10, 0xff, 0x0a, 0x38, 0x00, 0x06, 0x40]);
}


#[test]
fn dpi_list_preserves_discrete_values() {
    let raw = [
        0x11, 0xff, 0x0a, 0x18, 0x00, 0x01, 0x90, 0x03, 0x20, 0x06, 0x40, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let response = HidppResponse::parse(&raw).unwrap();
    let values = parse_dpi_values(&response).unwrap();
    assert_eq!(values, vec![400, 800, 1600]);
    assert!(validate_dpi_value(&values, 800).is_ok());
    assert!(validate_dpi_value(&values, 1200).is_err());
}

#[test]
fn dpi_list_expands_step_marker_and_keeps_range_end() {
    let raw = [
        0x11, 0xff, 0x0a, 0x18, 0x00, 0x00, 0x32, 0xe0, 0x32, 0x00, 0xc8, 0x03, 0x20, 0, 0, 0, 0, 0, 0, 0,
    ];
    let response = HidppResponse::parse(&raw).unwrap();
    assert_eq!(parse_dpi_values(&response).unwrap(), vec![50, 100, 150, 200, 800]);
}
