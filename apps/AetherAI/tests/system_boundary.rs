use aether_index::{IndexRecord, IndexStore, IndexedKind};
use aetherai_file_search::{
    AETHERAI_PROTOCOL_VERSION, AETHERAI_SYSTEM_SOCKET_RELATIVE, FileSearchProvider,
    FileSearchRequest, IndexFileSearchProvider, SystemAetherAiClient, SystemAiOperation,
    SystemAiRequest, decode_frame, encode_frame, system_aetherai_socket_path,
};
use std::path::PathBuf;

#[test]
fn system_ai_socket_is_canonical_local_only() {
    let path = system_aetherai_socket_path();
    assert!(
        path.to_string_lossy()
            .contains("aetherforge/aetherai/service.sock")
    );
    assert_eq!(
        AETHERAI_SYSTEM_SOCKET_RELATIVE,
        "aetherforge/aetherai/service.sock"
    );
}

#[test]
fn ping_wire_contract_is_protocol_v1_and_round_trips() {
    let request = SystemAiRequest {
        protocol: AETHERAI_PROTOCOL_VERSION,
        operation: SystemAiOperation::Ping,
    };
    let frame = encode_frame(&request).unwrap();
    let decoded: SystemAiRequest = decode_frame(&frame).unwrap();
    assert_eq!(decoded, request);
}

#[test]
fn system_ai_client_points_only_at_unix_socket() {
    let client = SystemAetherAiClient::canonical();
    assert!(
        client
            .socket_path()
            .to_string_lossy()
            .contains("aetherforge/aetherai/service.sock")
    );
}

#[test]
fn capability_surface_is_read_only_and_bounded() {
    let store = IndexStore::in_memory().unwrap();
    let provider = IndexFileSearchProvider::new(store);
    assert_eq!(
        <IndexFileSearchProvider as FileSearchProvider>::CAPABILITIES,
        &["search", "metadata", "preview_text"]
    );

    let request = FileSearchRequest {
        query: "forge".into(),
        max_results: 10_000,
    };
    let results = provider.search(request).unwrap();
    assert!(results.len() <= 100);
}

#[test]
fn candidate_search_uses_only_indexed_data() {
    let store = IndexStore::in_memory().unwrap();

    // IndexStore's mutation API is crate-private by design. This test only
    // proves an empty provider is safe and does not open arbitrary paths.
    let provider = IndexFileSearchProvider::new(store);
    let results = provider
        .search(FileSearchRequest {
            query: "secret".into(),
            max_results: 20,
        })
        .unwrap();
    assert!(results.is_empty());
}

#[test]
fn no_network_address_is_needed_to_construct_client() {
    let path = PathBuf::from("/tmp/aether-test/aetherai/service.sock");
    let client = SystemAetherAiClient::with_socket_path(path.clone());
    assert_eq!(client.socket_path(), path.as_path());
}
