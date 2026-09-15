use aether_session::{SessionError, SessionSnapshot, TabId, TabState, WindowId, WindowSession};

#[test]
fn private_windows_are_excluded_from_restorable_snapshot() {
    let normal = WindowSession::new(
        WindowId::new(1),
        false,
        vec![TabState::new(
            TabId::new(10),
            "https://example.com",
            "Example",
        )],
    )
    .unwrap();
    let private = WindowSession::new(
        WindowId::new(2),
        true,
        vec![TabState::new(
            TabId::new(20),
            "https://private.example",
            "Private",
        )],
    )
    .unwrap();

    let snapshot = SessionSnapshot::restorable_from(vec![normal, private]);
    assert_eq!(snapshot.windows().len(), 1);
    assert_eq!(snapshot.windows()[0].id(), WindowId::new(1));
}

#[test]
fn duplicate_tab_ids_are_rejected() {
    let duplicate = TabState::new(TabId::new(10), "https://example.com", "Example");
    let result = WindowSession::new(WindowId::new(1), false, vec![duplicate.clone(), duplicate]);
    assert_eq!(result, Err(SessionError::DuplicateTabId(TabId::new(10))));
}
