#![forbid(unsafe_code)]

pub type MemoryId = uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MemoryScope {
    Project(uuid::Uuid),
    Conversation(uuid::Uuid),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MemoryCategory {
    Requirement,
    DesignDecision,
    VersioningRule,
    ProjectPreference,
    KnownIssue,
    ResolvedIssue,
    BuildCheckpoint,
    CurrentBaseline,
    PackagingConvention,
    FileRelationship,
    PendingWork,
    MigrationNote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MemoryStatus {
    Active,
    Superseded,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MemorySourceType {
    UserExplicit,
    UserApproved,
    DeterministicSystem,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MemoryItem {
    pub id: MemoryId,
    pub scope: MemoryScope,
    pub category: MemoryCategory,
    pub summary: String,
    pub source_links: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub confidence: f32,
    pub source_type: MemorySourceType,
    pub status: MemoryStatus,
}

impl MemoryItem {
    pub fn new(
        scope: MemoryScope,
        category: MemoryCategory,
        summary: impl Into<String>,
        source_type: MemorySourceType,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            scope,
            category,
            summary: concise_summary(summary.into()),
            source_links: Vec::new(),
            created_at: now,
            updated_at: now,
            confidence: 1.0,
            source_type,
            status: MemoryStatus::Active,
        }
    }
}

fn concise_summary(mut text: String) -> String {
    const MAX_BYTES: usize = 4095;
    if text.len() <= MAX_BYTES {
        return text;
    }

    let mut end = MAX_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text
}
