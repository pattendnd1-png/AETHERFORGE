use crate::{SqliteStore, StorageError};
use aether_core::{AttachmentId, AttachmentRecord, ConversationId, IndexState};
use rusqlite::{OptionalExtension, params};

impl SqliteStore {
    pub fn save_attachment(&self, record: &AttachmentRecord) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO attachments(
                id,conversation_id,project_id,canonical_path,kind,
                permission_scope,added_at,last_seen_at,index_state,payload_json
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
             ON CONFLICT(id) DO UPDATE SET
                conversation_id=excluded.conversation_id,
                project_id=excluded.project_id,
                canonical_path=excluded.canonical_path,
                kind=excluded.kind,
                permission_scope=excluded.permission_scope,
                added_at=excluded.added_at,
                last_seen_at=excluded.last_seen_at,
                index_state=excluded.index_state,
                payload_json=excluded.payload_json",
            params![
                record.attachment_id.to_string(),
                record.conversation_id.to_string(),
                record.project_id.map(|id| id.to_string()),
                record.canonical_path.to_string_lossy(),
                format!("{:?}", record.attachment_kind),
                format!("{:?}", record.permission_scope),
                record.added_at.to_rfc3339(),
                record.last_seen_at.to_rfc3339(),
                format!("{:?}", record.index_state),
                serde_json::to_string(record)?,
            ],
        )?;
        Ok(())
    }

    pub fn list_attachments(
        &self,
        conversation_id: ConversationId,
    ) -> Result<Vec<AttachmentRecord>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT payload_json
             FROM attachments
             WHERE conversation_id=?1
             ORDER BY added_at,id",
        )?;
        let rows = statement.query_map(params![conversation_id.to_string()], |row| {
            row.get::<_, String>(0)
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(serde_json::from_str(&row?)?);
        }
        Ok(records)
    }

    pub fn mark_attachment_state(
        &self,
        attachment_id: AttachmentId,
        state: &IndexState,
    ) -> Result<(), StorageError> {
        let payload: Option<String> = self
            .connection
            .query_row(
                "SELECT payload_json FROM attachments WHERE id=?1",
                params![attachment_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        let Some(payload) = payload else {
            return Ok(());
        };

        let mut record: AttachmentRecord = serde_json::from_str(&payload)?;
        record.index_state = state.clone();

        self.connection.execute(
            "UPDATE attachments
             SET index_state=?2,payload_json=?3
             WHERE id=?1",
            params![
                attachment_id.to_string(),
                format!("{state:?}"),
                serde_json::to_string(&record)?,
            ],
        )?;
        Ok(())
    }

    pub fn remove_attachment(&self, attachment_id: AttachmentId) -> Result<(), StorageError> {
        let tx = self.connection.unchecked_transaction()?;
        let id = attachment_id.to_string();

        tx.execute(
            "DELETE FROM index_terms
             WHERE chunk_id IN (
                 SELECT id FROM index_chunks
                 WHERE file_id IN (
                     SELECT id FROM indexed_files WHERE attachment_id=?1
                 )
             )",
            params![id],
        )?;
        tx.execute(
            "DELETE FROM embedding_vectors
             WHERE chunk_id IN (
                 SELECT id FROM index_chunks
                 WHERE file_id IN (
                     SELECT id FROM indexed_files WHERE attachment_id=?1
                 )
             )",
            params![id],
        )?;
        tx.execute(
            "DELETE FROM index_chunks
             WHERE file_id IN (
                 SELECT id FROM indexed_files WHERE attachment_id=?1
             )",
            params![id],
        )?;
        tx.execute(
            "DELETE FROM index_symbols
             WHERE file_id IN (
                 SELECT id FROM indexed_files WHERE attachment_id=?1
             )",
            params![id],
        )?;
        tx.execute(
            "DELETE FROM indexed_files WHERE attachment_id=?1",
            params![id],
        )?;
        tx.execute("DELETE FROM attachments WHERE id=?1", params![id])?;
        tx.commit()?;
        Ok(())
    }
}

impl SqliteStore {
    pub fn attachment_by_id(
        &self,
        attachment_id: AttachmentId,
    ) -> Result<Option<AttachmentRecord>, StorageError> {
        use rusqlite::{OptionalExtension, params};

        let payload: Option<String> = self
            .connection
            .query_row(
                "SELECT payload_json FROM attachments WHERE id=?1",
                params![attachment_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        payload
            .map(|value| serde_json::from_str(&value).map_err(StorageError::from))
            .transpose()
    }
}
