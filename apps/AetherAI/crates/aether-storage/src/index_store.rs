use crate::{
    SqliteStore, StorageError,
    models::{IndexChunkRecord, IndexSymbolRecord, IndexedFileRecord},
};
use rusqlite::{OptionalExtension, params};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use uuid::Uuid;

impl SqliteStore {
    pub fn upsert_indexed_file(&self, record: &IndexedFileRecord) -> Result<(), StorageError> {
        upsert_indexed_file_on(&self.connection, record)?;
        Ok(())
    }

    pub fn replace_file_index(
        &self,
        file: &IndexedFileRecord,
        chunks: &[IndexChunkRecord],
        symbols: &[IndexSymbolRecord],
        terms: &[(String, Uuid, u32)],
    ) -> Result<(), StorageError> {
        let tx = self.connection.unchecked_transaction()?;
        remove_file_index_on(&tx, file.id)?;
        upsert_indexed_file_on(&tx, file)?;

        for chunk in chunks {
            tx.execute(
                "INSERT INTO index_chunks(
                    id,file_id,ordinal,start_line,end_line,content,
                    content_hash,estimated_tokens,payload_json
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    chunk.id.to_string(),
                    chunk.file_id.to_string(),
                    chunk.ordinal,
                    chunk.start_line,
                    chunk.end_line,
                    chunk.content,
                    chunk.content_hash,
                    chunk.estimated_tokens,
                    serde_json::to_string(chunk)?,
                ],
            )?;
        }

        for symbol in symbols {
            tx.execute(
                "INSERT INTO index_symbols(
                    id,file_id,kind,name,qualified_name,start_line,
                    end_line,parent_symbol,payload_json
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    symbol.id.to_string(),
                    symbol.file_id.to_string(),
                    symbol.kind,
                    symbol.name,
                    symbol.qualified_name,
                    symbol.start_line,
                    symbol.end_line,
                    symbol.parent_symbol.map(|id| id.to_string()),
                    serde_json::to_string(symbol)?,
                ],
            )?;
        }

        for (term, chunk_id, frequency) in terms {
            tx.execute(
                "INSERT INTO index_terms(term,chunk_id,term_frequency)
                 VALUES(?1,?2,?3)",
                params![term, chunk_id.to_string(), frequency],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn remove_file_index(&self, file_id: Uuid) -> Result<(), StorageError> {
        let tx = self.connection.unchecked_transaction()?;
        remove_file_index_on(&tx, file_id)?;
        tx.commit()?;
        Ok(())
    }

    pub fn indexed_file_by_path(
        &self,
        path: &Path,
    ) -> Result<Option<IndexedFileRecord>, StorageError> {
        let payload: Option<String> = self
            .connection
            .query_row(
                "SELECT payload_json
                 FROM indexed_files
                 WHERE canonical_path=?1",
                params![path.to_string_lossy()],
                |row| row.get(0),
            )
            .optional()?;

        payload
            .map(|value| serde_json::from_str(&value).map_err(StorageError::from))
            .transpose()
    }

    pub fn lexical_candidates(
        &self,
        terms: &[String],
        allowed_roots: &[PathBuf],
        limit: usize,
    ) -> Result<Vec<IndexChunkRecord>, StorageError> {
        if terms.is_empty() || allowed_roots.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let mut ranked: BTreeMap<Uuid, (u64, IndexChunkRecord)> = BTreeMap::new();

        for term in terms {
            let mut statement = self.connection.prepare(
                "SELECT c.payload_json,f.payload_json,t.term_frequency
                 FROM index_terms t
                 JOIN index_chunks c ON c.id=t.chunk_id
                 JOIN indexed_files f ON f.id=c.file_id
                 WHERE t.term=?1",
            )?;
            let rows = statement.query_map(params![term], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, u32>(2)?,
                ))
            })?;

            for row in rows {
                let (chunk_json, file_json, frequency) = row?;
                let file: IndexedFileRecord = serde_json::from_str(&file_json)?;
                if !path_allowed(&file.canonical_path, allowed_roots) {
                    continue;
                }
                let chunk: IndexChunkRecord = serde_json::from_str(&chunk_json)?;
                ranked
                    .entry(chunk.id)
                    .and_modify(|entry| entry.0 += u64::from(frequency))
                    .or_insert((u64::from(frequency), chunk));
            }
        }

        let mut values = ranked.into_values().collect::<Vec<_>>();
        values.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.id.cmp(&right.1.id))
        });
        values.truncate(limit);
        Ok(values.into_iter().map(|(_, record)| record).collect())
    }

    pub fn symbol_candidates(
        &self,
        query: &str,
        allowed_roots: &[PathBuf],
        limit: usize,
    ) -> Result<Vec<IndexSymbolRecord>, StorageError> {
        if query.trim().is_empty() || allowed_roots.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let pattern = format!("%{query}%");
        let mut statement = self.connection.prepare(
            "SELECT s.payload_json,f.payload_json
             FROM index_symbols s
             JOIN indexed_files f ON f.id=s.file_id
             WHERE s.name LIKE ?1
             ORDER BY
                CASE WHEN s.name=?2 THEN 0 ELSE 1 END,
                s.name,
                s.id",
        )?;
        let rows = statement.query_map(params![pattern, query], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut records = Vec::new();
        for row in rows {
            let (symbol_json, file_json) = row?;
            let file: IndexedFileRecord = serde_json::from_str(&file_json)?;
            if !path_allowed(&file.canonical_path, allowed_roots) {
                continue;
            }
            records.push(serde_json::from_str(&symbol_json)?);
            if records.len() == limit {
                break;
            }
        }
        Ok(records)
    }
}

