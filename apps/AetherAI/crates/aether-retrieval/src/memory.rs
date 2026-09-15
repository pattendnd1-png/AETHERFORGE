#![forbid(unsafe_code)]

use aether_core::{MemoryCategory, MemoryItem, MemoryStatus};
use chrono::{DateTime, Utc};

pub(crate) fn memory_score(
    item: &MemoryItem,
    query: &str,
    newest: Option<DateTime<Utc>>,
) -> Option<i32> {
    if item.status != MemoryStatus::Active {
        return None;
    }

    let query_terms = normalize_terms(query);
    if query_terms.is_empty() {
        return None;
    }

    let summary = item.summary.to_ascii_lowercase();
    let mut matched = 0_i32;
    let mut score = category_boost(item.category);

    for term in &query_terms {
        if summary.contains(term) {
            matched += 1;
            score = score.saturating_add(120);

            if looks_like_identifier(term) {
                score = score.saturating_add(520);
            }
        }
    }

    if matched == 0 {
        return None;
    }

    if let Some(newest) = newest {
        let age_hours = newest
            .signed_duration_since(item.updated_at)
            .num_hours()
            .max(0);
        score = score.saturating_add(50_i64.saturating_sub(age_hours.min(50)) as i32);
    }

    Some(score)
}

pub(crate) fn memory_display_path(category: MemoryCategory) -> String {
    format!("Project Memory · {category:?}")
}

fn category_boost(category: MemoryCategory) -> i32 {
    match category {
        MemoryCategory::CurrentBaseline => 500,
        MemoryCategory::BuildCheckpoint => 450,
        MemoryCategory::Requirement => 300,
        MemoryCategory::DesignDecision => 275,
        MemoryCategory::VersioningRule => 250,
        MemoryCategory::PackagingConvention => 225,
        MemoryCategory::KnownIssue
        | MemoryCategory::ResolvedIssue
        | MemoryCategory::PendingWork
        | MemoryCategory::MigrationNote => 175,
        MemoryCategory::ProjectPreference | MemoryCategory::FileRelationship => 150,
    }
}

fn normalize_terms(query: &str) -> Vec<String> {
    let mut terms = query
        .split_whitespace()
        .map(|term| {
            term.trim_matches(|ch: char| {
                matches!(ch, '"' | '\'' | ',' | ';' | '(' | ')' | '[' | ']')
            })
            .to_ascii_lowercase()
        })
        .filter(|term| term.len() >= 2)
        .collect::<Vec<_>>();
    terms.sort();
    terms.dedup();
    terms
}

fn looks_like_identifier(term: &str) -> bool {
    term.bytes().any(|byte| byte.is_ascii_digit())
        || term.contains('_')
        || term.contains('.')
        || term.contains('-')
}
