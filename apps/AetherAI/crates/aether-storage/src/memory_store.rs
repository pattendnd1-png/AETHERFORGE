use crate::{SqliteStore, StorageError};
use aether_core::{ConversationId, MemoryId, MemoryItem, MemoryScope, MemoryStatus, ProjectId};
use chrono::Utc;
use rusqlite::{OptionalExtension, params};

impl SqliteStore {
    pub fn save_memory(&self, item: &MemoryItem) -> Result<(), StorageError> {
        let (project_id, conversation_id) = scope_columns(item.scope);
        self.connection.execute(
            "INSERT INTO project_memory(
                id,project_id,conversation_id,category,status,updated_at,payload_json
             ) VALUES(?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(id) DO UPDATE SET
                project_id=excluded.project_id,
                conversation_id=excluded.conversation_id,
                category=excluded.category,
                status=excluded.status,
                updated_at=excluded.updated_at,
                payload_json=excluded.payload_json",
            params![
                item.id.to_string(),
                project_id,
                conversation_id,
                format!("{:?}", item.category),
                format!("{:?}", item.status),
                item.updated_at.to_rfc3339(),
                serde_json::to_string(item)?,
            ],
        )?;
        self.connection.execute(
            "DELETE FROM memory_links WHERE memory_id=?1",
            params![item.id.to_string()],
        )?;
        for source_link in &item.source_links {
            self.connection.execute(
                "INSERT INTO memory_links(memory_id,source_kind,source_id,payload_json)
                 VALUES(?1,'SourceLink',?2,?3)",
                params![
                    item.id.to_string(),
                    source_link,
                    serde_json::to_string(source_link)?,
                ],
            )?;
        }
        Ok(())
    }

    pub fn list_active_memory(
        &self,
        project_id: Option<ProjectId>,
        conversation_id: ConversationId,
    ) -> Result<Vec<MemoryItem>, StorageError> {
        let mut records = Vec::new();

        if let Some(project_id) = project_id {
            let mut statement = self.connection.prepare(
                "SELECT payload_json
                 FROM project_memory
                 WHERE status='Active'
                   AND (project_id=?1 OR conversation_id=?2)
                 ORDER BY updated_at DESC,id",
            )?;
            let rows = statement.query_map(
                params![project_id.to_string(), conversation_id.to_string()],
                |row| row.get::<_, String>(0),
            )?;
            for row in rows {
                records.push(serde_json::from_str(&row?)?);
            }
        } else {
            let mut statement = self.connection.prepare(
                "SELECT payload_json
                 FROM project_memory
                 WHERE status='Active' AND conversation_id=?1
                 ORDER BY updated_at DESC,id",
            )?;
            let rows = statement.query_map(params![conversation_id.to_string()], |row| {
                row.get::<_, String>(0)
            })?;
            for row in rows {
                records.push(serde_json::from_str(&row?)?);
            }
        }

        Ok(records)
    }

