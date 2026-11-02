//! Syntax highlighting with tree-sitter (ED-06): Rust, JavaScript/JSX, TypeScript/TSX, Go, JSON,
//! Dart and Markdown (block level). Output is per-line spans with byte ranges relative to the line
//! start, never crossing a line break, so the editor can style each line independently.
//!
//! Limits: highlighting is purely syntactic (tree-sitter highlight queries, no semantic info);
//! Markdown has no inline styling (emphasis, links inside paragraphs); JSON does not distinguish
//! keys from string values; files above `MAX_BYTES` are not highlighted.

use std::collections::HashMap;
use std::ops::Range;
use std::path::Path;

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

mod analysis;
pub use analysis::{diagnostics, outline, Diagnostic, Symbol};

/// Files larger than this are shown unhighlighted (re-parsing on every edit would lag).
pub const MAX_BYTES: usize = 1_500_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    JavaScript,
    TypeScript,
    Tsx,
    Go,
    Json,
    Dart,
    Markdown,
}

impl Language {
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Some(match ext.as_str() {
            "rs" => Language::Rust,
            "js" | "mjs" | "cjs" | "jsx" => Language::JavaScript,
            "ts" | "mts" | "cts" => Language::TypeScript,
            "tsx" => Language::Tsx,
            "go" => Language::Go,
            "json" => Language::Json,
            "dart" => Language::Dart,
            "md" | "markdown" => Language::Markdown,
            _ => return None,
        })
    }
}

/// What a span is, coarse enough to map to a theme color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Token {
    Attribute,
    Comment,
    Constant,
    Function,
    Keyword,
    Number,
    Operator,
    Property,
    String,
    Tag,
    Type,
    Heading,
    Code,
    Link,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineSpan {
    /// Byte range within the line text (line terminator excluded).
    pub range: Range<usize>,
    pub token: Token,
}

/// Capture names we ask tree-sitter to report, in this order (index = `Highlight.0`).
const NAMES: &[&str] = &[
    "attribute",
    "comment",
    "constant",
    "constant.builtin",
    "boolean",
    "constructor",
    "escape",
    "function",
    "function.builtin",
    "function.macro",
    "function.method",
    "keyword",
    "label",
    "module",
    "number",
    "operator",
    "property",
    "string",
    "string.special",
    "string.escape",
    "tag",
    "type",
    "type.builtin",
    "variable.builtin",
    "markup.heading",
    "text.title",
    "text.literal",
    "markup.raw",
    "text.uri",
    "markup.link.url",
];

fn token_of(name: &str) -> Option<Token> {
    Some(match name {
        "attribute" => Token::Attribute,
        "comment" => Token::Comment,
        "constant" | "constant.builtin" | "boolean" | "variable.builtin" => Token::Constant,
        "constructor" | "type" | "type.builtin" | "label" | "module" => Token::Type,
        "function" | "function.builtin" | "function.macro" | "function.method" => Token::Function,
        "keyword" => Token::Keyword,
        "number" => Token::Number,
        "operator" => Token::Operator,
        "property" => Token::Property,
        "string" | "string.special" | "string.escape" | "escape" => Token::String,
        "tag" => Token::Tag,
        "markup.heading" | "text.title" => Token::Heading,
        "text.literal" | "markup.raw" => Token::Code,
        "text.uri" | "markup.link.url" => Token::Link,
        _ => return None,
    })
}

fn build(
    language: tree_sitter::Language,
    name: &str,
    highlights: &str,
    injections: &str,
    locals: &str,
) -> Option<HighlightConfiguration> {
    let mut c = HighlightConfiguration::new(language, name, highlights, injections, locals).ok()?;
    c.configure(NAMES);
    Some(c)
}

