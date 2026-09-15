use aether_index::{IndexRecord, IndexStore, IndexedFileId, IndexedKind};
use forgeclean_client::ManagedState;

const MAX_RESULTS: usize = 100;
const MAX_PREVIEW_CHARS: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSearchRequest {
    pub query: String,
    pub max_results: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileSearchCandidate {
    pub id: IndexedFileId,
    pub name: String,
    pub path_hint: String,
    pub size: u64,
    pub modified_unix_ms: i64,
    pub project: Option<String>,
    pub version_hint: Option<String>,
    pub forgeclean_state: Option<ManagedState>,
    pub matched_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileMetadata {
    pub id: IndexedFileId,
    pub name: String,
    pub path_hint: String,
    pub size: u64,
    pub modified_unix_ms: i64,
    pub kind: IndexedKind,
    pub project: Option<String>,
    pub version_hint: Option<String>,
    pub forgeclean_state: Option<ManagedState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchableText {
    pub id: IndexedFileId,
    pub text: String,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    #[error("indexed file id not found")]
    NotFound,
    #[error("search query is empty")]
    EmptyQuery,
    #[error("index error: {0}")]
    Index(String),
}

pub trait FileSearchProvider: Send + Sync {
    const CAPABILITIES: &'static [&'static str] = &["search", "metadata", "preview_text"];

    fn search(
        &self,
        request: FileSearchRequest,
    ) -> Result<Vec<FileSearchCandidate>, CapabilityError>;
    fn metadata(&self, id: &IndexedFileId) -> Result<FileMetadata, CapabilityError>;
    fn preview_text(&self, id: &IndexedFileId) -> Result<SearchableText, CapabilityError>;
}

#[derive(Debug, Clone)]
pub struct IndexFileSearchProvider {
    store: IndexStore,
}

impl IndexFileSearchProvider {
    pub fn new(store: IndexStore) -> Self {
        Self { store }
    }

    fn record(&self, id: &IndexedFileId) -> Result<IndexRecord, CapabilityError> {
        self.store
            .records()
            .map_err(|error| CapabilityError::Index(error.to_string()))?
            .into_iter()
            .find(|record| &record.id == id)
            .ok_or(CapabilityError::NotFound)
    }
}

impl FileSearchProvider for IndexFileSearchProvider {
    fn search(
        &self,
        request: FileSearchRequest,
    ) -> Result<Vec<FileSearchCandidate>, CapabilityError> {
        let query = request.query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return Err(CapabilityError::EmptyQuery);
        }

        let limit = request.max_results.clamp(1, MAX_RESULTS);
        let mut candidates = self
            .store
            .records()
            .map_err(|error| CapabilityError::Index(error.to_string()))?
            .into_iter()
            .filter_map(|record| {
                let mut matched_fields = Vec::new();
                if record.name.to_ascii_lowercase().contains(&query) {
                    matched_fields.push("name".to_owned());
                }
                if record.path_hint.to_ascii_lowercase().contains(&query) {
                    matched_fields.push("path_hint".to_owned());
                }
                if record
                    .project
                    .as_deref()
                    .is_some_and(|value| value.to_ascii_lowercase().contains(&query))
                {
                    matched_fields.push("project".to_owned());
                }
                if record
                    .version_hint
                    .as_deref()
                    .is_some_and(|value| value.to_ascii_lowercase().contains(&query))
                {
                    matched_fields.push("version_hint".to_owned());
                }
                if record
                    .searchable_text
                    .as_deref()
                    .is_some_and(|value| value.to_ascii_lowercase().contains(&query))
                {
                    matched_fields.push("searchable_text".to_owned());
                }

                if matched_fields.is_empty() {
                    return None;
                }

                Some(FileSearchCandidate {
                    id: record.id,
                    name: record.name,
                    path_hint: record.path_hint,
                    size: record.size,
                    modified_unix_ms: record.modified_unix_ms,
                    project: record.project,
                    version_hint: record.version_hint,
                    forgeclean_state: record.forgeclean_state,
                    matched_fields,
                })
            })
            .collect::<Vec<_>>();

        candidates.sort_by(|left, right| {
            right
                .matched_fields
                .len()
                .cmp(&left.matched_fields.len())
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.path_hint.cmp(&right.path_hint))
        });
        candidates.truncate(limit);
        Ok(candidates)
    }

    fn metadata(&self, id: &IndexedFileId) -> Result<FileMetadata, CapabilityError> {
        let record = self.record(id)?;
        Ok(FileMetadata {
            id: record.id,
            name: record.name,
            path_hint: record.path_hint,
            size: record.size,
            modified_unix_ms: record.modified_unix_ms,
            kind: record.kind,
            project: record.project,
            version_hint: record.version_hint,
            forgeclean_state: record.forgeclean_state,
        })
    }

    fn preview_text(&self, id: &IndexedFileId) -> Result<SearchableText, CapabilityError> {
        let record = self.record(id)?;
        let text = record.searchable_text.unwrap_or_default();
        let truncated = text.chars().count() > MAX_PREVIEW_CHARS;
        let text = text.chars().take(MAX_PREVIEW_CHARS).collect();
        Ok(SearchableText {
            id: record.id,
            text,
            truncated,
        })
    }
}
