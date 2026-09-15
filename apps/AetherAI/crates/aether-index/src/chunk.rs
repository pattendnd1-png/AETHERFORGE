#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use aether_core::SourceRange;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{FileKind, IndexSymbol, SymbolKind, sha256_hex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkBudget {
    pub max_lines: u32,
    pub max_estimated_tokens: u32,
}

impl Default for ChunkBudget {
    fn default() -> Self {
        Self {
            max_lines: 120,
            max_estimated_tokens: 512,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexChunk {
    pub id: Uuid,
    pub file_id: Uuid,
    pub ordinal: u32,
    pub range: SourceRange,
    pub content: String,
    pub content_hash: String,
    pub estimated_tokens: u32,
}

pub fn chunk_file(
    file_id: Uuid,
    kind: FileKind,
    text: &str,
    symbols: &[IndexSymbol],
    budget: ChunkBudget,
) -> Vec<IndexChunk> {
    let lines = text.split_inclusive('\n').collect::<Vec<_>>();
    if lines.is_empty() {
        return Vec::new();
    }

    let ranges = match kind {
        FileKind::Rust => rust_ranges(&lines, symbols, budget),
        FileKind::Markdown => markdown_ranges(&lines, budget),
        _ => line_window_ranges(&lines, 1, lines.len(), budget),
    };

    ranges
        .into_iter()
        .enumerate()
        .map(|(ordinal, (start, end))| {
            let content = lines[start - 1..end].concat();
            let estimated_tokens = u32::try_from(content.len().div_ceil(4)).unwrap_or(u32::MAX);
            IndexChunk {
                id: Uuid::new_v4(),
                file_id,
                ordinal: u32::try_from(ordinal).unwrap_or(u32::MAX),
                range: SourceRange {
                    start_line: u32::try_from(start).unwrap_or(u32::MAX),
                    end_line: u32::try_from(end).unwrap_or(u32::MAX),
                },
                content_hash: sha256_hex(content.as_bytes()),
                content,
                estimated_tokens,
            }
        })
        .collect()
}

pub fn terms_for_chunk(content: &str) -> BTreeMap<String, u32> {
    let mut terms = BTreeMap::new();
    let mut token = String::new();

    let flush = |token: &mut String, terms: &mut BTreeMap<String, u32>| {
        if token.is_empty() {
            return;
        }

        let original = std::mem::take(token);
        let normalized = original.to_ascii_lowercase();
        add_term(terms, &normalized);

        for part in normalized.split(['_', '-', '/', '.']) {
            if !part.is_empty() && part != normalized {
                add_term(terms, part);
            }
        }

        for part in camel_parts(&original) {
            let lower = part.to_ascii_lowercase();
            if !lower.is_empty() && lower != normalized {
                add_term(terms, &lower);
            }
        }
    };

    for ch in content.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/' | '.') {
            token.push(ch);
        } else {
            flush(&mut token, &mut terms);
        }
    }
    flush(&mut token, &mut terms);

    terms
}

fn add_term(terms: &mut BTreeMap<String, u32>, term: &str) {
    *terms.entry(term.to_string()).or_insert(0) += 1;
}

fn camel_parts(token: &str) -> Vec<String> {
    let chars = token.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Vec::new();
    }

    let mut parts = Vec::new();
    let mut start = 0_usize;

    for index in 1..chars.len() {
        let previous = chars[index - 1];
        let current = chars[index];
        let next = chars.get(index + 1).copied();

        let boundary = (previous.is_ascii_lowercase() || previous.is_ascii_digit())
            && current.is_ascii_uppercase()
            || previous.is_ascii_uppercase()
                && current.is_ascii_uppercase()
                && next.is_some_and(|value| value.is_ascii_lowercase());

        if boundary {
            parts.push(chars[start..index].iter().collect::<String>());
            start = index;
        }
    }

    parts.push(chars[start..].iter().collect::<String>());
    parts
}

