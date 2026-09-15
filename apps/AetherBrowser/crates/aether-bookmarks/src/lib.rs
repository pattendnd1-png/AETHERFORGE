#![forbid(unsafe_code)]
//! Bookmark tree, folder, tag, and favorite contracts backed by Aether Library.

use aether_storage::{BookmarkRow, LibraryStore, StorageError, unix_now};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bookmark {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub folder: String,
    pub tags: Vec<String>,
    pub favorite: bool,
}

impl From<BookmarkRow> for Bookmark {
    fn from(row: BookmarkRow) -> Self {
        Self {
            id: row.id.max(0) as u64,
            url: row.url,
            title: row.title,
            folder: row.folder,
            tags: row.tags,
            favorite: row.favorite,
        }
    }
}

pub fn save(store: &LibraryStore, bookmark: &Bookmark) -> Result<u64, StorageError> {
    let id = store.upsert_bookmark(
        &bookmark.url,
        &bookmark.title,
        &bookmark.folder,
        &bookmark.tags,
        bookmark.favorite,
        unix_now(),
    )?;
    Ok(id.max(0) as u64)
}

pub fn toggle_current(store: &LibraryStore, url: &str, title: &str) -> Result<bool, StorageError> {
    if store.is_bookmarked(url)? {
        store.remove_bookmark_for_url(url)?;
        Ok(false)
    } else {
        store.upsert_bookmark(url, title, "", &[], true, unix_now())?;
        Ok(true)
    }
}

pub fn query(
    store: &LibraryStore,
    needle: Option<&str>,
    limit: usize,
) -> Result<Vec<Bookmark>, StorageError> {
    Ok(store
        .snapshot(needle, limit)?
        .bookmarks
        .into_iter()
        .map(Bookmark::from)
        .collect())
}
