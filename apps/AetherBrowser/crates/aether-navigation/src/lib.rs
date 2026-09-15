#![forbid(unsafe_code)]
//! Omnibox input classification and navigation-target contracts.

use url::{Url, form_urlencoded};

pub const DEFAULT_SEARCH_ENDPOINT: &str = "https://duckduckgo.com/";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NavigationTarget {
    Url(String),
    SearchQuery(String),
}

#[must_use]
pub fn classify_omnibox_input(input: &str) -> NavigationTarget {
    let normalized = input.trim();
    if normalized.starts_with("https://")
        || normalized.starts_with("http://")
        || normalized.starts_with("about:")
        || (!normalized.contains(char::is_whitespace)
            && (normalized.contains('.') || normalized.starts_with("localhost")))
    {
        NavigationTarget::Url(normalized.to_owned())
    } else {
        NavigationTarget::SearchQuery(normalized.to_owned())
    }
}

pub fn resolve_omnibox_input(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("omnibox input is empty".into());
    }

    match classify_omnibox_input(trimmed) {
        NavigationTarget::Url(value) => {
            let candidate = if value.contains("://") || value.starts_with("about:") {
                value
            } else {
                format!("https://{value}")
            };
            Url::parse(&candidate)
                .map(|url| url.to_string())
                .map_err(|error| format!("invalid URL: {error}"))
        }
        NavigationTarget::SearchQuery(query) => {
            let mut url = Url::parse(DEFAULT_SEARCH_ENDPOINT)
                .map_err(|error| format!("invalid search endpoint: {error}"))?;
            let encoded: String = form_urlencoded::Serializer::new(String::new())
                .append_pair("q", &query)
                .finish();
            url.set_query(Some(&encoded));
            Ok(url.to_string())
        }
    }
}