fn rust_ranges(
    lines: &[&str],
    symbols: &[IndexSymbol],
    budget: ChunkBudget,
) -> Vec<(usize, usize)> {
    let mut semantic = symbols
        .iter()
        .filter(|symbol| !matches!(symbol.kind, SymbolKind::Module | SymbolKind::Impl))
        .map(|symbol| {
            (
                usize::try_from(symbol.range.start_line).unwrap_or(usize::MAX),
                usize::try_from(symbol.range.end_line).unwrap_or(usize::MAX),
            )
        })
        .filter(|(start, end)| *start >= 1 && *start <= *end && *end <= lines.len())
        .collect::<Vec<_>>();

    semantic.sort_unstable();
    semantic.dedup();

    let mut ranges = Vec::new();
    let mut cursor = 1_usize;

    for (start, end) in semantic {
        if start < cursor {
            continue;
        }

        if cursor < start {
            ranges.extend(line_window_ranges(lines, cursor, start - 1, budget));
        }

        if range_fits(lines, start, end, budget) {
            ranges.push((start, end));
        } else {
            ranges.extend(line_window_ranges(lines, start, end, budget));
        }
        cursor = end + 1;
    }

    if cursor <= lines.len() {
        ranges.extend(line_window_ranges(lines, cursor, lines.len(), budget));
    }

    if ranges.is_empty() {
        line_window_ranges(lines, 1, lines.len(), budget)
    } else {
        merge_adjacent_emptyish(lines, ranges, budget)
    }
}

fn markdown_ranges(lines: &[&str], budget: ChunkBudget) -> Vec<(usize, usize)> {
    let headings = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.trim_start().starts_with('#').then_some(index + 1))
        .collect::<Vec<_>>();

    if headings.is_empty() {
        return line_window_ranges(lines, 1, lines.len(), budget);
    }

    let mut ranges = Vec::new();
    if headings[0] > 1 {
        ranges.extend(line_window_ranges(lines, 1, headings[0] - 1, budget));
    }

    for (position, start) in headings.iter().copied().enumerate() {
        let end = headings
            .get(position + 1)
            .copied()
            .map_or(lines.len(), |next| next - 1);
        if range_fits(lines, start, end, budget) {
            ranges.push((start, end));
        } else {
            ranges.extend(line_window_ranges(lines, start, end, budget));
        }
    }

    ranges
}

fn line_window_ranges(
    lines: &[&str],
    start: usize,
    end: usize,
    budget: ChunkBudget,
) -> Vec<(usize, usize)> {
    if start > end || start == 0 {
        return Vec::new();
    }

    let max_lines = usize::try_from(budget.max_lines.max(1)).unwrap_or(usize::MAX);
    let max_tokens = usize::try_from(budget.max_estimated_tokens.max(1)).unwrap_or(usize::MAX);

    let mut ranges = Vec::new();
    let mut cursor = start;

    while cursor <= end {
        let mut next_end = cursor;
        let mut bytes = 0_usize;

        while next_end <= end && next_end - cursor < max_lines {
            let candidate_bytes = bytes.saturating_add(lines[next_end - 1].len());
            if next_end > cursor && candidate_bytes.div_ceil(4) > max_tokens {
                break;
            }
            bytes = candidate_bytes;
            next_end += 1;
        }

        let actual_end = next_end.saturating_sub(1).max(cursor);
        ranges.push((cursor, actual_end));
        cursor = actual_end + 1;
    }

    ranges
}

fn range_fits(lines: &[&str], start: usize, end: usize, budget: ChunkBudget) -> bool {
    let line_count = end.saturating_sub(start) + 1;
    if line_count > usize::try_from(budget.max_lines.max(1)).unwrap_or(usize::MAX) {
        return false;
    }

    let bytes = lines[start - 1..end]
        .iter()
        .map(|line| line.len())
        .sum::<usize>();
    bytes.div_ceil(4) <= usize::try_from(budget.max_estimated_tokens.max(1)).unwrap_or(usize::MAX)
}

fn merge_adjacent_emptyish(
    lines: &[&str],
    ranges: Vec<(usize, usize)>,
    budget: ChunkBudget,
) -> Vec<(usize, usize)> {
    let mut merged: Vec<(usize, usize)> = Vec::new();

    for range in ranges {
        if let Some(last) = merged.last_mut() {
            let gap_free = last.1 + 1 == range.0;
            let left_blank = lines[last.0 - 1..last.1]
                .iter()
                .all(|line| line.trim().is_empty());
            let right_blank = lines[range.0 - 1..range.1]
                .iter()
                .all(|line| line.trim().is_empty());

            if gap_free && (left_blank || right_blank) && range_fits(lines, last.0, range.1, budget)
            {
                last.1 = range.1;
                continue;
            }
        }
        merged.push(range);
    }

    let covered = merged
        .iter()
        .flat_map(|(start, end)| *start..=*end)
        .collect::<BTreeSet<_>>();
    debug_assert_eq!(covered.len(), lines.len());

    merged
}