fn config_for(lang: Language) -> Option<HighlightConfiguration> {
    match lang {
        Language::Rust => build(
            tree_sitter_rust::LANGUAGE.into(),
            "rust",
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            tree_sitter_rust::INJECTIONS_QUERY,
            "",
        ),
        Language::JavaScript => build(
            tree_sitter_javascript::LANGUAGE.into(),
            "javascript",
            &format!(
                "{}\n{}",
                tree_sitter_javascript::HIGHLIGHT_QUERY,
                tree_sitter_javascript::JSX_HIGHLIGHT_QUERY
            ),
            "",
            tree_sitter_javascript::LOCALS_QUERY,
        ),
        // The TypeScript query only adds TS-specific rules; JS rules must come first.
        Language::TypeScript => build(
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            "typescript",
            &format!(
                "{}\n{}",
                tree_sitter_javascript::HIGHLIGHT_QUERY,
                tree_sitter_typescript::HIGHLIGHTS_QUERY
            ),
            "",
            &format!(
                "{}\n{}",
                tree_sitter_javascript::LOCALS_QUERY,
                tree_sitter_typescript::LOCALS_QUERY
            ),
        ),
        Language::Tsx => build(
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            "tsx",
            &format!(
                "{}\n{}\n{}",
                tree_sitter_javascript::HIGHLIGHT_QUERY,
                tree_sitter_javascript::JSX_HIGHLIGHT_QUERY,
                tree_sitter_typescript::HIGHLIGHTS_QUERY
            ),
            "",
            &format!(
                "{}\n{}",
                tree_sitter_javascript::LOCALS_QUERY,
                tree_sitter_typescript::LOCALS_QUERY
            ),
        ),
        Language::Go => build(
            tree_sitter_go::LANGUAGE.into(),
            "go",
            tree_sitter_go::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Json => build(
            tree_sitter_json::LANGUAGE.into(),
            "json",
            tree_sitter_json::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Dart => build(
            tree_sitter_dart::LANGUAGE.into(),
            "dart",
            tree_sitter_dart::HIGHLIGHTS_QUERY,
            "",
            tree_sitter_dart::LOCALS_QUERY,
        ),
        Language::Markdown => build(
            tree_sitter_md::LANGUAGE.into(),
            "markdown",
            tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
            "",
            "",
        ),
    }
}

/// Reusable highlighter: grammar configurations are built lazily and kept.
#[derive(Default)]
pub struct SyntaxHighlighter {
    configs: HashMap<Language, Option<HighlightConfiguration>>,
    highlighter: Highlighter,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Highlight `source`. Returns one `Vec<LineSpan>` per line (split on `\n`), or `None` if the
    /// grammar failed to load, the parse failed, or the file is above `MAX_BYTES`.
    pub fn highlight(&mut self, lang: Language, source: &str) -> Option<Vec<Vec<LineSpan>>> {
        if source.len() > MAX_BYTES {
            return None;
        }
        let config = self
            .configs
            .entry(lang)
            .or_insert_with(|| config_for(lang))
            .as_ref()?;

        let line_starts: Vec<usize> = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        let mut lines: Vec<Vec<LineSpan>> = vec![Vec::new(); line_starts.len()];

        let events = self
            .highlighter
            .highlight(config, source.as_bytes(), None, None, |_| None)
            .ok()?;
        let mut stack: Vec<usize> = Vec::new();
        for event in events {
            match event.ok()? {
                HighlightEvent::HighlightStart(h) => stack.push(h.0),
                HighlightEvent::HighlightEnd => {
                    stack.pop();
                }
                HighlightEvent::Source { start, end } => {
                    let Some(token) = stack
                        .last()
                        .and_then(|&i| NAMES.get(i))
                        .and_then(|n| token_of(n))
                    else {
                        continue;
                    };
                    push_split(source, &line_starts, &mut lines, start..end, token);
                }
            }
        }
        Some(lines)
    }
}

/// Add `token` over `range` (absolute bytes), split per line and clipped before `\n` / `\r\n`.
fn push_split(
    source: &str,
    line_starts: &[usize],
    lines: &mut [Vec<LineSpan>],
    range: Range<usize>,
    token: Token,
) {
    let mut line = line_starts.partition_point(|&s| s <= range.start) - 1;
    let mut pos = range.start;
    while pos < range.end && line < lines.len() {
        let start = line_starts[line];
        let mut text_end = line_starts.get(line + 1).map_or(source.len(), |&n| n - 1);
        if text_end > start && source.as_bytes()[text_end - 1] == b'\r' && text_end < source.len() {
            text_end -= 1; // exclude the CR of a CRLF terminator
        }
        let seg_end = range.end.min(text_end);
        if seg_end > pos {
            lines[line].push(LineSpan {
                range: pos - start..seg_end - start,
                token,
            });
        }
        // continue on the next line
        pos = line_starts.get(line + 1).copied().unwrap_or(range.end);
        line += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(lang: Language, src: &str) -> Vec<Vec<(String, Token)>> {
        let mut h = SyntaxHighlighter::new();
        let lines = h.highlight(lang, src).expect("highlight");
        src.split('\n')
            .zip(lines)
            .map(|(text, spans)| {
                spans
                    .into_iter()
                    .map(|s| (text[s.range.clone()].to_owned(), s.token))
                    .collect()
            })
            .collect()
    }

    fn has(line: &[(String, Token)], text: &str, token: Token) -> bool {
        line.iter().any(|(t, k)| t == text && *k == token)
    }

    #[test]
    fn rust_tokens() {
        let l = spans(
            Language::Rust,
            "use std::io;\nfn main() { let x = 1; println!(\"hi\"); } // c\n#[derive(Debug)]\nstruct S;\n",
        );
        assert!(has(&l[0], "use", Token::Keyword));
        assert!(has(&l[1], "fn", Token::Keyword));
        assert!(has(&l[1], "main", Token::Function));
        assert!(has(&l[1], "let", Token::Keyword));
        assert!(has(&l[1], "1", Token::Constant) || has(&l[1], "1", Token::Number));
        assert!(has(&l[1], "\"hi\"", Token::String));
        assert!(has(&l[1], "println", Token::Function));
        assert!(has(&l[1], "// c", Token::Comment));
        assert!(
            l[2].iter()
                .any(|(t, k)| t.contains("derive") && *k == Token::Attribute),
            "{:?}",
            l[2]
        );
        assert!(has(&l[2], "Debug", Token::Type));
        assert!(has(&l[3], "struct", Token::Keyword));
    }

    #[test]
    fn multiline_block_comment_is_split_per_line_and_clipped_before_newline() {
        let src = "a /* one\ntwo\nthree */ b\n";
        let mut h = SyntaxHighlighter::new();
        let lines = h.highlight(Language::Rust, src).unwrap();
        assert_eq!(lines.len(), 4);
        for (i, text) in src.split('\n').enumerate().take(3) {
            let c: Vec<_> = lines[i]
                .iter()
                .filter(|s| s.token == Token::Comment)
                .collect();
            assert_eq!(c.len(), 1, "line {i}");
            assert!(
                c[0].range.end <= text.len(),
                "span must not include the newline"
            );
        }
        assert_eq!(&"two"[lines[1][0].range.clone()], "two");
    }

    #[test]
    fn unicode_and_emoji_spans_land_on_char_boundaries() {
        let src = "let s = \"héllo 日本語 🌍\"; // ünï\n";
        let mut h = SyntaxHighlighter::new();
        let lines = h.highlight(Language::Rust, src).unwrap();
        let text = src.trim_end_matches('\n');
        for s in &lines[0] {
            assert!(text.is_char_boundary(s.range.start) && text.is_char_boundary(s.range.end));
        }
        let string = lines[0].iter().find(|s| s.token == Token::String).unwrap();
        assert_eq!(&text[string.range.clone()], "\"héllo 日本語 🌍\"");
    }

    #[test]
    fn crlf_line_endings_do_not_leak_the_cr_into_spans() {
        let src = "// a\r\nfn x() {}\r\n";
        let mut h = SyntaxHighlighter::new();
        let lines = h.highlight(Language::Rust, src).unwrap();
        let first = "// a";
        let c = lines[0].iter().find(|s| s.token == Token::Comment).unwrap();
        assert_eq!(&first[c.range.clone()], "// a", "CR excluded");
        assert!(lines[1].iter().any(|s| s.token == Token::Keyword));
    }

    #[test]
    fn javascript_and_jsx() {
        let l = spans(
            Language::JavaScript,
            "import a from 'x';\nconst f = (n) => `t ${n}`; // c\nclass A { static x = 1 }\nconst e = <div className=\"x\">hi</div>;\n",
        );
        assert!(has(&l[0], "import", Token::Keyword));
        assert!(has(&l[0], "'x'", Token::String));
        assert!(has(&l[1], "const", Token::Keyword));
        assert!(has(&l[1], "// c", Token::Comment));
        assert!(has(&l[2], "class", Token::Keyword));
        assert!(
            l[3].iter().any(|(t, k)| t == "div" && *k == Token::Tag),
            "{:?}",
            l[3]
        );
    }

    #[test]
    fn typescript_needs_both_js_and_ts_rules() {
        let l = spans(
            Language::TypeScript,
            "interface P { a: number }\nfunction f<T>(x: T): T { return x }\n",
        );
        assert!(has(&l[0], "interface", Token::Keyword));
        assert!(has(&l[0], "number", Token::Type));
        assert!(
            has(&l[1], "function", Token::Keyword),
            "JS keywords come from the JS query: {:?}",
            l[1]
        );
        assert!(has(&l[1], "return", Token::Keyword));
    }

    #[test]
    fn tsx_highlights_jsx_and_types() {
        let l = spans(
            Language::Tsx,
            "const A = (p: {n: number}) => <div className=\"x\">{p.n}</div>;\n",
        );
        assert!(has(&l[0], "const", Token::Keyword));
        assert!(has(&l[0], "number", Token::Type));
        assert!(
            l[0].iter().any(|(t, k)| t == "div" && *k == Token::Tag),
            "{:?}",
            l[0]
        );
    }

    #[test]
    fn go_json_dart_markdown() {
        let go = spans(
            Language::Go,
            "package main\n// c\nfunc main() { x := `raw`; _ = 42 }\n",
        );
        assert!(has(&go[0], "package", Token::Keyword));
        assert!(has(&go[1], "// c", Token::Comment));
        assert!(has(&go[2], "func", Token::Keyword));
        assert!(has(&go[2], "`raw`", Token::String));
        assert!(has(&go[2], "42", Token::Number));

        let json = spans(Language::Json, "{\"a\": [1, true, null], \"b\": \"s\"}\n");
        assert!(has(&json[0], "\"s\"", Token::String));
        assert!(has(&json[0], "1", Token::Number));
        assert!(has(&json[0], "true", Token::Constant));

        let dart = spans(
            Language::Dart,
            "import 'package:flutter/material.dart';\nclass A extends StatelessWidget {\n  @override\n  int n = 14;\n}\n",
        );
        assert!(has(&dart[0], "import", Token::Keyword));
        assert!(dart[0].iter().any(|(_, k)| *k == Token::String));
        assert!(has(&dart[1], "class", Token::Keyword));
        assert!(has(&dart[1], "StatelessWidget", Token::Type));
        assert!(has(&dart[2], "override", Token::Attribute));
        assert!(has(&dart[3], "14", Token::Number));

        let md = spans(
            Language::Markdown,
            "# Title\n\ntext\n\n```rust\nfn x() {}\n```\n",
        );
        assert!(has(&md[0], "Title", Token::Heading));
        assert!(md[5].iter().any(|(_, k)| *k == Token::Code), "{:?}", md[5]);
    }

    #[test]
    fn unknown_language_oversized_and_empty_inputs() {
        assert_eq!(Language::from_path(Path::new("a.txt")), None);
        assert_eq!(Language::from_path(Path::new("Makefile")), None);
        assert_eq!(Language::from_path(Path::new("X.RS")), Some(Language::Rust));
        assert_eq!(
            Language::from_path(Path::new("a.test.tsx")),
            Some(Language::Tsx)
        );
        let mut h = SyntaxHighlighter::new();
        assert!(h
            .highlight(Language::Rust, &"a".repeat(MAX_BYTES + 1))
            .is_none());
        let empty = h.highlight(Language::Rust, "").unwrap();
        assert_eq!(empty, vec![Vec::<LineSpan>::new()]);
        // broken code still yields spans (tree-sitter is error tolerant) and never panics
        let broken = h
            .highlight(Language::Rust, "fn ( { let \"unterminated\n")
            .unwrap();
        assert!(broken.iter().flatten().any(|s| s.token == Token::Keyword));
    }

    #[test]
    fn spans_never_overlap_within_a_line_and_stay_in_bounds() {
        let src = "fn a() { let s = \"x\"; /* c */ }\nlet t = 'a;\n";
        let mut h = SyntaxHighlighter::new();
        let lines = h.highlight(Language::Rust, src).unwrap();
        for (text, spans) in src.split('\n').zip(&lines) {
            let mut last = 0;
            let mut sorted = spans.clone();
            sorted.sort_by_key(|s| s.range.start);
            for s in sorted {
                assert!(s.range.start >= last, "overlap in {text:?}");
                assert!(s.range.end <= text.len());
                last = s.range.end;
            }
        }
    }
}
