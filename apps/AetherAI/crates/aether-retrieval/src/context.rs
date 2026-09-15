#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use aether_core::{LocalCitation, Message, Role};
use aether_index::{
    AttachmentPermissionEvaluator, FreshnessReport, IndexActivitySink, IndexRepository,
    IndexService,
};
use thiserror::Error;

use crate::{ContextItem, ContextSource, ExclusionReason, RetrievedContext, TraceCandidate};

#[derive(Debug, Error)]
pub enum RetrievalContextError {
    #[error("freshness check failed: {0}")]
    Freshness(String),
}

pub trait FreshnessChecker: Send + Sync {
    fn ensure_fresh_path(&self, path: &Path) -> Result<FreshnessReport, String>;
}

impl<R, P, A> FreshnessChecker for IndexService<R, P, A>
where
    R: IndexRepository + Send + Sync,
    P: AttachmentPermissionEvaluator + Send + Sync,
    A: IndexActivitySink + Send + Sync,
{
    fn ensure_fresh_path(&self, path: &Path) -> Result<FreshnessReport, String> {
        self.ensure_fresh(&[path.to_path_buf()])
            .map_err(|error| error.to_string())
    }
}

pub fn ensure_retrieved_context_fresh<C>(
    context: &mut RetrievedContext,
    checker: &C,
) -> Result<Vec<LocalCitation>, RetrievalContextError>
where
    C: FreshnessChecker,
{
    let mut citations = Vec::new();
    let mut retained = Vec::with_capacity(context.items.len());

    for item in context.items.drain(..) {
        let Some(citation) = citation_for_item(&item, false) else {
            retained.push(item);
            continue;
        };

        let report = checker
            .ensure_fresh_path(&citation.canonical_path)
            .map_err(RetrievalContextError::Freshness)?;

        let changed =
            report.reindexed_files > 0 || report.removed_files > 0 || report.failed_files > 0;

        if changed {
            let mut stale = citation;
            stale.stale = true;
            citations.push(stale);
            context.trace.excluded.push(TraceCandidate {
                display_path: item.source.display_path().to_string(),
                score_milli: item.score_milli,
                rationale: item.rationale.clone(),
                exclusion: Some(ExclusionReason::Stale),
            });
        } else {
            citations.push(citation);
            retained.push(item);
        }
    }

    context.items = retained;
    context.estimated_tokens = context.items.iter().map(|item| item.estimated_tokens).sum();
    context.trace.estimated_tokens = context.estimated_tokens;
    context.trace.selected.retain(|candidate| {
        context
            .items
            .iter()
            .any(|item| item.source.display_path() == candidate.display_path)
    });

    Ok(citations)
}

pub fn citations_for_context(context: &RetrievedContext) -> Vec<LocalCitation> {
    context
        .items
        .iter()
        .filter_map(|item| citation_for_item(item, false))
        .collect()
}

pub fn build_context_messages(context: &RetrievedContext) -> Vec<Message> {
    let mut rendered = String::new();
    let mut source_index = 0_usize;
    let mut memory_index = 0_usize;
    let mut chat_index = 0_usize;

    for item in &context.items {
        match &item.source {
            ContextSource::File {
                display_path,
                range,
                ..
            }
            | ContextSource::Symbol {
                display_path,
                range,
                ..
            }
            | ContextSource::VerificationCheckpoint {
                display_path,
                range,
                ..
            } => {
                source_index += 1;
                rendered.push_str(&format!(
                    "[LOCAL_SOURCE S{source_index}]\npath={display_path}\nlines={}-{}\n{}\n[/LOCAL_SOURCE]\n",
                    range.start_line,
                    range.end_line,
                    item.content
                ));
            }
            ContextSource::Memory { display_path, .. } => {
                memory_index += 1;
                let category = display_path
                    .strip_prefix("Project Memory · ")
                    .unwrap_or(display_path);
                rendered.push_str(&format!(
                    "[PROJECT_MEMORY M{memory_index}]\ncategory={category}\n{}\n[/PROJECT_MEMORY]\n",
                    item.content
                ));
            }
            ContextSource::ExternalChat {
                source_type,
                conversation_id,
                message_id,
                ..
            } => {
                chat_index += 1;
                rendered.push_str(&format!(
                    "[IMPORTED_CHAT C{chat_index}]\nsource={source_type}\nconversation={conversation_id}\nmessage={message_id}\n{}\n[/IMPORTED_CHAT]\n",
                    item.content
                ));
            }
        }
    }

    if rendered.is_empty() {
        Vec::new()
    } else {
        vec![Message::new(Role::Developer, rendered)]
    }
}

pub fn file_paths(context: &RetrievedContext) -> Vec<PathBuf> {
    context
        .items
        .iter()
        .filter_map(|item| match &item.source {
            ContextSource::File { canonical_path, .. }
            | ContextSource::Symbol { canonical_path, .. }
            | ContextSource::VerificationCheckpoint { canonical_path, .. } => {
                Some(canonical_path.clone())
            }
            ContextSource::Memory { .. } | ContextSource::ExternalChat { .. } => None,
        })
        .collect()
}

fn citation_for_item(item: &ContextItem, stale: bool) -> Option<LocalCitation> {
    match &item.source {
        ContextSource::File {
            canonical_path,
            display_path,
            indexed_content_hash,
            range,
        }
        | ContextSource::Symbol {
            canonical_path,
            display_path,
            indexed_content_hash,
            range,
            ..
        }
        | ContextSource::VerificationCheckpoint {
            canonical_path,
            display_path,
            indexed_content_hash,
            range,
        } => Some(LocalCitation {
            canonical_path: canonical_path.clone(),
            display_path: display_path.clone(),
            range: *range,
            indexed_content_hash: indexed_content_hash.clone(),
            stale,
        }),
        ContextSource::Memory { .. } | ContextSource::ExternalChat { .. } => None,
    }
}
