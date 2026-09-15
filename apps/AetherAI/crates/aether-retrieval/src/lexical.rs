#![forbid(unsafe_code)]

use std::collections::BTreeSet;

pub(crate) fn query_terms(query: &str) -> Vec<String> {
    let mut terms = BTreeSet::new();
    let mut token = String::new();

    let flush = |token: &mut String, terms: &mut BTreeSet<String>| {
        if token.is_empty() {
            return;
        }
        let original = std::mem::take(token);
        let lower = original.to_ascii_lowercase();
        if !lower.is_empty() {
            terms.insert(lower.clone());
        }

        for part in lower.split(['_', '-', '/', '.', ':']) {
            if part.len() >= 2 {
                terms.insert(part.to_string());
            }
        }

        for part in camel_parts(&original) {
            let lower = part.to_ascii_lowercase();
            if lower.len() >= 2 {
                terms.insert(lower);
            }
        }
    };

    for ch in query.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/' | '.' | ':') {
            token.push(ch);
        } else {
            flush(&mut token, &mut terms);
        }
    }
    flush(&mut token, &mut terms);

    terms.into_iter().collect()
}

pub(crate) fn lexical_points(content: &str, terms: &[String]) -> (i32, usize) {
    let lower = content.to_ascii_lowercase();
    let mut distinct = 0_i32;
    let mut total = 0_usize;

    for term in terms {
        if term.is_empty() {
            continue;
        }
        let count = lower.matches(term).count();
        if count > 0 {
            distinct += 1;
            total = total.saturating_add(count);
        }
    }

    let exact_term_points = (distinct * 100).min(500);
    let tf_points = i32::try_from(total.saturating_mul(10))
        .unwrap_or(i32::MAX)
        .min(100);
    (exact_term_points + tf_points, total)
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

        let boundary = ((previous.is_ascii_lowercase() || previous.is_ascii_digit())
            && current.is_ascii_uppercase())
            || (previous.is_ascii_uppercase()
                && current.is_ascii_uppercase()
                && next.is_some_and(|value| value.is_ascii_lowercase()));

        if boundary {
            parts.push(chars[start..index].iter().collect());
            start = index;
        }
    }

    parts.push(chars[start..].iter().collect());
    parts
}
