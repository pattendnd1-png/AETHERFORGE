#![forbid(unsafe_code)]

use aether_storage::models::IndexSymbolRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SymbolMatch {
    Exact,
    Prefix,
    Generic,
}

pub(crate) fn symbol_match(
    symbol: &IndexSymbolRecord,
    query_tokens: &[String],
) -> Option<SymbolMatch> {
    let name = symbol.name.to_ascii_lowercase();
    let qualified = symbol
        .qualified_name
        .as_ref()
        .map(|value| value.to_ascii_lowercase());

    if query_tokens
        .iter()
        .any(|token| token == &name || qualified.as_ref().is_some_and(|value| token == value))
    {
        return Some(SymbolMatch::Exact);
    }

    if query_tokens.iter().any(|token| {
        name.starts_with(token)
            || qualified
                .as_ref()
                .is_some_and(|value| value.starts_with(token))
    }) {
        return Some(SymbolMatch::Prefix);
    }

    query_tokens
        .iter()
        .any(|token| {
            name.contains(token)
                || qualified
                    .as_ref()
                    .is_some_and(|value| value.contains(token))
        })
        .then_some(SymbolMatch::Generic)
}
