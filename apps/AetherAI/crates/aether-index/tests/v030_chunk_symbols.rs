use std::collections::BTreeMap;

use aether_core::SourceRange;
use aether_index::{
    ChunkBudget, FileKind, SymbolKind, chunk_file, extract_rust_symbols, terms_for_chunk,
};
use uuid::Uuid;

#[test]
fn extracts_rust_types_functions_impls_and_tests_with_exact_lines() {
    let src = r#"mod inner {
struct Widget { value: u32 }
impl Widget {
    fn value(&self) -> u32 { self.value }
}
#[test]
fn widget_value_works() { assert_eq!(1, 1); } }"#;

    let symbols = extract_rust_symbols(Uuid::nil(), src);
    let locate = |kind, name| {
        symbols
            .iter()
            .find(|symbol| symbol.kind == kind && symbol.name == name)
            .unwrap()
            .range
    };

    assert_eq!(
        locate(SymbolKind::Module, "inner"),
        SourceRange {
            start_line: 1,
            end_line: 7,
        }
    );
    assert_eq!(
        locate(SymbolKind::Struct, "Widget"),
        SourceRange {
            start_line: 2,
            end_line: 2,
        }
    );
    assert_eq!(
        locate(SymbolKind::Method, "value"),
        SourceRange {
            start_line: 4,
            end_line: 4,
        }
    );
    assert_eq!(
        locate(SymbolKind::Test, "widget_value_works"),
        SourceRange {
            start_line: 7,
            end_line: 7,
        }
    );
}

#[test]
fn ignores_keywords_inside_comments_and_strings() {
    let src = r#"
// fn not_real() {}
const TEXT: &str = "struct Fake { field: u8 }";
/* enum AlsoFake { A } */
fn real() {}
"#;

    let names = extract_rust_symbols(Uuid::nil(), src)
        .into_iter()
        .map(|symbol| symbol.name)
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["TEXT", "real"]);
}

#[test]
fn raw_strings_do_not_create_false_symbols() {
    let src = "const RAW: &str = r#\"fn fake() {} struct Nope {}\"#;\nfn real() {}\n";
    let names = extract_rust_symbols(Uuid::nil(), src)
        .into_iter()
        .map(|symbol| symbol.name)
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["RAW", "real"]);
}

#[test]
fn rust_function_stays_whole_when_within_budget() {
    let src = "fn alpha() {\n    let x = 1;\n    println!(\"{x}\");\n}\n";
    let file_id = Uuid::new_v4();
    let symbols = extract_rust_symbols(file_id, src);
    let chunks = chunk_file(
        file_id,
        FileKind::Rust,
        src,
        &symbols,
        ChunkBudget {
            max_lines: 8,
            max_estimated_tokens: 128,
        },
    );

    let alpha = chunks
        .iter()
        .find(|chunk| chunk.content.contains("fn alpha"))
        .unwrap();
    assert_eq!(alpha.range.start_line, 1);
    assert_eq!(alpha.range.end_line, 4);
    assert!(alpha.content.ends_with("}\n"));
}

#[test]
fn markdown_chunks_break_on_headings() {
    let src = "# One\nfirst paragraph\n## Two\nsecond paragraph\n";
    let chunks = chunk_file(
        Uuid::new_v4(),
        FileKind::Markdown,
        src,
        &[],
        ChunkBudget::default(),
    );

    assert_eq!(chunks.len(), 2);
    assert_eq!(
        chunks[0].range,
        SourceRange {
            start_line: 1,
            end_line: 2,
        }
    );
    assert_eq!(
        chunks[1].range,
        SourceRange {
            start_line: 3,
            end_line: 4,
        }
    );
}

#[test]
fn text_and_logs_use_bounded_line_windows() {
    let src = "one\ntwo\nthree\nfour\nfive\n";
    let budget = ChunkBudget {
        max_lines: 2,
        max_estimated_tokens: 100,
    };
    let chunks = chunk_file(Uuid::new_v4(), FileKind::BuildLog, src, &[], budget);

    let ranges = chunks.iter().map(|chunk| chunk.range).collect::<Vec<_>>();
    assert_eq!(
        ranges,
        vec![
            SourceRange {
                start_line: 1,
                end_line: 2,
            },
            SourceRange {
                start_line: 3,
                end_line: 4,
            },
            SourceRange {
                start_line: 5,
                end_line: 5,
            },
        ]
    );
}

#[test]
fn chunk_hash_and_ordinals_are_deterministic() {
    let src = "one\ntwo\nthree\n";
    let budget = ChunkBudget {
        max_lines: 2,
        max_estimated_tokens: 100,
    };
    let first = chunk_file(Uuid::nil(), FileKind::Text, src, &[], budget);
    let second = chunk_file(Uuid::nil(), FileKind::Text, src, &[], budget);

    assert_eq!(first.len(), second.len());
    for (left, right) in first.iter().zip(second.iter()) {
        assert_eq!(left.ordinal, right.ordinal);
        assert_eq!(left.range, right.range);
        assert_eq!(left.content_hash, right.content_hash);
        assert_eq!(left.content, right.content);
    }
}

#[test]
fn term_normalization_is_deterministic_and_keeps_config_tokens() {
    let terms = terms_for_chunk("HTTPServer snake_case config/v1.2 snake_case");
    let expected = BTreeMap::from([
        ("case".to_string(), 2),
        ("config".to_string(), 1),
        ("config/v1.2".to_string(), 1),
        ("httpserver".to_string(), 1),
        ("http".to_string(), 1),
        ("server".to_string(), 1),
        ("snake".to_string(), 2),
        ("snake_case".to_string(), 2),
        ("v1".to_string(), 1),
        ("2".to_string(), 1),
    ]);
    assert_eq!(terms, expected);
}
