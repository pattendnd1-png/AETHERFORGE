#![forbid(unsafe_code)]

use rusqlite::{OptionalExtension, params};

use crate::{
    SqliteStore, StorageError,
    models::{ExternalRecord, ExternalRecordKind, ExternalSourceType, ImportExportRecord},
};

impl SqliteStore {
    pub fn save_import_export(&self, record: &ImportExportRecord) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO import_exports(
                fingerprint,source_type,imported_at,payload_json
             ) VALUES(?1,?2,?3,?4)
             ON CONFLICT(fingerprint) DO UPDATE SET
                source_type=excluded.source_type,
                imported_at=excluded.imported_at,
                payload_json=excluded.payload_json",
            params![
                record.fingerprint,
                record.source_type.storage_key(),
                record.imported_at.to_rfc3339(),
                serde_json::to_string(record)?,
            ],
        )?;
        Ok(())
    }

    pub fn import_export_by_fingerprint(
        &self,
        fingerprint: &str,
    ) -> Result<Option<ImportExportRecord>, StorageError> {
        let payload = self
            .connection
            .query_row(
                "SELECT payload_json FROM import_exports WHERE fingerprint=?1",
                params![fingerprint],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        payload
            .map(|value| serde_json::from_str(&value).map_err(StorageError::from))
            .transpose()
    }

    pub fn upsert_external_record(&self, record: &ExternalRecord) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO external_records(
                source_type,record_kind,external_id,aether_id,
                export_fingerprint,source_timestamp,updated_at,payload_json
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(source_type,record_kind,external_id) DO UPDATE SET
                export_fingerprint=excluded.export_fingerprint,
                source_timestamp=excluded.source_timestamp,
                updated_at=excluded.updated_at,
                payload_json=excluded.payload_json",
            params![
                record.source_type.storage_key(),
                record.record_kind.storage_key(),
                record.external_id,
                record.aether_id.to_string(),
                record.export_fingerprint,
                record.source_timestamp.map(|value| value.to_rfc3339()),
                record.updated_at.to_rfc3339(),
                serde_json::to_string(record)?,
            ],
        )?;
        Ok(())
    }

    pub fn external_record(
        &self,
        source_type: ExternalSourceType,
        record_kind: ExternalRecordKind,
        external_id: &str,
    ) -> Result<Option<ExternalRecord>, StorageError> {
        let payload = self
            .connection
            .query_row(
                "SELECT payload_json FROM external_records
                 WHERE source_type=?1 AND record_kind=?2 AND external_id=?3",
                params![
                    source_type.storage_key(),
                    record_kind.storage_key(),
                    external_id,
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        payload
            .map(|value| serde_json::from_str(&value).map_err(StorageError::from))
            .transpose()
    }

    pub fn list_external_records(
        &self,
        source_type: ExternalSourceType,
    ) -> Result<Vec<ExternalRecord>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT payload_json FROM external_records
             WHERE source_type=?1
             ORDER BY record_kind,external_id",
        )?;
        let rows = statement.query_map(params![source_type.storage_key()], |row| {
            row.get::<_, String>(0)
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(serde_json::from_str(&row?)?);
        }
        Ok(records)
    }
}

impl SqliteStore {
    pub fn commit_external_import(
        &self,
        export: &ImportExportRecord,
        projects: &[crate::models::ProjectRecord],
        conversations: &[aether_core::Conversation],
        external_records: &[ExternalRecord],
    ) -> Result<(), StorageError> {
        let tx = self.connection.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO import_exports(
                fingerprint,source_type,imported_at,payload_json
             ) VALUES(?1,?2,?3,?4)
             ON CONFLICT(fingerprint) DO UPDATE SET
                source_type=excluded.source_type,
                imported_at=excluded.imported_at,
                payload_json=excluded.payload_json",
            params![
                export.fingerprint,
                export.source_type.storage_key(),
                export.imported_at.to_rfc3339(),
                serde_json::to_string(export)?,
            ],
        )?;

        for project in projects {
            tx.execute(
                "INSERT INTO projects(id,name,root,payload_json)
                 VALUES(?1,?2,?3,?4)
                 ON CONFLICT(id) DO UPDATE SET
                    name=excluded.name,
                    root=excluded.root,
                    payload_json=excluded.payload_json",
                params![
                    project.id.to_string(),
                    project.name,
                    project.root,
                    serde_json::to_string(project)?,
                ],
            )?;
        }

        for conversation in conversations {
            tx.execute(
                "INSERT INTO conversations(id,title,updated_at,payload_json)
                 VALUES(?1,?2,?3,?4)
                 ON CONFLICT(id) DO UPDATE SET
                    title=excluded.title,
                    updated_at=excluded.updated_at,
                    payload_json=excluded.payload_json",
                params![
                    conversation.id.to_string(),
                    conversation.title,
                    conversation.updated_at.to_rfc3339(),
                    serde_json::to_string(conversation)?,
                ],
            )?;
        }

        for record in external_records {
            tx.execute(
                "INSERT INTO external_records(
                    source_type,record_kind,external_id,aether_id,
                    export_fingerprint,source_timestamp,updated_at,payload_json
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
                 ON CONFLICT(source_type,record_kind,external_id) DO UPDATE SET
                    aether_id=excluded.aether_id,
                    export_fingerprint=excluded.export_fingerprint,
                    source_timestamp=excluded.source_timestamp,
                    updated_at=excluded.updated_at,
                    payload_json=excluded.payload_json",
                params![
                    record.source_type.storage_key(),
                    record.record_kind.storage_key(),
                    record.external_id,
                    record.aether_id.to_string(),
                    record.export_fingerprint,
                    record.source_timestamp.map(|value| value.to_rfc3339()),
                    record.updated_at.to_rfc3339(),
                    serde_json::to_string(record)?,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }
}
