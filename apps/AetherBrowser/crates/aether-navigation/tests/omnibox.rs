use aether_navigation::{NavigationTarget, classify_omnibox_input, resolve_omnibox_input};

#[test]
fn absolute_http_url_is_navigation() {
    assert_eq!(
        classify_omnibox_input("https://example.com/path"),
        NavigationTarget::Url("https://example.com/path".into())
    );
}

#[test]
fn scheme_less_host_is_navigation() {
    assert_eq!(
        classify_omnibox_input("example.com"),
        NavigationTarget::Url("example.com".into())
    );
    assert_eq!(
        resolve_omnibox_input("example.com"),
        Ok("https://example.com/".into())
    );
}

#[test]
fn ordinary_words_are_search_query() {
    assert_eq!(
        classify_omnibox_input("rust browser engine"),
        NavigationTarget::SearchQuery("rust browser engine".into())
    );
    assert_eq!(
        resolve_omnibox_input("rust browser engine"),
        Ok("https://duckduckgo.com/?q=rust+browser+engine".into())
    );
}

#[test]
fn empty_input_is_rejected() {
    assert_eq!(
        resolve_omnibox_input("  "),
        Err("omnibox input is empty".into())
    );
}
