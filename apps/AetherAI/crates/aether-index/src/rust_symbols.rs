#![forbid(unsafe_code)]

use aether_core::SourceRange;
use uuid::Uuid;

use crate::{IndexSymbol, SymbolKind};

#[derive(Debug, Clone)]
struct Candidate {
    kind: SymbolKind,
    name: String,
    range: SourceRange,
}

pub fn extract_rust_symbols(file_id: Uuid, source: &str) -> Vec<IndexSymbol> {
    let masked = mask_rust_noncode(source);
    let lines = masked.lines().collect::<Vec<_>>();
    let mut candidates = Vec::new();
    let mut pending_test = false;

    for (line_index, line) in lines.iter().enumerate() {
        let line_number = u32::try_from(line_index + 1).unwrap_or(u32::MAX);
        let trimmed = line.trim();

        if trimmed.starts_with("#[test]") {
            pending_test = true;
            if !trimmed.contains(" fn ") && !trimmed.starts_with("#[test] fn ") {
                continue;
            }
        }

        if let Some((kind, name, declaration_offset)) = declaration_on_line(trimmed, pending_test) {
            let range = declaration_range(&lines, line_index, declaration_offset);
            candidates.push(Candidate { kind, name, range });
            pending_test = false;
        } else if !trimmed.is_empty() && !trimmed.starts_with("#[") {
            pending_test = false;
        }

        let _ = line_number;
    }

    classify_methods(&mut candidates);

    let mut symbols = candidates
        .into_iter()
        .map(|candidate| IndexSymbol {
            id: Uuid::new_v4(),
            file_id,
            kind: candidate.kind,
            name: candidate.name,
            qualified_name: None,
            range: candidate.range,
            parent_symbol: None,
        })
        .collect::<Vec<_>>();

    attach_parents(&mut symbols);
    symbols.sort_by_key(|symbol| (symbol.range.start_line, symbol.range.end_line));
    symbols
}

fn declaration_on_line(line: &str, pending_test: bool) -> Option<(SymbolKind, String, usize)> {
    let normalized = strip_visibility(line);

    if let Some(name) = name_after_keyword(normalized, "mod") {
        return Some((SymbolKind::Module, name, keyword_offset(line, "mod")));
    }
    if let Some(name) = name_after_keyword(normalized, "struct") {
        return Some((SymbolKind::Struct, name, keyword_offset(line, "struct")));
    }
    if let Some(name) = name_after_keyword(normalized, "enum") {
        return Some((SymbolKind::Enum, name, keyword_offset(line, "enum")));
    }
    if let Some(name) = name_after_keyword(normalized, "trait") {
        return Some((SymbolKind::Trait, name, keyword_offset(line, "trait")));
    }
    if normalized.starts_with("impl ") || normalized == "impl" {
        let name = impl_name(normalized);
        return Some((SymbolKind::Impl, name, keyword_offset(line, "impl")));
    }
    if let Some(name) = name_after_keyword(normalized, "fn") {
        let kind = if pending_test {
            SymbolKind::Test
        } else {
            SymbolKind::Function
        };
        return Some((kind, name, keyword_offset(line, "fn")));
    }
    if let Some(name) = name_after_keyword(normalized, "const") {
        return Some((SymbolKind::Const, name, keyword_offset(line, "const")));
    }
    if let Some(name) = name_after_keyword(normalized, "static") {
        return Some((SymbolKind::Static, name, keyword_offset(line, "static")));
    }
    if let Some(rest) = normalized.strip_prefix("macro_rules!") {
        let name = first_identifier(rest)?;
        return Some((
            SymbolKind::Macro,
            name,
            keyword_offset(line, "macro_rules!"),
        ));
    }

    None
}

fn strip_visibility(line: &str) -> &str {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("pub(crate) ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("pub(super) ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("pub ") {
        rest
    } else {
        trimmed
    }
}

fn name_after_keyword(line: &str, keyword: &str) -> Option<String> {
    let rest = line.strip_prefix(keyword)?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    first_identifier(rest)
}

fn first_identifier(text: &str) -> Option<String> {
    let trimmed = text.trim_start();
    let name = trimmed
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    (!name.is_empty()).then_some(name)
}

fn impl_name(line: &str) -> String {
    let rest = line.strip_prefix("impl").unwrap_or(line).trim();
    let before_brace = rest.split('{').next().unwrap_or(rest).trim();
    if let Some((_, target)) = before_brace.rsplit_once(" for ") {
        target.trim().to_string()
    } else {
        before_brace
            .split_whitespace()
            .last()
            .unwrap_or("impl")
            .trim_matches(|ch: char| ch == '<' || ch == '>')
            .to_string()
    }
}

fn keyword_offset(line: &str, keyword: &str) -> usize {
    line.find(keyword).unwrap_or(0)
}

fn declaration_range(lines: &[&str], start_index: usize, declaration_offset: usize) -> SourceRange {
    let start_line = u32::try_from(start_index + 1).unwrap_or(u32::MAX);
    let first = &lines[start_index][declaration_offset..];

    let Some(open_offset) = first.find('{') else {
        return SourceRange {
            start_line,
            end_line: start_line,
        };
    };

    let mut depth = 0_i32;
    for (index, line) in lines.iter().enumerate().skip(start_index) {
        let slice = if index == start_index {
            &line[declaration_offset + open_offset..]
        } else {
            line
        };

        for byte in slice.bytes() {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return SourceRange {
                            start_line,
                            end_line: u32::try_from(index + 1).unwrap_or(u32::MAX),
                        };
                    }
                }
                _ => {}
            }
        }
    }

    SourceRange {
        start_line,
        end_line: u32::try_from(lines.len()).unwrap_or(u32::MAX),
    }
}

