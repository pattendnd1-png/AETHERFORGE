use reforge_hid::{probe_indices, response_matches};
use reforge_protocol::{HidppRequest, HidppResponse};

#[test]
fn direct_index_is_probed_before_receiver_slots() {
    assert_eq!(probe_indices(), [0xff, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn response_match_requires_device_feature_and_function() {
    let request = HidppRequest::new(0xff, 0x0a, 0x2, vec![0, 0, 0]).unwrap();
    let good = HidppResponse::parse(&[0x10, 0xff, 0x0a, 0x28, 0, 6, 0x40]).unwrap();
    let event = HidppResponse::parse(&[0x10, 0xff, 0x0b, 0x28, 0, 0, 0]).unwrap();
    assert!(response_matches(&request, &good));
    assert!(!response_matches(&request, &event));
}

#[test]
fn hidpp_error_response_is_matched_to_original_request() {
    let request = HidppRequest::new(0xff, 0x00, 0x1, vec![0, 0, 0]).unwrap();
    let error = HidppResponse::parse(&[0x10, 0xff, 0x8f, 0x00, 0x18, 0x01, 0]).unwrap();
    assert!(response_matches(&request, &error));
}

#[test]
fn feature_set_enumeration_uses_one_based_feature_indices() {
    assert_eq!(reforge_hid::feature_indices(3), vec![1, 2, 3]);
}

use reforge_core::{DeviceClass, DeviceSummary, FeatureInfo, LightingCapabilities, ProviderKind, TransportKind};

fn mirror(index: u8, feature_id: u16) -> DeviceSummary {
    DeviceSummary {
        key: format!("mirror-{index}"),
        path: "/dev/hidraw0".into(),
        device_index: index,
        vendor_id: 0x046d,
        product_id: 0xc343,
        product: if index == 0xff { "G515 LS TKL".into() } else { format!("G515 LS TKL [receiver slot {index}]") },
        serial: Some("ABC".into()),
        hardware_id: Some("UNITABC".into()),
        interface_number: 2,
        usage_page: 0xff00,
        usage: 1,
        transport: TransportKind::Usb,
        hidpp: true,
        device_class: DeviceClass::Keyboard,
        providers: vec![ProviderKind::Hidpp],
        controls: vec![],
        features: vec![FeatureInfo { index: 1, feature_id, name: "feature".into(), metadata: 0, version: 0 }],
        dpi: None,
        lighting: Some(LightingCapabilities::default()),
        error: None,
    }
}

#[test]
fn receiver_slot_mirrors_collapse_to_one_physical_device() {
    let devices = vec![mirror(0xff, 0x8081), mirror(1, 0x8081), mirror(2, 0x8081)];
    let collapsed = reforge_hid::collapse_receiver_mirrors(devices);
    assert_eq!(collapsed.len(), 1);
    assert_eq!(collapsed[0].device_index, 0xff);
}

#[test]
fn distinct_receiver_capabilities_are_not_collapsed() {
    let devices = vec![mirror(0xff, 0x8081), mirror(1, 0x2201)];
    let collapsed = reforge_hid::collapse_receiver_mirrors(devices);
    assert_eq!(collapsed.len(), 2);
}

#[test]
fn same_model_receiver_children_with_distinct_unit_ids_are_kept() {
    let direct = mirror(0xff, 0x8081);
    let mut child = mirror(1, 0x8081);
    child.hardware_id = Some("DIFFERENTUNIT".into());
    let collapsed = reforge_hid::collapse_receiver_mirrors(vec![direct, child]);
    assert_eq!(collapsed.len(), 2);
}
