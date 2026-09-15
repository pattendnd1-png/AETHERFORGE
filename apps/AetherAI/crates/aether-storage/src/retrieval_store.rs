use crate::{
    SqliteStore, StorageError,
    models::{IndexJobRecord, RetrievalHistoryRecord},
};
use rusqlite::params;

impl SqliteStore {
    pub fn append_retrieval_history(
        &self,
        record: &RetrievalHistoryRecord,
    ) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO retrieval_history(
                id,conversation_id,project_id,created_at,payload_json
             ) VALUES(?1,?2,?3,?4,?5)",
            params![
                record.id.to_string(),
                record.conversation_id.to_string(),
                record.project_id.map(|id| id.to_string()),
                record.created_at.to_rfc3339(),
                serde_json::to_string(record)?,
            ],
        )?;
        Ok(())
    }

    pub fn prune_retrieval_history(&self, keep_latest: usize) -> Result<(), StorageError> {
        if keep_latest == 0 {
            self.connection
                .execute("DELETE FROM retrieval_history", [])?;
            return Ok(());
        }

        self.connection.execute(
            "DELETE FROM retrieval_history
             WHERE id IN (
                 SELECT id
                 FROM retrieval_history
                 ORDER BY created_at DESC,id DESC
                 LIMIT -1 OFFSET ?1
             )",
            params![i64::try_from(keep_latest).unwrap_or(i64::MAX)],
        )?;
        Ok(())
    }

    pub fn retrieval_history_count(&self) -> Result<u64, StorageError> {
        let count: i64 =
            self.connection
                .query_row("SELECT COUNT(*) FROM retrieval_history", [], |row| {
                    row.get(0)
                })?;
        Ok(u64::try_from(count).unwrap_or(0))
    }

    pub fn save_index_job(&self, record: &IndexJobRecord) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO index_jobs(
                id,conversation_id,project_id,status,started_at,finished_at,payload_json
             ) VALUES(?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(id) DO UPDATE SET
                status=excluded.status,
                finished_at=excluded.finished_at,
                payload_json=excluded.payload_json",
            params![
                record.id.to_string(),
                record.conversation_id.to_string(),
                record.project_id.map(|id| id.to_string()),
                record.status,
                record.started_at.to_rfc3339(),
                record.finished_at.map(|time| time.to_rfc3339()),
                serde_json::to_string(record)?,
            ],
        )?;
        Ok(())
    }
}