    pub fn set_memory_status(
        &self,
        memory_id: MemoryId,
        status: MemoryStatus,
    ) -> Result<(), StorageError> {
        let payload: Option<String> = self
            .connection
            .query_row(
                "SELECT payload_json FROM project_memory WHERE id=?1",
                params![memory_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        let Some(payload) = payload else {
            return Ok(());
        };

        let mut item: MemoryItem = serde_json::from_str(&payload)?;
        item.status = status;
        item.updated_at = Utc::now();

        self.connection.execute(
            "UPDATE project_memory
             SET status=?2,updated_at=?3,payload_json=?4
             WHERE id=?1",
            params![
                memory_id.to_string(),
                format!("{status:?}"),
                item.updated_at.to_rfc3339(),
                serde_json::to_string(&item)?,
            ],
        )?;
        Ok(())
    }

    pub fn delete_memory(&self, memory_id: MemoryId) -> Result<(), StorageError> {
        let tx = self.connection.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM memory_links WHERE memory_id=?1",
            params![memory_id.to_string()],
        )?;
        tx.execute(
            "DELETE FROM project_memory WHERE id=?1",
            params![memory_id.to_string()],
        )?;
        tx.commit()?;
        Ok(())
    }
}

fn scope_columns(scope: MemoryScope) -> (Option<String>, Option<String>) {
    match scope {
        MemoryScope::Project(id) => (Some(id.to_string()), None),
        MemoryScope::Conversation(id) => (None, Some(id.to_string())),
    }
}

pub const MAX_MEMORY_SUMMARY_BYTES: usize = 4095;

#[derive(Debug, thiserror::Error)]
pub enum MemoryStoreError {
    #[error("memory summary exceeds {max_bytes} bytes: {actual_bytes}")]
    SummaryTooLong {
        max_bytes: usize,
        actual_bytes: usize,
    },
    #[error("memory storage error: {0}")]
    Storage(String),
}

impl SqliteStore {
    pub fn memory_by_id(
        &self,
        memory_id: MemoryId,
    ) -> Result<Option<MemoryItem>, MemoryStoreError> {
        let payload: Option<String> = self
            .connection
            .query_row(
                "SELECT payload_json FROM project_memory WHERE id=?1",
                params![memory_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| MemoryStoreError::Storage(error.to_string()))?;

        payload
            .map(|payload| {
                serde_json::from_str(&payload)
                    .map_err(|error| MemoryStoreError::Storage(error.to_string()))
            })
            .transpose()
    }

    pub fn create_memory(
        &self,
        scope: MemoryScope,
        category: aether_core::MemoryCategory,
        summary: &str,
        source_type: aether_core::MemorySourceType,
    ) -> Result<MemoryItem, MemoryStoreError> {
        validate_summary(summary)?;
        let item = MemoryItem::new(scope, category, summary, source_type);
        self.save_memory(&item)
            .map_err(|error| MemoryStoreError::Storage(error.to_string()))?;
        Ok(item)
    }

    pub fn edit_memory(
        &self,
        memory_id: MemoryId,
        summary: &str,
    ) -> Result<Option<MemoryItem>, MemoryStoreError> {
        validate_summary(summary)?;
        let Some(mut item) = self.memory_by_id(memory_id)? else {
            return Ok(None);
        };
        item.summary = summary.to_string();
        item.updated_at = Utc::now();
        self.save_memory(&item)
            .map_err(|error| MemoryStoreError::Storage(error.to_string()))?;
        Ok(Some(item))
    }

    pub fn activate_memory(&self, memory_id: MemoryId) -> Result<(), MemoryStoreError> {
        self.set_memory_status(memory_id, MemoryStatus::Active)
            .map_err(|error| MemoryStoreError::Storage(error.to_string()))
    }

    pub fn archive_memory(&self, memory_id: MemoryId) -> Result<(), MemoryStoreError> {
        self.set_memory_status(memory_id, MemoryStatus::Archived)
            .map_err(|error| MemoryStoreError::Storage(error.to_string()))
    }

    pub fn supersede_memory(&self, memory_id: MemoryId) -> Result<(), MemoryStoreError> {
        self.set_memory_status(memory_id, MemoryStatus::Superseded)
            .map_err(|error| MemoryStoreError::Storage(error.to_string()))
    }

    pub fn upsert_checkpoint_memory(
        &self,
        project_id: ProjectId,
        version: Option<&str>,
        pass_endpoint: &str,
        source_link: &str,
    ) -> Result<(MemoryItem, MemoryItem), MemoryStoreError> {
        let version_label = version.unwrap_or("unknown");

        let build_id = deterministic_memory_id(&format!(
            "{project_id}:BuildCheckpoint:{version_label}:{pass_endpoint}:{source_link}"
        ));
        let baseline_id = deterministic_memory_id(&format!(
            "{project_id}:CurrentBaseline:{version_label}:{pass_endpoint}"
        ));

        let active = self
            .list_active_memory(Some(project_id), uuid::Uuid::nil())
            .map_err(|error| MemoryStoreError::Storage(error.to_string()))?;

        for item in active {
            if item.category == aether_core::MemoryCategory::CurrentBaseline
                && item.id != baseline_id
            {
                self.supersede_memory(item.id)?;
            }
        }

        let now = Utc::now();
        let mut build = self.memory_by_id(build_id)?.unwrap_or_else(|| MemoryItem {
            id: build_id,
            scope: MemoryScope::Project(project_id),
            category: aether_core::MemoryCategory::BuildCheckpoint,
            summary: format!("Verification PASS checkpoint {version_label}: {pass_endpoint}"),
            source_links: vec![source_link.to_string()],
            created_at: now,
            updated_at: now,
            confidence: 1.0,
            source_type: aether_core::MemorySourceType::DeterministicSystem,
            status: MemoryStatus::Active,
        });
        build.summary = format!("Verification PASS checkpoint {version_label}: {pass_endpoint}");
        build.source_links = vec![source_link.to_string()];
        build.updated_at = now;
        build.status = MemoryStatus::Active;
        self.save_memory(&build)
            .map_err(|error| MemoryStoreError::Storage(error.to_string()))?;

        let mut baseline = self
            .memory_by_id(baseline_id)?
            .unwrap_or_else(|| MemoryItem {
                id: baseline_id,
                scope: MemoryScope::Project(project_id),
                category: aether_core::MemoryCategory::CurrentBaseline,
                summary: format!("Current baseline {version_label} ({pass_endpoint})"),
                source_links: vec![source_link.to_string()],
                created_at: now,
                updated_at: now,
                confidence: 1.0,
                source_type: aether_core::MemorySourceType::DeterministicSystem,
                status: MemoryStatus::Active,
            });
        baseline.summary = format!("Current baseline {version_label} ({pass_endpoint})");
        baseline.source_links = vec![source_link.to_string()];
        baseline.updated_at = now;
        baseline.status = MemoryStatus::Active;
        self.save_memory(&baseline)
            .map_err(|error| MemoryStoreError::Storage(error.to_string()))?;

        Ok((build, baseline))
    }
}

fn validate_summary(summary: &str) -> Result<(), MemoryStoreError> {
    if summary.len() > MAX_MEMORY_SUMMARY_BYTES {
        return Err(MemoryStoreError::SummaryTooLong {
            max_bytes: MAX_MEMORY_SUMMARY_BYTES,
            actual_bytes: summary.len(),
        });
    }
    Ok(())
}

fn deterministic_memory_id(input: &str) -> MemoryId {
    let mut left = 0xcbf29ce484222325_u64;
    let mut right = 0x84222325cbf29ce4_u64;
    for byte in input.bytes() {
        left ^= u64::from(byte);
        left = left.wrapping_mul(0x100000001b3);
        right ^= u64::from(byte).wrapping_add(0x9e37);
        right = right.wrapping_mul(0x100000001b3);
    }
    uuid::Uuid::from_u128((u128::from(left) << 64) | u128::from(right))
}
