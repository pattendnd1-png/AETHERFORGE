use reforge_core::{DeviceClass, DeviceSummary, Profile, ProviderKind, TransportKind};
use std::collections::BTreeMap;

fn device() -> DeviceSummary {
    DeviceSummary {
        key: "046d:c539:ff:/dev/hidraw4".into(),
        path: "/dev/hidraw4".into(),
        device_index: 0xff,
        vendor_id: 0x046d,
        product_id: 0xc539,
        product: "Example Logitech Device".into(),
        serial: Some("ABC123".into()),
        hardware_id: Some("UNITABC123".into()),
        interface_number: 2,
        usage_page: 0xff00,
        usage: 1,
        transport: TransportKind::Usb,
        hidpp: true,
        device_class: DeviceClass::Keyboard,
        providers: vec![ProviderKind::Hidpp],
        controls: vec![],
        features: vec![],
        dpi: None,
        lighting: None,
        error: None,
    }
}

#[test]
fn profile_matches_only_requested_pid_and_serial() {
    let profile = Profile {
        name: "Gaming".into(),
        product_id: Some(0xc539),
        product: Some("Example Logitech Device".into()),
        serial: Some("ABC123".into()),
        dpi: Some(1600),
        lighting: None,
        applications: vec![],
        auto_switch: false,
        controls: BTreeMap::new(),
    };
    assert!(profile.matches(&device()));

    let mut other = device();
    other.serial = Some("DIFFERENT".into());
    assert!(!profile.matches(&other));
}

#[test]
fn path_helpers_prefer_xdg_and_fall_back_to_home() {
    use std::path::PathBuf;
    assert_eq!(
        reforge_core::paths::config_dir_from(Some("/tmp/xdg"), Some("/home/u")),
        PathBuf::from("/tmp/xdg/reforge-logitech")
    );
    assert_eq!(
        reforge_core::paths::config_dir_from(None, Some("/home/u")),
        PathBuf::from("/home/u/.config/reforge-logitech")
    );
}