fn upsert_indexed_file_on(
    connection: &rusqlite::Connection,
    record: &IndexedFileRecord,
) -> Result<(), StorageError> {
    connection.execute(
        "INSERT INTO indexed_files(
            id,attachment_id,canonical_path,size_bytes,modified_ns,
            content_hash,index_version,extractor_version,state,
            last_indexed_at,error,payload_json
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
         ON CONFLICT(id) DO UPDATE SET
            attachment_id=excluded.attachment_id,
            canonical_path=excluded.canonical_path,
            size_bytes=excluded.size_bytes,
            modified_ns=excluded.modified_ns,
            content_hash=excluded.content_hash,
            index_version=excluded.index_version,
            extractor_version=excluded.extractor_version,
            state=excluded.state,
            last_indexed_at=excluded.last_indexed_at,
            error=excluded.error,
            payload_json=excluded.payload_json",
        params![
            record.id.to_string(),
            record.attachment_id.to_string(),
            record.canonical_path.to_string_lossy(),
            i64::try_from(record.size_bytes).unwrap_or(i64::MAX),
            record.modified_ns,
            record.content_hash,
            record.index_version,
            record.extractor_version,
            format!("{:?}", record.state),
            record.last_indexed_at.to_rfc3339(),
            record.error,
            serde_json::to_string(record)?,
        ],
    )?;
    Ok(())
}

fn remove_file_index_on(
    connection: &rusqlite::Connection,
    file_id: Uuid,
) -> Result<(), StorageError> {
    let id = file_id.to_string();
    connection.execute(
        "DELETE FROM index_terms
         WHERE chunk_id IN (
             SELECT id FROM index_chunks WHERE file_id=?1
         )",
        params![id],
    )?;
    connection.execute(
        "DELETE FROM embedding_vectors
         WHERE chunk_id IN (
             SELECT id FROM index_chunks WHERE file_id=?1
         )",
        params![id],
    )?;
    connection.execute("DELETE FROM index_chunks WHERE file_id=?1", params![id])?;
    connection.execute("DELETE FROM index_symbols WHERE file_id=?1", params![id])?;
    connection.execute("DELETE FROM indexed_files WHERE id=?1", params![id])?;
    Ok(())
}

fn path_allowed(path: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| path.starts_with(root))
}

