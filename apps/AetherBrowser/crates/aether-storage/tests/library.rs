use aether_storage::{DownloadWrite, LibraryStore, PersistenceScope, SchemaVersion};

#[test]
fn library_schema_migrates_and_private_history_is_not_persisted() {
    let store = LibraryStore::open_in_memory().expect("library");
    assert_eq!(store.schema_version().expect("schema"), SchemaVersion(1));
    store
        .record_history(
            PersistenceScope::Private,
            "https://private.invalid/",
            "Private",
            1,
        )
        .unwrap();
    store
        .record_history(PersistenceScope::Normal, "https://servo.org/", "Servo", 2)
        .unwrap();
    let snapshot = store.snapshot(None, 50).unwrap();
    assert_eq!(snapshot.history.len(), 1);
    assert_eq!(snapshot.history[0].url, "https://servo.org/");
}

#[test]
fn bookmarks_downloads_and_recently_closed_round_trip() {
    let store = LibraryStore::open_in_memory().expect("library");
    let tags = vec!["rust".to_owned(), "browser".to_owned()];
    let id = store
        .upsert_bookmark("https://servo.org/", "Servo", "Engines", &tags, true, 3)
        .unwrap();
    assert!(id > 0);
    assert!(store.is_bookmarked("https://servo.org/").unwrap());
    store
        .record_download(DownloadWrite {
            id: 7,
            source_url: "https://example.invalid/file",
            destination: "/tmp/file",
            state: "active",
            received_bytes: 10,
            total_bytes: Some(100),
            unix_seconds: 4,
        })
        .unwrap();
    store
        .record_recently_closed(
            PersistenceScope::Normal,
            "https://example.invalid/",
            "Example",
            5,
        )
        .unwrap();
    let snapshot = store.snapshot(Some("servo"), 50).unwrap();
    assert_eq!(snapshot.bookmarks.len(), 1);
    let all = store.snapshot(None, 50).unwrap();
    assert_eq!(all.downloads[0].state, "active");
    assert_eq!(all.recently_closed[0].title, "Example");
}
