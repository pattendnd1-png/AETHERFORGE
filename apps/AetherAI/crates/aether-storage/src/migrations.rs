use crate::StorageError;
use aether_core::Conversation;
use rusqlite::{Connection, OptionalExtension, params};

pub const SCHEMA_VERSION: u32 = 3;

const BASE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS schema_meta(
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS conversations(
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    payload_json TEXT NOT NULL
);
"#;

const V2_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS ui_state(
    key TEXT PRIMARY KEY,
    payload_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS projects(
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root TEXT NOT NULL UNIQUE,
    payload_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS workspace_roots(
    conversation_id TEXT NOT NULL,
    root TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    PRIMARY KEY(conversation_id, root)
);
CREATE TABLE IF NOT EXISTS permissions(
    scope_key TEXT NOT NULL,
    kind TEXT NOT NULL,
    decision TEXT NOT NULL,
    PRIMARY KEY(scope_key, kind)
);
CREATE TABLE IF NOT EXISTS models(
    id TEXT PRIMARY KEY,
    backend TEXT NOT NULL,
    path TEXT NOT NULL,
    payload_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS activities(
    id TEXT PRIMARY KEY,
    conversation_id TEXT,
    project_id TEXT,
    started_at TEXT NOT NULL,
    payload_json TEXT NOT NULL
);
"#;

const V3_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS attachments (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    project_id TEXT,
    canonical_path TEXT NOT NULL,
    kind TEXT NOT NULL,
    permission_scope TEXT NOT NULL,
    added_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    index_state TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    UNIQUE(conversation_id, canonical_path)
);

CREATE TABLE IF NOT EXISTS indexed_files (
    id TEXT PRIMARY KEY,
    attachment_id TEXT NOT NULL,
    canonical_path TEXT NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL,
    modified_ns INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    index_version INTEGER NOT NULL,
    extractor_version INTEGER NOT NULL,
    state TEXT NOT NULL,
    last_indexed_at TEXT NOT NULL,
    error TEXT,
    payload_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS index_chunks (
    id TEXT PRIMARY KEY,
    file_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    estimated_tokens INTEGER NOT NULL,
    payload_json TEXT NOT NULL,
    UNIQUE(file_id, ordinal)
);

CREATE TABLE IF NOT EXISTS index_terms (
    term TEXT NOT NULL,
    chunk_id TEXT NOT NULL,
    term_frequency INTEGER NOT NULL,
    PRIMARY KEY(term, chunk_id)
);

CREATE TABLE IF NOT EXISTS index_symbols (
    id TEXT PRIMARY KEY,
    file_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    qualified_name TEXT,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    parent_symbol TEXT,
    payload_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS embedding_vectors (
    chunk_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    dimensions INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    vector_blob BLOB NOT NULL,
    PRIMARY KEY(chunk_id, model_id)
);

CREATE TABLE IF NOT EXISTS project_memory (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    conversation_id TEXT,
    category TEXT NOT NULL,
    status TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    payload_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS memory_links (
    memory_id TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    source_id TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    PRIMARY KEY(memory_id, source_kind, source_id)
);

CREATE TABLE IF NOT EXISTS retrieval_history (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    project_id TEXT,
    created_at TEXT NOT NULL,
    payload_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS index_jobs (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    project_id TEXT,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    payload_json TEXT NOT NULL
);


CREATE TABLE IF NOT EXISTS import_exports (
    fingerprint TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,
    imported_at TEXT NOT NULL,
    payload_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS external_records (
    source_type TEXT NOT NULL,
    record_kind TEXT NOT NULL,
    external_id TEXT NOT NULL,
    aether_id TEXT NOT NULL,
    export_fingerprint TEXT NOT NULL,
    source_timestamp TEXT,
    updated_at TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    PRIMARY KEY(source_type, record_kind, external_id)
);

CREATE INDEX IF NOT EXISTS idx_external_records_export
    ON external_records(export_fingerprint);
CREATE INDEX IF NOT EXISTS idx_external_records_aether
    ON external_records(aether_id);

CREATE INDEX IF NOT EXISTS idx_attachments_conversation
    ON attachments(conversation_id);
CREATE INDEX IF NOT EXISTS idx_chunks_file
    ON index_chunks(file_id);
CREATE INDEX IF NOT EXISTS idx_terms_term
    ON index_terms(term);
CREATE INDEX IF NOT EXISTS idx_symbols_name
    ON index_symbols(name);
CREATE INDEX IF NOT EXISTS idx_memory_project
    ON project_memory(project_id, status);
"#;

pub fn migrate(connection: &Connection) -> Result<(), StorageError> {
    let tx = connection.unchecked_transaction()?;
    tx.execute_batch(BASE_SCHEMA)?;

    let current: Option<String> = tx
        .query_row(
            "SELECT value FROM schema_meta WHERE key='schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()?;

    let version = current
        .as_deref()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);

    if version > SCHEMA_VERSION {
        return Err(StorageError::UnsupportedSchema(version));
    }

    tx.execute_batch(V2_SCHEMA)?;

    if version == 1 {
        normalize_v1_conversations(&tx)?;
    }

    tx.execute_batch(V3_SCHEMA)?;

    tx.execute(
        "INSERT INTO schema_meta(key,value) VALUES('schema_version',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![SCHEMA_VERSION.to_string()],
    )?;

    tx.commit()?;
    Ok(())
}

fn normalize_v1_conversations(connection: &rusqlite::Transaction<'_>) -> Result<(), StorageError> {
    let mut statement = connection.prepare("SELECT id,payload_json FROM conversations")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut updates = Vec::new();
    for row in rows {
        let (id, payload) = row?;
        let mut conversation: Conversation = serde_json::from_str(&payload)?;
        conversation.normalize_legacy();
        updates.push((id, serde_json::to_string(&conversation)?));
    }
    drop(statement);

    for (id, payload) in updates {
        connection.execute(
            "UPDATE conversations SET payload_json=?2 WHERE id=?1",
            params![id, payload],
        )?;
    }

    Ok(())
}
