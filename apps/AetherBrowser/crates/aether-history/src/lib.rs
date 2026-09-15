#![forbid(unsafe_code)]
//! Navigation-history contracts backed by the persistent Aether Library.

use aether_storage::{HistoryRow, LibraryStore, PersistenceScope, StorageError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub visited_unix_seconds: u64,
}

impl From<HistoryRow> for HistoryEntry {
    fn from(row: HistoryRow) -> Self {
        Self {
            url: row.url,
            title: row.title,
            visited_unix_seconds: row.visited_unix_seconds,
        }
    }
}

pub fn record_visit(
    store: &LibraryStore,
    scope: PersistenceScope,
    entry: &HistoryEntry,
) -> Result<(), StorageError> {
    store.record_history(scope, &entry.url, &entry.title, entry.visited_unix_seconds)
}

pub fn query(
    store: &LibraryStore,
    needle: Option<&str>,
    limit: usize,
) -> Result<Vec<HistoryEntry>, StorageError> {
    Ok(store
        .snapshot(needle, limit)?
        .history
        .into_iter()
        .map(HistoryEntry::from)
        .collect())
}
