use sanctuary_battlenet::{NetworkBridgeState, network_probe_hosts};

#[test]
fn official_network_probe_hosts_are_battlenet_https_endpoints() {
    let hosts = network_probe_hosts();
    assert!(hosts.contains(&"account.battle.net"));
    assert!(hosts.contains(&"download.battle.net"));
    assert!(hosts.iter().all(|host| host.ends_with(".battle.net")));
}

#[test]
fn network_bridge_state_labels_are_player_facing() {
    assert_eq!(NetworkBridgeState::Unknown.label(), "Checking");
    assert_eq!(NetworkBridgeState::Online.label(), "Online");
    assert_eq!(NetworkBridgeState::Degraded.label(), "Degraded");
    assert_eq!(NetworkBridgeState::Offline.label(), "Offline");
}