impl SqliteStore {
    pub fn indexed_files_for_attachment(
        &self,
        attachment_id: Uuid,
    ) -> Result<Vec<IndexedFileRecord>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT payload_json
             FROM indexed_files
             WHERE attachment_id=?1
             ORDER BY canonical_path,id",
        )?;

        let rows = statement.query_map(params![attachment_id.to_string()], |row| {
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
    pub fn indexed_file_by_id(
        &self,
        file_id: Uuid,
    ) -> Result<Option<IndexedFileRecord>, StorageError> {
        let payload: Option<String> = self
            .connection
            .query_row(
                "SELECT payload_json FROM indexed_files WHERE id=?1",
                params![file_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        payload
            .map(|value| serde_json::from_str(&value).map_err(StorageError::from))
            .transpose()
    }

    pub fn indexed_files_in_roots(
        &self,
        allowed_roots: &[PathBuf],
        limit: usize,
    ) -> Result<Vec<IndexedFileRecord>, StorageError> {
        if allowed_roots.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let mut statement = self.connection.prepare(
            "SELECT payload_json
             FROM indexed_files
             ORDER BY canonical_path,id",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;

        let mut records = Vec::new();
        for row in rows {
            let record: IndexedFileRecord = serde_json::from_str(&row?)?;
            if path_allowed(&record.canonical_path, allowed_roots) {
                records.push(record);
                if records.len() == limit {
                    break;
                }
            }
        }
        Ok(records)
    }

    pub fn chunks_for_file(&self, file_id: Uuid) -> Result<Vec<IndexChunkRecord>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT payload_json
             FROM index_chunks
             WHERE file_id=?1
             ORDER BY ordinal,id",
        )?;
        let rows =
            statement.query_map(params![file_id.to_string()], |row| row.get::<_, String>(0))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(serde_json::from_str(&row?)?);
        }
        Ok(records)
    }
}

impl SqliteStore {
    pub fn save_embedding_vector(
        &self,
        chunk_id: Uuid,
        model_id: &str,
        content_hash: &str,
        vector: &[f32],
    ) -> Result<(), StorageError> {
        let blob = crate::models::vector_to_le_bytes(vector);
        self.connection.execute(
            "INSERT INTO embedding_vectors(
                chunk_id,model_id,dimensions,content_hash,vector_blob
             ) VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(chunk_id,model_id) DO UPDATE SET
                dimensions=excluded.dimensions,
                content_hash=excluded.content_hash,
                vector_blob=excluded.vector_blob",
            params![
                chunk_id.to_string(),
                model_id,
                i64::try_from(vector.len()).unwrap_or(i64::MAX),
                content_hash,
                blob,
            ],
        )?;
        Ok(())
    }

    pub fn embedding_vector(
        &self,
        chunk_id: Uuid,
        model_id: &str,
        content_hash: &str,
        dimensions: usize,
    ) -> Result<Option<Vec<f32>>, StorageError> {
        let row: Option<(i64, String, Vec<u8>)> = self
            .connection
            .query_row(
                "SELECT dimensions,content_hash,vector_blob
             FROM embedding_vectors
             WHERE chunk_id=?1 AND model_id=?2",
                params![chunk_id.to_string(), model_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;

        let Some((stored_dimensions, stored_hash, blob)) = row else {
            return Ok(None);
        };
        if stored_hash != content_hash
            || usize::try_from(stored_dimensions).ok() != Some(dimensions)
            || blob.len() != dimensions.saturating_mul(std::mem::size_of::<f32>())
        {
            return Ok(None);
        }

        let mut vector = Vec::with_capacity(dimensions);
        for bytes in blob.chunks_exact(std::mem::size_of::<f32>()) {
            let value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            if !value.is_finite() {
                return Ok(None);
            }
            vector.push(value);
        }
        Ok(Some(vector))
    }

    pub fn invalidate_embedding_models_except(
        &self,
        model_id: &str,
    ) -> Result<usize, StorageError> {
        Ok(self.connection.execute(
            "DELETE FROM embedding_vectors WHERE model_id<>?1",
            params![model_id],
        )?)
    }
}
