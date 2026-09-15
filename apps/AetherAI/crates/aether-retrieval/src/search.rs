#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use aether_core::{
    Conversation, ConversationId, IndexState, MemoryCategory, MemoryId, MemoryItem, ProjectId,
    SourceRange,
};
use aether_storage::models::ProjectRecord;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchSource {
    Conversation {
        id: ConversationId,
    },
    Project {
        id: ProjectId,
    },
    File {
        path: PathBuf,
        range: Option<SourceRange>,
    },
    Symbol {
        path: PathBuf,
        range: SourceRange,
        name: String,
    },
    Memory {
        id: MemoryId,
    },
    Verification {
        path: PathBuf,
        range: SourceRange,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResult {
    pub source: SearchSource,
    pub label: String,
    pub snippet: String,
    pub score_milli: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchConversationRow {
    pub id: ConversationId,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchProjectRow {
    pub id: ProjectId,
    pub name: String,
    pub root: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchFileRow {
    pub id: Uuid,
    pub path: PathBuf,
    pub state: IndexState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchChunkRow {
    pub id: Uuid,
    pub file_id: Uuid,
    pub path: PathBuf,
    pub range: SourceRange,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSymbolRow {
    pub id: Uuid,
    pub file_id: Uuid,
    pub path: PathBuf,
    pub range: SourceRange,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchMemoryRow {
    pub id: MemoryId,
    pub category: MemoryCategory,
    pub summary: String,
}

#[derive(Debug, Error)]
pub enum UniversalSearchError {
    #[error("universal search repository error: {0}")]
    Repository(String),
}

pub trait UniversalSearchRepository {
    fn conversations(&self) -> Result<Vec<SearchConversationRow>, UniversalSearchError>;
    fn projects(&self) -> Result<Vec<SearchProjectRow>, UniversalSearchError>;
    fn files(&self) -> Result<Vec<SearchFileRow>, UniversalSearchError>;
    fn lexical_chunks(
        &self,
        terms: &[String],
        limit: usize,
    ) -> Result<Vec<SearchChunkRow>, UniversalSearchError>;
    fn symbols(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchSymbolRow>, UniversalSearchError>;
    fn memories(&self) -> Result<Vec<SearchMemoryRow>, UniversalSearchError>;
}

impl UniversalSearchRepository for aether_storage::SqliteStore {
    fn conversations(&self) -> Result<Vec<SearchConversationRow>, UniversalSearchError> {
        self.list_conversations()
            .map_err(repository_error)
            .map(|conversations| {
                conversations
                    .into_iter()
                    .map(|conversation| SearchConversationRow {
                        id: conversation.id,
                        title: conversation.title,
                    })
                    .collect()
            })
    }

    fn projects(&self) -> Result<Vec<SearchProjectRow>, UniversalSearchError> {
        self.list_projects()
            .map_err(repository_error)
            .map(|projects| {
                projects
                    .into_iter()
                    .map(|project| SearchProjectRow {
                        id: project.id,
                        name: project.name,
                        root: project.root,
                    })
                    .collect()
            })
    }

    fn files(&self) -> Result<Vec<SearchFileRow>, UniversalSearchError> {
        let roots = sqlite_search_roots(self)?;
        self.indexed_files_in_roots(&roots, 4096)
            .map_err(repository_error)
            .map(|files| {
                files
                    .into_iter()
                    .map(|file| SearchFileRow {
                        id: file.id,
                        path: file.canonical_path,
                        state: file.state,
                    })
                    .collect()
            })
    }

    fn lexical_chunks(
        &self,
        terms: &[String],
        limit: usize,
    ) -> Result<Vec<SearchChunkRow>, UniversalSearchError> {
        let roots = sqlite_search_roots(self)?;
        let chunks = self
            .lexical_candidates(terms, &roots, limit)
            .map_err(repository_error)?;
        let mut out = Vec::new();
        for chunk in chunks {
            let Some(file) = self
                .indexed_file_by_id(chunk.file_id)
                .map_err(repository_error)?
            else {
                continue;
            };
            out.push(SearchChunkRow {
                id: chunk.id,
                file_id: chunk.file_id,
                path: file.canonical_path,
                range: SourceRange {
                    start_line: chunk.start_line,
                    end_line: chunk.end_line,
                },
                content: chunk.content,
            });
        }
        Ok(out)
    }

    fn symbols(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchSymbolRow>, UniversalSearchError> {
        let roots = sqlite_search_roots(self)?;
        let symbols = self
            .symbol_candidates(query, &roots, limit)
            .map_err(repository_error)?;
        let mut out = Vec::new();
        for symbol in symbols {
            let Some(file) = self
                .indexed_file_by_id(symbol.file_id)
                .map_err(repository_error)?
            else {
                continue;
            };
            out.push(SearchSymbolRow {
                id: symbol.id,
                file_id: symbol.file_id,
                path: file.canonical_path,
                range: SourceRange {
                    start_line: symbol.start_line,
                    end_line: symbol.end_line,
                },
                name: symbol.name,
            });
        }
        Ok(out)
    }

    fn memories(&self) -> Result<Vec<SearchMemoryRow>, UniversalSearchError> {
        let projects = self.list_projects().map_err(repository_error)?;
        let conversations = self.list_conversations().map_err(repository_error)?;
        let mut memories = BTreeMap::<MemoryId, MemoryItem>::new();

        for project in projects {
            for memory in self
                .list_active_memory(Some(project.id), Uuid::nil())
                .map_err(repository_error)?
            {
                memories.insert(memory.id, memory);
            }
        }

        for conversation in conversations {
            let project_id = conversation
                .workspace
                .as_ref()
                .and_then(|workspace| workspace.project_id);
            for memory in self
                .list_active_memory(project_id, conversation.id)
                .map_err(repository_error)?
            {
                memories.insert(memory.id, memory);
            }
        }

        Ok(memories
            .into_values()
            .map(|memory| SearchMemoryRow {
                id: memory.id,
                category: memory.category,
                summary: memory.summary,
            })
            .collect())
    }
}

fn repository_error(error: impl ToString) -> UniversalSearchError {
    UniversalSearchError::Repository(error.to_string())
}

fn sqlite_search_roots(
    store: &aether_storage::SqliteStore,
) -> Result<Vec<PathBuf>, UniversalSearchError> {
    let mut roots = BTreeSet::new();

    for ProjectRecord { root, .. } in store.list_projects().map_err(repository_error)? {
        if !root.contains("://") && !root.trim().is_empty() {
            roots.insert(PathBuf::from(root));
        }
    }

    for Conversation { workspace, .. } in store.list_conversations().map_err(repository_error)? {
        if let Some(workspace) = workspace {
            for root in workspace.roots {
                if !root.path.contains("://") && !root.path.trim().is_empty() {
                    roots.insert(PathBuf::from(root.path));
                }
            }
        }
    }

    Ok(roots.into_iter().collect())
}

pub struct UniversalSearchService<R> {
    repository: R,
}

impl<R> UniversalSearchService<R>
where
    R: UniversalSearchRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, UniversalSearchError> {
        let terms = normalized_terms(query);
        if terms.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let query_lower = query.trim().to_ascii_lowercase();
        let mut results = BTreeMap::<String, SearchResult>::new();

        for row in self.repository.conversations()? {
            let score = text_score(&row.title, &query_lower, &terms, 600);
            if score > 0 {
                insert_result(
                    &mut results,
                    format!("conversation:{}", row.id),
                    SearchResult {
                        source: SearchSource::Conversation { id: row.id },
                        label: row.title.clone(),
                        snippet: format!("Conversation · {}", row.title),
                        score_milli: score,
                    },
                );
            }
        }

        for row in self.repository.projects()? {
            let score = text_score(
                &format!("{} {}", row.name, row.root),
                &query_lower,
                &terms,
                650,
            );
            if score > 0 {
                insert_result(
                    &mut results,
                    format!("project:{}", row.id),
                    SearchResult {
                        source: SearchSource::Project { id: row.id },
                        label: row.name.clone(),
                        snippet: row.root,
                        score_milli: score,
                    },
                );
            }
        }

        for row in self.repository.files()? {
            if row.state != IndexState::Indexed {
                continue;
            }
            let display = row.path.to_string_lossy();
            let filename = row
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            let mut score = text_score(&display, &query_lower, &terms, 400);
            if filename.eq_ignore_ascii_case(query.trim()) {
                score = score.max(900);
            }
            if score > 0 {
                insert_result(
                    &mut results,
                    format!("file:{}:none", row.path.display()),
                    SearchResult {
                        source: SearchSource::File {
                            path: row.path.clone(),
                            range: None,
                        },
                        label: filename.to_string(),
                        snippet: display.into_owned(),
                        score_milli: score,
                    },
                );
            }
        }

        let candidate_limit = limit.saturating_mul(16).max(64);
        for chunk in self.repository.lexical_chunks(&terms, candidate_limit)? {
            let occurrences = term_occurrences(&chunk.content, &terms);
            if occurrences == 0 {
                continue;
            }
            let score = 300_i32
                .saturating_add((i32::try_from(occurrences).unwrap_or(i32::MAX) * 20).min(400));
            let snippet = compact_snippet(&chunk.content);
            let filename = chunk
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string();

            if is_verification_path(&chunk.path) {
                insert_result(
                    &mut results,
                    format!(
                        "verification:{}:{}:{}",
                        chunk.path.display(),
                        chunk.range.start_line,
                        chunk.range.end_line
                    ),
                    SearchResult {
                        source: SearchSource::Verification {
                            path: chunk.path.clone(),
                            range: chunk.range,
                        },
                        label: filename,
                        snippet,
                        score_milli: score.saturating_add(80),
                    },
                );
            } else {
                insert_result(
                    &mut results,
                    format!(
                        "file:{}:{}:{}",
                        chunk.path.display(),
                        chunk.range.start_line,
                        chunk.range.end_line
                    ),
                    SearchResult {
                        source: SearchSource::File {
                            path: chunk.path.clone(),
                            range: Some(chunk.range),
                        },
                        label: filename,
                        snippet,
                        score_milli: score,
                    },
                );
            }
        }

        let mut seen_symbols = BTreeSet::new();
        for term in &terms {
            for symbol in self.repository.symbols(term, candidate_limit)? {
                if !seen_symbols.insert(symbol.id) {
                    continue;
                }

                let low = symbol.name.to_ascii_lowercase();
                let term_low = term.to_ascii_lowercase();
                let score = if low == term_low {
                    1100
                } else if low.starts_with(&term_low) {
                    700
                } else if low.contains(&term_low) {
                    500
                } else {
                    continue;
                };

                insert_result(
                    &mut results,
                    format!("symbol:{}:{}", symbol.path.display(), symbol.id),
                    SearchResult {
                        source: SearchSource::Symbol {
                            path: symbol.path.clone(),
                            range: symbol.range,
                            name: symbol.name.clone(),
                        },
                        label: symbol.name,
                        snippet: format!(
                            "{}:{}-{}",
                            symbol.path.display(),
                            symbol.range.start_line,
                            symbol.range.end_line
                        ),
                        score_milli: score,
                    },
                );
            }
        }

        for memory in self.repository.memories()? {
            let occurrences = term_occurrences(&memory.summary, &terms);
            if occurrences == 0 {
                continue;
            }
            let score = memory_category_score(memory.category)
                .saturating_add((i32::try_from(occurrences).unwrap_or(i32::MAX) * 120).min(600));
            insert_result(
                &mut results,
                format!("memory:{}", memory.id),
                SearchResult {
                    source: SearchSource::Memory { id: memory.id },
                    label: format!("Project Memory · {:?}", memory.category),
                    snippet: compact_snippet(&memory.summary),
                    score_milli: score,
                },
            );
        }

        let mut values = results.into_values().collect::<Vec<_>>();
        values.sort_by(|left, right| {
            right
                .score_milli
                .cmp(&left.score_milli)
                .then_with(|| source_rank(&left.source).cmp(&source_rank(&right.source)))
                .then_with(|| left.label.cmp(&right.label))
                .then_with(|| {
                    stable_source_key(&left.source).cmp(&stable_source_key(&right.source))
                })
        });
        values.truncate(limit);
        Ok(values)
    }
}

fn insert_result(
    results: &mut BTreeMap<String, SearchResult>,
    key: String,
    incoming: SearchResult,
) {
    match results.get_mut(&key) {
        Some(existing) if incoming.score_milli > existing.score_milli => *existing = incoming,
        Some(_) => {}
        None => {
            results.insert(key, incoming);
        }
    }
}

fn normalized_terms(query: &str) -> Vec<String> {
    let mut terms = query
        .split_whitespace()
        .map(|term| {
            term.trim_matches(|ch: char| {
                matches!(
                    ch,
                    '"' | '\'' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}'
                )
            })
            .to_ascii_lowercase()
        })
        .filter(|term| term.len() >= 2)
        .collect::<Vec<_>>();
    terms.sort();
    terms.dedup();
    terms
}

fn text_score(text: &str, query_lower: &str, terms: &[String], base: i32) -> i32 {
    let low = text.to_ascii_lowercase();
    let mut score = 0_i32;
    if low == query_lower {
        score = base.saturating_add(400);
    } else if !query_lower.is_empty() && low.contains(query_lower) {
        score = base.saturating_add(200);
    }

    let matched = terms
        .iter()
        .filter(|term| low.contains(term.as_str()))
        .count();
    if matched > 0 {
        score = score.max(
            base.saturating_add(
                i32::try_from(matched)
                    .unwrap_or(i32::MAX)
                    .saturating_mul(60),
            ),
        );
    }
    score
}

fn term_occurrences(text: &str, terms: &[String]) -> usize {
    let low = text.to_ascii_lowercase();
    terms
        .iter()
        .map(|term| low.match_indices(term).count())
        .sum()
}

fn compact_snippet(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(240).collect()
}

fn is_verification_path(path: &std::path::Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_uppercase();
    name.ends_with("-VERIFY.TXT") || name.contains("-PACKAGE-VERIFY")
}

fn memory_category_score(category: MemoryCategory) -> i32 {
    match category {
        MemoryCategory::CurrentBaseline => 500,
        MemoryCategory::BuildCheckpoint => 450,
        MemoryCategory::Requirement => 300,
        MemoryCategory::DesignDecision => 275,
        MemoryCategory::VersioningRule => 250,
        MemoryCategory::PackagingConvention => 225,
        _ => 175,
    }
}

fn source_rank(source: &SearchSource) -> u8 {
    match source {
        SearchSource::Symbol { .. } => 0,
        SearchSource::File { .. } => 1,
        SearchSource::Verification { .. } => 2,
        SearchSource::Memory { .. } => 3,
        SearchSource::Project { .. } => 4,
        SearchSource::Conversation { .. } => 5,
    }
}

fn stable_source_key(source: &SearchSource) -> String {
    match source {
        SearchSource::Conversation { id } => format!("conversation:{id}"),
        SearchSource::Project { id } => format!("project:{id}"),
        SearchSource::File { path, range } => format!(
            "file:{}:{:?}",
            path.display(),
            range.map(|range| (range.start_line, range.end_line))
        ),
        SearchSource::Symbol {
            path, range, name, ..
        } => format!(
            "symbol:{}:{}:{}:{name}",
            path.display(),
            range.start_line,
            range.end_line
        ),
        SearchSource::Memory { id } => format!("memory:{id}"),
        SearchSource::Verification { path, range } => format!(
            "verification:{}:{}:{}",
            path.display(),
            range.start_line,
            range.end_line
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MemoryUniversalSearchRepository {
        conversations: Vec<SearchConversationRow>,
        projects: Vec<SearchProjectRow>,
        files: Vec<SearchFileRow>,
        chunks: Vec<SearchChunkRow>,
        symbols: Vec<SearchSymbolRow>,
        memories: Vec<SearchMemoryRow>,
    }

    impl UniversalSearchRepository for MemoryUniversalSearchRepository {
        fn conversations(&self) -> Result<Vec<SearchConversationRow>, UniversalSearchError> {
            Ok(self.conversations.clone())
        }

        fn projects(&self) -> Result<Vec<SearchProjectRow>, UniversalSearchError> {
            Ok(self.projects.clone())
        }

        fn files(&self) -> Result<Vec<SearchFileRow>, UniversalSearchError> {
            Ok(self.files.clone())
        }

        fn lexical_chunks(
            &self,
            terms: &[String],
            limit: usize,
        ) -> Result<Vec<SearchChunkRow>, UniversalSearchError> {
            let mut rows = self
                .chunks
                .iter()
                .filter(|row| term_occurrences(&row.content, terms) > 0)
                .cloned()
                .collect::<Vec<_>>();
            rows.sort_by_key(|row| row.id);
            rows.truncate(limit);
            Ok(rows)
        }

        fn symbols(
            &self,
            query: &str,
            limit: usize,
        ) -> Result<Vec<SearchSymbolRow>, UniversalSearchError> {
            let low = query.to_ascii_lowercase();
            let mut rows = self
                .symbols
                .iter()
                .filter(|row| row.name.to_ascii_lowercase().contains(&low))
                .cloned()
                .collect::<Vec<_>>();
            rows.sort_by_key(|row| row.id);
            rows.truncate(limit);
            Ok(rows)
        }

        fn memories(&self) -> Result<Vec<SearchMemoryRow>, UniversalSearchError> {
            Ok(self.memories.clone())
        }
    }

    #[test]
    fn universal_search_returns_typed_file_symbol_memory_results() {
        let file_id = Uuid::from_u128(1);
        let symbol_id = Uuid::from_u128(2);
        let memory_id = Uuid::from_u128(3);
        let path = PathBuf::from("/project/src/lib.rs");
        let range = SourceRange {
            start_line: 4,
            end_line: 20,
        };

        let repo = MemoryUniversalSearchRepository {
            conversations: Vec::new(),
            projects: Vec::new(),
            files: vec![SearchFileRow {
                id: file_id,
                path: path.clone(),
                state: IndexState::Indexed,
            }],
            chunks: vec![SearchChunkRow {
                id: Uuid::from_u128(4),
                file_id,
                path: path.clone(),
                range,
                content: "pub struct ChatService;".into(),
            }],
            symbols: vec![SearchSymbolRow {
                id: symbol_id,
                file_id,
                path,
                range,
                name: "ChatService".into(),
            }],
            memories: vec![SearchMemoryRow {
                id: memory_id,
                category: MemoryCategory::CurrentBaseline,
                summary: "AetherAI 0.2.2 verified".into(),
            }],
        };

        let service = UniversalSearchService::new(repo);
        let results = service.search("ChatService 0.2.2", 20).unwrap();

        assert!(
            results
                .iter()
                .any(|result| matches!(result.source, SearchSource::Symbol { .. }))
        );
        assert!(
            results
                .iter()
                .any(|result| matches!(result.source, SearchSource::File { .. }))
        );
        assert!(
            results
                .iter()
                .any(|result| matches!(result.source, SearchSource::Memory { .. }))
        );
    }

    #[test]
    fn universal_search_order_is_deterministic() {
        let repo = MemoryUniversalSearchRepository {
            conversations: vec![
                SearchConversationRow {
                    id: Uuid::from_u128(10),
                    title: "Aether task".into(),
                },
                SearchConversationRow {
                    id: Uuid::from_u128(11),
                    title: "Aether notes".into(),
                },
            ],
            projects: Vec::new(),
            files: Vec::new(),
            chunks: Vec::new(),
            symbols: Vec::new(),
            memories: Vec::new(),
        };
        let service = UniversalSearchService::new(repo);

        let first = service.search("Aether", 20).unwrap();
        let second = service.search("Aether", 20).unwrap();
        assert_eq!(first, second);
    }
}