fn classify_methods(candidates: &mut [Candidate]) {
    let impl_ranges = candidates
        .iter()
        .filter(|candidate| candidate.kind == SymbolKind::Impl)
        .map(|candidate| candidate.range)
        .collect::<Vec<_>>();

    for candidate in candidates {
        if candidate.kind == SymbolKind::Function
            && impl_ranges.iter().any(|range| {
                candidate.range.start_line > range.start_line
                    && candidate.range.start_line <= range.end_line
            })
        {
            candidate.kind = SymbolKind::Method;
        }
    }
}

fn attach_parents(symbols: &mut [IndexSymbol]) {
    let snapshot = symbols
        .iter()
        .map(|symbol| (symbol.id, symbol.kind, symbol.range))
        .collect::<Vec<_>>();

    for symbol in symbols {
        let mut parent = None;
        let mut parent_span = u32::MAX;

        for (candidate_id, candidate_kind, candidate_range) in &snapshot {
            if *candidate_id == symbol.id
                || !matches!(
                    candidate_kind,
                    SymbolKind::Module
                        | SymbolKind::Impl
                        | SymbolKind::Trait
                        | SymbolKind::Enum
                        | SymbolKind::Struct
                )
            {
                continue;
            }

            if candidate_range.start_line <= symbol.range.start_line
                && candidate_range.end_line >= symbol.range.end_line
            {
                let span = candidate_range
                    .end_line
                    .saturating_sub(candidate_range.start_line);
                if span < parent_span {
                    parent = Some(*candidate_id);
                    parent_span = span;
                }
            }
        }

        symbol.parent_symbol = parent;
    }
}

fn mask_rust_noncode(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = bytes.to_vec();
    let mut index = 0_usize;
    let mut block_depth = 0_u32;

    while index < bytes.len() {
        if block_depth > 0 {
            if starts_with(bytes, index, b"/*") {
                blank(&mut output, index, 2);
                block_depth += 1;
                index += 2;
            } else if starts_with(bytes, index, b"*/") {
                blank(&mut output, index, 2);
                block_depth -= 1;
                index += 2;
            } else {
                blank_one(&mut output, bytes, index);
                index += 1;
            }
            continue;
        }

        if starts_with(bytes, index, b"//") {
            while index < bytes.len() && bytes[index] != b'\n' {
                output[index] = b' ';
                index += 1;
            }
            continue;
        }

        if starts_with(bytes, index, b"/*") {
            blank(&mut output, index, 2);
            block_depth = 1;
            index += 2;
            continue;
        }

        if let Some((prefix_len, hashes)) = raw_string_prefix(bytes, index) {
            blank(&mut output, index, prefix_len);
            index += prefix_len;
            while index < bytes.len() {
                if bytes[index] == b'"' && closing_hashes_match(bytes, index + 1, hashes) {
                    blank(&mut output, index, 1 + hashes);
                    index += 1 + hashes;
                    break;
                }
                blank_one(&mut output, bytes, index);
                index += 1;
            }
            continue;
        }

        if bytes[index] == b'"' {
            blank(&mut output, index, 1);
            index += 1;
            let mut escaped = false;
            while index < bytes.len() {
                let byte = bytes[index];
                blank_one(&mut output, bytes, index);
                index += 1;
                if byte == b'"' && !escaped {
                    break;
                }
                escaped = byte == b'\\' && !escaped;
            }
            continue;
        }

        if bytes[index] == b'\'' && looks_like_char_literal(bytes, index) {
            blank(&mut output, index, 1);
            index += 1;
            let mut escaped = false;
            while index < bytes.len() {
                let byte = bytes[index];
                blank_one(&mut output, bytes, index);
                index += 1;
                if byte == b'\'' && !escaped {
                    break;
                }
                escaped = byte == b'\\' && !escaped;
            }
            continue;
        }

        index += 1;
    }

    String::from_utf8(output).expect("mask preserves UTF-8-compatible bytes")
}

fn raw_string_prefix(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
    if bytes.get(index) != Some(&b'r') {
        return None;
    }
    let mut cursor = index + 1;
    let mut hashes = 0_usize;
    while bytes.get(cursor) == Some(&b'#') {
        hashes += 1;
        cursor += 1;
    }
    if bytes.get(cursor) == Some(&b'"') {
        Some((cursor - index + 1, hashes))
    } else {
        None
    }
}

fn closing_hashes_match(bytes: &[u8], index: usize, hashes: usize) -> bool {
    (0..hashes).all(|offset| bytes.get(index + offset) == Some(&b'#'))
}

fn looks_like_char_literal(bytes: &[u8], index: usize) -> bool {
    let limit = bytes.len().min(index + 8);
    let mut cursor = index + 1;
    let mut escaped = false;
    while cursor < limit {
        let byte = bytes[cursor];
        if byte == b'\n' {
            return false;
        }
        if byte == b'\'' && !escaped {
            return true;
        }
        escaped = byte == b'\\' && !escaped;
        cursor += 1;
    }
    false
}

fn starts_with(bytes: &[u8], index: usize, needle: &[u8]) -> bool {
    bytes.get(index..index + needle.len()) == Some(needle)
}

fn blank(output: &mut [u8], start: usize, len: usize) {
    for byte in output.iter_mut().skip(start).take(len) {
        if *byte != b'\n' {
            *byte = b' ';
        }
    }
}

fn blank_one(output: &mut [u8], original: &[u8], index: usize) {
    if original[index] != b'\n' {
        output[index] = b' ';
    }
}
