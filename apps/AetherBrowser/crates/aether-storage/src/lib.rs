#![forbid(unsafe_code)]
//! Persistent Aether Library storage backed by SQLite.

use rusqlite::{Connection, OptionalExtension, params};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SchemaVersion(pub u32);

pub trait Migration {
    fn from(&self) -> SchemaVersion;
    fn to(&self) -> SchemaVersion;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistenceScope {
    Normal,
    Private,
}

#[derive(Debug)]
pub enum StorageError {
    Sql(rusqlite::Error),
    Io(std::io::Error),
}

impl Display for StorageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql(error) => write!(formatter, "sqlite error: {error}"),
            Self::Io(error) => write!(formatter, "storage I/O error: {error}"),
        }
    }
}

impl Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}

impl From<std::io::Error> for StorageError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryRow {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub visited_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookmarkRow {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub folder: String,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub created_unix_seconds: u64,
    pub updated_unix_seconds: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DownloadWrite<'a> {
    pub id: u64,
    pub source_url: &'a str,
    pub destination: &'a str,
    pub state: &'a str,
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
    pub unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadRow {
    pub id: u64,
    pub source_url: String,
    pub destination: String,
    pub state: String,
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
    pub created_unix_seconds: u64,
    pub updated_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecentlyClosedRow {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub closed_unix_seconds: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LibrarySnapshot {
    pub history: Vec<HistoryRow>,
    pub bookmarks: Vec<BookmarkRow>,
    pub downloads: Vec<DownloadRow>,
    pub recently_closed: Vec<RecentlyClosedRow>,
}

impl LibrarySnapshot {
    #[must_use]
    pub fn active_download_count(&self) -> usize {
        self.downloads
            .iter()
            .filter(|download| matches!(download.state.as_str(), "queued" | "active" | "paused"))
            .count()
    }
}

pub struct LibraryStore {
    connection: Connection,
    path: Option<PathBuf>,
}

impl LibraryStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        let store = Self {
            connection,
            path: Some(path.to_path_buf()),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_default() -> Result<Self, StorageError> {
        Self::open(default_library_path())
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory()?;
        let store = Self {
            connection,
            path: None,
        };
        store.migrate()?;
        Ok(store)
    }

    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn schema_version(&self) -> Result<SchemaVersion, StorageError> {
        let version = self
            .connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        Ok(SchemaVersion(version))
    }

    fn migrate(&self) -> Result<(), StorageError> {
        self.connection.execute_batch(
            r#"
PRAGMA foreign_keys=ON;
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    visited_unix_seconds INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS history_visit_idx ON history(visited_unix_seconds DESC);
CREATE INDEX IF NOT EXISTS history_url_idx ON history(url);
CREATE TABLE IF NOT EXISTS bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL DEFAULT '',
    folder TEXT NOT NULL DEFAULT '',
    tags TEXT NOT NULL DEFAULT '',
    favorite INTEGER NOT NULL DEFAULT 1,
    created_unix_seconds INTEGER NOT NULL,
    updated_unix_seconds INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS bookmarks_updated_idx ON bookmarks(updated_unix_seconds DESC);
CREATE TABLE IF NOT EXISTS downloads (
    id INTEGER PRIMARY KEY,
    source_url TEXT NOT NULL,
    destination TEXT NOT NULL,
    state TEXT NOT NULL,
    received_bytes INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER,
    created_unix_seconds INTEGER NOT NULL,
    updated_unix_seconds INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS downloads_updated_idx ON downloads(updated_unix_seconds DESC);
CREATE TABLE IF NOT EXISTS recently_closed (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    closed_unix_seconds INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS recently_closed_time_idx ON recently_closed(closed_unix_seconds DESC);
PRAGMA user_version=1;
"#,
        )?;
        Ok(())
    }

    pub fn record_history(
        &self,
        scope: PersistenceScope,
        url: &str,
        title: &str,
        visited_unix_seconds: u64,
    ) -> Result<(), StorageError> {
        match scope {
            PersistenceScope::Private => return Ok(()),
            PersistenceScope::Normal => {}
        }
        if !should_persist_url(url) {
            return Ok(());
        }
        self.connection.execute(
            "INSERT INTO history(url,title,visited_unix_seconds) VALUES(?1,?2,?3)",
            params![url, title, visited_unix_seconds],
        )?;
        Ok(())
    }

    pub fn update_latest_history_title(
        &self,
        scope: PersistenceScope,
        url: &str,
        title: &str,
    ) -> Result<(), StorageError> {
        match scope {
            PersistenceScope::Private => return Ok(()),
            PersistenceScope::Normal => {}
        }
        self.connection.execute(
            "UPDATE history SET title=?1 WHERE id=(SELECT id FROM history WHERE url=?2 ORDER BY id DESC LIMIT 1)",
            params![title, url],
        )?;
        Ok(())
    }

    pub fn upsert_bookmark(
        &self,
        url: &str,
        title: &str,
        folder: &str,
        tags: &[String],
        favorite: bool,
        unix_seconds: u64,
    ) -> Result<i64, StorageError> {
        let tags = encode_tags(tags);
        self.connection.execute(
            r#"INSERT INTO bookmarks(url,title,folder,tags,favorite,created_unix_seconds,updated_unix_seconds)
VALUES(?1,?2,?3,?4,?5,?6,?6)
ON CONFLICT(url) DO UPDATE SET title=excluded.title, folder=excluded.folder, tags=excluded.tags,
favorite=excluded.favorite, updated_unix_seconds=excluded.updated_unix_seconds"#,
            params![url, title, folder, tags, if favorite { 1_i64 } else { 0_i64 }, unix_seconds],
        )?;
        let id =
            self.connection
                .query_row("SELECT id FROM bookmarks WHERE url=?1", [url], |row| {
                    row.get(0)
                })?;
        Ok(id)
    }

    pub fn remove_bookmark(&self, id: i64) -> Result<bool, StorageError> {
        Ok(self
            .connection
            .execute("DELETE FROM bookmarks WHERE id=?1", [id])?
            > 0)
    }

    pub fn remove_bookmark_for_url(&self, url: &str) -> Result<bool, StorageError> {
        Ok(self
            .connection
            .execute("DELETE FROM bookmarks WHERE url=?1", [url])?
            > 0)
    }

    pub fn bookmark_for_url(&self, url: &str) -> Result<Option<BookmarkRow>, StorageError> {
        let row = self.connection.query_row(
            "SELECT id,url,title,folder,tags,favorite,created_unix_seconds,updated_unix_seconds FROM bookmarks WHERE url=?1",
            [url],
            bookmark_from_row,
        ).optional()?;
        Ok(row)
    }

    pub fn is_bookmarked(&self, url: &str) -> Result<bool, StorageError> {
        Ok(self.bookmark_for_url(url)?.is_some())
    }

    pub fn record_download(&self, download: DownloadWrite<'_>) -> Result<(), StorageError> {
        self.connection.execute(
            r#"INSERT INTO downloads(id,source_url,destination,state,received_bytes,total_bytes,created_unix_seconds,updated_unix_seconds)
VALUES(?1,?2,?3,?4,?5,?6,?7,?7)
ON CONFLICT(id) DO UPDATE SET source_url=excluded.source_url,destination=excluded.destination,
state=excluded.state,received_bytes=excluded.received_bytes,total_bytes=excluded.total_bytes,
updated_unix_seconds=excluded.updated_unix_seconds"#,
            params![
                download.id,
                download.source_url,
                download.destination,
                download.state,
                download.received_bytes,
                download.total_bytes,
                download.unix_seconds
            ],
        )?;
        Ok(())
    }

    pub fn record_recently_closed(
        &self,
        scope: PersistenceScope,
        url: &str,
        title: &str,
        closed_unix_seconds: u64,
    ) -> Result<(), StorageError> {
        match scope {
            PersistenceScope::Private => return Ok(()),
            PersistenceScope::Normal => {}
        }
        if !should_persist_url(url) {
            return Ok(());
        }
        self.connection.execute(
            "INSERT INTO recently_closed(url,title,closed_unix_seconds) VALUES(?1,?2,?3)",
            params![url, title, closed_unix_seconds],
        )?;
        self.connection.execute(
            "DELETE FROM recently_closed WHERE id NOT IN (SELECT id FROM recently_closed ORDER BY closed_unix_seconds DESC, id DESC LIMIT 100)",
            [],
        )?;
        Ok(())
    }

    pub fn remove_recently_closed(&self, id: i64) -> Result<bool, StorageError> {
        Ok(self
            .connection
            .execute("DELETE FROM recently_closed WHERE id=?1", [id])?
            > 0)
    }

    pub fn clear_history(&self) -> Result<(), StorageError> {
        self.connection.execute("DELETE FROM history", [])?;
        Ok(())
    }

    pub fn clear_recently_closed(&self) -> Result<(), StorageError> {
        self.connection.execute("DELETE FROM recently_closed", [])?;
        Ok(())
    }

    pub fn snapshot(
        &self,
        query: Option<&str>,
        limit: usize,
    ) -> Result<LibrarySnapshot, StorageError> {
        let limit = limit.clamp(1, 500);
        let mut snapshot = LibrarySnapshot {
            history: self.history(limit)?,
            bookmarks: self.bookmarks(limit)?,
            downloads: self.downloads(limit)?,
            recently_closed: self.recently_closed(limit)?,
        };
        if let Some(query) = query.map(str::trim).filter(|query| !query.is_empty()) {
            let needle = query.to_lowercase();
            snapshot
                .history
                .retain(|row| contains_query(&needle, [&row.url, &row.title]));
            snapshot.bookmarks.retain(|row| {
                contains_query(&needle, [&row.url, &row.title, &row.folder])
                    || row
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&needle))
            });
            snapshot.downloads.retain(|row| {
                contains_query(&needle, [&row.source_url, &row.destination, &row.state])
            });
            snapshot
                .recently_closed
                .retain(|row| contains_query(&needle, [&row.url, &row.title]));
        }
        Ok(snapshot)
    }

    fn history(&self, limit: usize) -> Result<Vec<HistoryRow>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id,url,title,visited_unix_seconds FROM history ORDER BY visited_unix_seconds DESC,id DESC LIMIT ?1",
        )?;
        Ok(statement
            .query_map([limit as i64], |row| {
                Ok(HistoryRow {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    visited_unix_seconds: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    fn bookmarks(&self, limit: usize) -> Result<Vec<BookmarkRow>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id,url,title,folder,tags,favorite,created_unix_seconds,updated_unix_seconds FROM bookmarks ORDER BY favorite DESC,updated_unix_seconds DESC,id DESC LIMIT ?1",
        )?;
        Ok(statement
            .query_map([limit as i64], bookmark_from_row)?
            .collect::<Result<Vec<_>, _>>()?)
    }

    fn downloads(&self, limit: usize) -> Result<Vec<DownloadRow>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id,source_url,destination,state,received_bytes,total_bytes,created_unix_seconds,updated_unix_seconds FROM downloads ORDER BY updated_unix_seconds DESC,id DESC LIMIT ?1",
        )?;
        Ok(statement
            .query_map([limit as i64], |row| {
                Ok(DownloadRow {
                    id: row.get(0)?,
                    source_url: row.get(1)?,
                    destination: row.get(2)?,
                    state: row.get(3)?,
                    received_bytes: row.get(4)?,
                    total_bytes: row.get(5)?,
                    created_unix_seconds: row.get(6)?,
                    updated_unix_seconds: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    fn recently_closed(&self, limit: usize) -> Result<Vec<RecentlyClosedRow>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id,url,title,closed_unix_seconds FROM recently_closed ORDER BY closed_unix_seconds DESC,id DESC LIMIT ?1",
        )?;
        Ok(statement
            .query_map([limit as i64], |row| {
                Ok(RecentlyClosedRow {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    closed_unix_seconds: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn self_test(&self) -> Result<(), StorageError> {
        let marker = format!("aether://library-self-test/{}", unix_now());
        let before = self.snapshot(Some(&marker), 10)?.history.len();
        self.record_history(PersistenceScope::Private, &marker, "private", unix_now())?;
        let private_after = self.snapshot(Some(&marker), 10)?.history.len();
        if private_after != before {
            return Err(StorageError::Sql(rusqlite::Error::InvalidQuery));
        }
        Ok(())
    }
}

fn bookmark_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BookmarkRow> {
    let tags: String = row.get(4)?;
    Ok(BookmarkRow {
        id: row.get(0)?,
        url: row.get(1)?,
        title: row.get(2)?,
        folder: row.get(3)?,
        tags: decode_tags(&tags),
        favorite: row.get::<_, i64>(5)? != 0,
        created_unix_seconds: row.get(6)?,
        updated_unix_seconds: row.get(7)?,
    })
}

fn encode_tags(tags: &[String]) -> String {
    tags.iter()
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>()
        .join("\u{1f}")
}

fn decode_tags(tags: &str) -> Vec<String> {
    if tags.is_empty() {
        Vec::new()
    } else {
        tags.split('\u{1f}').map(ToOwned::to_owned).collect()
    }
}

fn contains_query<'a>(needle: &str, values: impl IntoIterator<Item = &'a String>) -> bool {
    values
        .into_iter()
        .any(|value| value.to_lowercase().contains(needle))
}

#[must_use]
pub fn should_persist_url(url: &str) -> bool {
    !(url.trim().is_empty() || url == "about:blank" || url.starts_with("data:"))
}

#[must_use]
pub fn default_library_path() -> PathBuf {
    if let Some(root) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(root).join("aetherforge/aether-browser/library.sqlite3");
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".local/share/aetherforge/aether-browser/library.sqlite3")
}

#[must_use]
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
