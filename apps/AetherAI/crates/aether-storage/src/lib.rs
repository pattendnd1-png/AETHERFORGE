pub mod attachment_store;
pub mod import_store;
pub mod index_store;
pub mod memory_store;
pub use memory_store::{MAX_MEMORY_SUMMARY_BYTES, MemoryStoreError};
mod migrations;
pub mod models;
pub mod retrieval_store;
use aether_core::{Conversation, ConversationId};
use models::{ActivityRecord, ModelRecord, ProjectRecord, UiState};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use thiserror::Error;

pub struct SqliteStore {
    connection: Connection,
}
impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        let store = Self { connection };
        migrations::migrate(&store.connection)?;
        Ok(store)
    }
    pub fn schema_version(&self) -> Result<u32, StorageError> {
        let v: String = self.connection.query_row(
            "SELECT value FROM schema_meta WHERE key='schema_version'",
            [],
            |r| r.get(0),
        )?;
        v.parse().map_err(|_| StorageError::InvalidSchema(v))
    }
    pub fn save_conversation(&self, c: &Conversation) -> Result<(), StorageError> {
        let payload = serde_json::to_string(c)?;
        self.connection.execute("INSERT INTO conversations(id,title,updated_at,payload_json)VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET title=excluded.title,updated_at=excluded.updated_at,payload_json=excluded.payload_json",params![c.id.to_string(),c.title,c.updated_at.to_rfc3339(),payload])?;
        Ok(())
    }
    pub fn load_conversation(
        &self,
        id: ConversationId,
    ) -> Result<Option<Conversation>, StorageError> {
        let row = self
            .connection
            .query_row(
                "SELECT id,payload_json FROM conversations WHERE id=?1",
                params![id.to_string()],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?;
        let Some((row_id, payload)) = row else {
            return Ok(None);
        };
        let mut c: Conversation = serde_json::from_str(&payload)?;
        c.normalize_legacy();
        if row_id != c.id.to_string() {
            return Err(StorageError::IdMismatch {
                row_id,
                payload_id: c.id.to_string(),
            });
        }
        Ok(Some(c))
    }
    pub fn list_conversations(&self) -> Result<Vec<Conversation>, StorageError> {
        let mut s = self
            .connection
            .prepare("SELECT payload_json FROM conversations ORDER BY updated_at DESC")?;
        let rows = s.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = vec![];
        for row in rows {
            let mut c: Conversation = serde_json::from_str(&row?)?;
            c.normalize_legacy();
            out.push(c);
        }
        Ok(out)
    }
    pub fn delete_conversation(&self, id: ConversationId) -> Result<(), StorageError> {
        self.connection.execute(
            "DELETE FROM conversations WHERE id=?1",
            params![id.to_string()],
        )?;
        Ok(())
    }
    pub fn save_ui_state(&self, state: &UiState) -> Result<(), StorageError> {
        self.connection.execute("INSERT INTO ui_state(key,payload_json)VALUES('desktop',?1) ON CONFLICT(key) DO UPDATE SET payload_json=excluded.payload_json",params![serde_json::to_string(state)?])?;
        Ok(())
    }
    pub fn load_ui_state(&self) -> Result<UiState, StorageError> {
        let p: Option<String> = self
            .connection
            .query_row(
                "SELECT payload_json FROM ui_state WHERE key='desktop'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        match p {
            Some(p) => Ok(serde_json::from_str(&p)?),
            None => Ok(UiState::default()),
        }
    }
    pub fn save_project(&self, p: &ProjectRecord) -> Result<(), StorageError> {
        self.connection.execute("INSERT INTO projects(id,name,root,payload_json)VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET name=excluded.name,root=excluded.root,payload_json=excluded.payload_json",params![p.id.to_string(),p.name,p.root,serde_json::to_string(p)?])?;
        Ok(())
    }
    pub fn list_projects(&self) -> Result<Vec<ProjectRecord>, StorageError> {
        load_json_rows(
            &self.connection,
            "SELECT payload_json FROM projects ORDER BY name",
        )
    }
    pub fn save_model(&self, m: &ModelRecord) -> Result<(), StorageError> {
        let d = &m.descriptor;
        self.connection.execute("INSERT INTO models(id,backend,path,payload_json)VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET backend=excluded.backend,path=excluded.path,payload_json=excluded.payload_json",params![d.id,format!("{:?}",d.backend),d.path,serde_json::to_string(m)?])?;
        Ok(())
    }
    pub fn list_models(&self) -> Result<Vec<ModelRecord>, StorageError> {
        load_json_rows(
            &self.connection,
            "SELECT payload_json FROM models ORDER BY id",
        )
    }
    pub fn append_activity(&self, a: &ActivityRecord) -> Result<(), StorageError> {
        self.connection.execute("INSERT OR REPLACE INTO activities(id,conversation_id,project_id,started_at,payload_json)VALUES(?1,?2,?3,?4,?5)",params![a.id.to_string(),a.conversation_id.map(|v|v.to_string()),a.project_id.map(|v|v.to_string()),a.started_at.to_rfc3339(),serde_json::to_string(a)?])?;
        Ok(())
    }
    pub fn list_activities(&self) -> Result<Vec<ActivityRecord>, StorageError> {
        load_json_rows(
            &self.connection,
            "SELECT payload_json FROM activities ORDER BY started_at DESC",
        )
    }
}
fn load_json_rows<T: serde::de::DeserializeOwned>(
    c: &Connection,
    sql: &str,
) -> Result<Vec<T>, StorageError> {
    let mut s = c.prepare(sql)?;
    let rows = s.query_map([], |r| r.get::<_, String>(0))?;
    let mut out = vec![];
    for row in rows {
        out.push(serde_json::from_str(&row?)?);
    }
    Ok(out)
}
#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("stored conversation id mismatch: row={row_id}, payload={payload_id}")]
    IdMismatch { row_id: String, payload_id: String },
    #[error("unsupported schema {0}")]
    UnsupportedSchema(u32),
    #[error("invalid schema value {0}")]
    InvalidSchema(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_model_api::{ModelBackend, ModelDescriptor, ModelFormat};
    use chrono::Utc;
    use uuid::Uuid;
    #[test]
    fn round_trip_conversation_and_ui() {
        let d = tempfile::tempdir().unwrap();
        let s = SqliteStore::open(d.path().join("a.db")).unwrap();
        let mut c = Conversation::new("P");
        c.attach_workspace_root("/tmp/p", true).unwrap();
        s.save_conversation(&c).unwrap();
        assert_eq!(s.load_conversation(c.id).unwrap().unwrap(), c);
        let ui = UiState {
            left_width: 301.0,
            ..UiState::default()
        };
        s.save_ui_state(&ui).unwrap();
        assert_eq!(s.load_ui_state().unwrap(), ui);
    }
    #[test]
    fn v1_migrates_without_mock_or_identity_loss() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("v1.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);INSERT INTO schema_meta VALUES('schema_version','1');CREATE TABLE conversations(id TEXT PRIMARY KEY,title TEXT NOT NULL,updated_at TEXT NOT NULL,payload_json TEXT NOT NULL);").unwrap();
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let payload = serde_json::json!({"id":id,"title":"Legacy","kind":"Chat","created_at":now,"updated_at":now,"settings":{"model":"aetherai/mock","generation":{"preset":"Balanced","max_output_tokens":2048,"temperature":0.7,"top_p":0.95,"top_k":40,"seed":null,"context_tokens":8192,"cpu_threads":null,"gpu_layers":null},"memory_enabled":true,"web_enabled":false,"code_execution_enabled":false,"agent_depth":1,"concurrent_subagents":1},"workspace":{"id":Uuid::new_v4(),"name":"Old","root":"/tmp/old"},"messages":[]});
        conn.execute(
            "INSERT INTO conversations VALUES(?1,'Legacy',?2,?3)",
            params![id.to_string(), now, payload.to_string()],
        )
        .unwrap();
        drop(conn);
        let s = SqliteStore::open(&path).unwrap();
        assert_eq!(s.schema_version().unwrap(), 3);
        let c = s.load_conversation(id).unwrap().unwrap();
        assert_eq!(c.id, id);
        assert_eq!(c.settings.model, None);
        assert_eq!(c.workspace.unwrap().roots[0].path, "/tmp/old");
    }
    #[test]
    fn model_registry_round_trip() {
        let d = tempfile::tempdir().unwrap();
        let s = SqliteStore::open(d.path().join("a.db")).unwrap();
        let r = ModelRecord {
            descriptor: ModelDescriptor {
                id: "local".into(),
                display_name: "Local".into(),
                backend: ModelBackend::AetherGguf,
                format: ModelFormat::Gguf,
                path: "/m.gguf".into(),
                context_tokens: 8192,
                local: true,
                load_state: aether_model_api::ModelLoadState::Registered,
            },
            registered_at: Utc::now(),
            last_used: None,
        };
        s.save_model(&r).unwrap();
        assert_eq!(s.list_models().unwrap(), vec![r]);
    }
}
