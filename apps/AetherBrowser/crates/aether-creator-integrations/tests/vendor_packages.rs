use aether_creator_integrations::vendor_packages::{
    BrowserIntegrationTarget, GROUND_CONTROL_2120, OBS_STREAMELEMENTS_LATEST, OBS_STUDIO_3222,
    STREAMLABS_DESKTOP_1219, VendorExecutionPolicy, authoritative_vendor_packages,
    ground_control_alert_actions,
};

#[test]
fn authoritative_registry_covers_all_uploaded_creator_packages() {
    let packages = authoritative_vendor_packages();
    assert_eq!(packages.len(), 4);
    assert!(packages.contains(&OBS_STUDIO_3222));
    assert!(packages.contains(&OBS_STREAMELEMENTS_LATEST));
    assert!(packages.contains(&STREAMLABS_DESKTOP_1219));
    assert!(packages.contains(&GROUND_CONTROL_2120));
    assert!(packages.iter().all(|package| package.execution_policy == VendorExecutionPolicy::MetadataReferenceOnly));
}

#[test]
fn package_targets_map_to_existing_browser_integrations() {
    assert_eq!(
        OBS_STUDIO_3222.browser_target,
        BrowserIntegrationTarget::ObsWorkspace
    );
    assert_eq!(
        OBS_STREAMELEMENTS_LATEST.browser_target,
        BrowserIntegrationTarget::StreamElementsStudio
    );
    assert_eq!(
        STREAMLABS_DESKTOP_1219.browser_target,
        BrowserIntegrationTarget::StreamlabsStudio
    );
    assert_eq!(
        GROUND_CONTROL_2120.browser_target,
        BrowserIntegrationTarget::StreamElementsGroundControl
    );
}

#[test]
fn ground_control_action_names_are_vendor_aligned() {
    assert_eq!(
        ground_control_alert_actions(),
        &[
            "Mute Alerts",
            "UnMute Alerts",
            "Pause Alerts",
            "Resume Alerts",
            "Skip Alert",
            "Toggle Alerts"
        ]
    );
}
