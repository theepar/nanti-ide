//! Syntax-level analysis on top of the tree-sitter parse: syntax problems (ED-10, syntax only: this is
//! NOT a type checker or a language server) and a symbol outline.

use tree_sitter::{Node, Parser};

use crate::{Language, MAX_BYTES};

/// At most this many problems are reported per file (a broken file can produce thousands).
pub const MAX_DIAGNOSTICS: usize = 200;
/// At most this many symbols are listed per file.
pub const MAX_SYMBOLS: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// 0-based line and char column where the problem starts.
    pub line: usize,
    pub col: usize,
    /// Number of chars highlighted on that line (at least 1).
    pub len: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    /// Short kind label: `fn`, `struct`, `class`, `h2`, ...
    pub kind: &'static str,
    pub line: usize,
    /// Nesting level (methods inside an `impl`/class are 1).
    pub depth: usize,
}

fn ts_language(lang: Language) -> tree_sitter::Language {
    match lang {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::Json => tree_sitter_json::LANGUAGE.into(),
        Language::Dart => tree_sitter_dart::LANGUAGE.into(),
        Language::Markdown => tree_sitter_md::LANGUAGE.into(),
    }
}

fn parse(lang: Language, source: &str) -> Option<tree_sitter::Tree> {
    if source.len() > MAX_BYTES {
        return None;
    }
    let mut parser = Parser::new();
    parser.set_language(&ts_language(lang)).ok()?;
    parser.parse(source, None)
}

/// Position of byte offset `byte` as (0-based line, char column).
fn position(source: &str, line_starts: &[usize], byte: usize) -> (usize, usize) {
    let line = line_starts.partition_point(|&s| s <= byte) - 1;
    let col = source[line_starts[line]..byte.min(source.len())]
        .chars()
        .count();
    (line, col)
}

fn line_starts(source: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(source.match_indices('\n').map(|(i, _)| i + 1))
        .collect()
}

/// Syntax errors of `source`: tree-sitter ERROR nodes and tokens the grammar expected but did not find.
/// Empty when the file parses cleanly or is above `MAX_BYTES`.
pub fn diagnostics(lang: Language, source: &str) -> Vec<Diagnostic> {
    let Some(tree) = parse(lang, source) else {
        return Vec::new();
    };
    let root = tree.root_node();
    if !root.has_error() {
        return Vec::new();
    }
    let starts = line_starts(source);
    let mut out = Vec::new();
    collect_errors(root, source, &starts, &mut out);
    out.sort_by_key(|d| (d.line, d.col));
    out.truncate(MAX_DIAGNOSTICS);
    out
}

fn collect_errors(node: Node, source: &str, starts: &[usize], out: &mut Vec<Diagnostic>) {
    if out.len() >= MAX_DIAGNOSTICS {
        return;
    }
    if node.is_missing() || node.is_error() {
        let (line, col) = position(source, starts, node.start_byte());
        let end = node
            .end_byte()
            .min(starts.get(line + 1).map_or(source.len(), |&n| n - 1));
        let len = source[node.start_byte().min(end)..end]
            .chars()
            .count()
            .max(1);
        let message = if node.is_missing() {
            format!("Missing {}", node.kind().trim_matches('"'))
        } else {
            "Syntax error".to_owned()
        };
        out.push(Diagnostic {
            line,
            col,
            len,
            message,
        });
        if node.is_error() {
            return; // the children of an error node are noise; report the region once
        }
    }
    if !node.has_error() && !node.is_missing() {
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_errors(child, source, starts, out);
    }
}

/// Symbols (functions, types, classes, headings) of `source` in document order.
/// Supported: Rust, JavaScript/TypeScript/TSX, Go, Markdown headings. Other languages return nothing.
pub fn outline(lang: Language, source: &str) -> Vec<Symbol> {
    if !matches!(
        lang,
        Language::Rust
            | Language::JavaScript
            | Language::TypeScript
            | Language::Tsx
            | Language::Go
            | Language::Markdown
    ) {
        return Vec::new();
    }
    let Some(tree) = parse(lang, source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    walk_symbols(tree.root_node(), lang, source, 0, &mut out);
    out
}

/// (kind label, is a container whose children are nested one level deeper)
fn symbol_kind(lang: Language, node_kind: &str) -> Option<(&'static str, bool)> {
    Some(match (lang, node_kind) {
        (Language::Rust, "function_item") => ("fn", false),
        (Language::Rust, "struct_item") => ("struct", false),
        (Language::Rust, "enum_item") => ("enum", false),
        (Language::Rust, "trait_item") => ("trait", true),
        (Language::Rust, "impl_item") => ("impl", true),
        (Language::Rust, "mod_item") => ("mod", true),
        (Language::Rust, "const_item") => ("const", false),
        (Language::Rust, "type_item") => ("type", false),
        (Language::JavaScript | Language::TypeScript | Language::Tsx, "function_declaration") => {
            ("function", false)
        }
        (Language::JavaScript | Language::TypeScript | Language::Tsx, "class_declaration") => {
            ("class", true)
        }
        (Language::JavaScript | Language::TypeScript | Language::Tsx, "method_definition") => {
            ("method", false)
        }
        (Language::TypeScript | Language::Tsx, "interface_declaration") => ("interface", false),
        (Language::TypeScript | Language::Tsx, "type_alias_declaration") => ("type", false),
        (Language::TypeScript | Language::Tsx, "enum_declaration") => ("enum", false),
        (Language::Go, "function_declaration") => ("func", false),
        (Language::Go, "method_declaration") => ("method", false),
        (Language::Go, "type_spec") => ("type", false),
        (Language::Markdown, "atx_heading") => ("h", false),
        _ => return None,
    })
}

fn symbol_name(node: Node, lang: Language, source: &str) -> Option<String> {
    let text = |n: Node| source.get(n.byte_range()).map(str::to_owned);
    if lang == Language::Markdown {
        let content = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "inline")?;
        return text(content).map(|t| t.trim().to_owned());
    }
    let field = if node.kind() == "impl_item" {
        "type"
    } else {
        "name"
    };
    node.child_by_field_name(field).and_then(text)
}

fn walk_symbols(node: Node, lang: Language, source: &str, depth: usize, out: &mut Vec<Symbol>) {
    if out.len() >= MAX_SYMBOLS {
        return;
    }
    let mut child_depth = depth;
    if let Some((kind, container)) = symbol_kind(lang, node.kind()) {
        if let Some(name) = symbol_name(node, lang, source).filter(|n| !n.is_empty()) {
            let kind = if kind == "h" {
                // heading level = number of leading '#'
                let hashes = source[node.start_byte()..]
                    .chars()
                    .take_while(|c| *c == '#')
                    .count();
                ["h1", "h2", "h3", "h4", "h5", "h6"][hashes.clamp(1, 6) - 1]
            } else {
                kind
            };
            out.push(Symbol {
                name,
                kind,
                line: node.start_position().row,
                depth,
            });
            if container {
                child_depth += 1;
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_symbols(child, lang, source, child_depth, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_files_have_no_diagnostics() {
        assert!(diagnostics(Language::Rust, "fn main() { let x = 1; }\n").is_empty());
        assert!(diagnostics(Language::Json, "{\"a\": [1, 2, 3]}").is_empty());
        assert!(diagnostics(Language::Go, "package main\nfunc main() {}\n").is_empty());
    }

    #[test]
    fn broken_rust_reports_where_it_breaks() {
        let d = diagnostics(Language::Rust, "fn ok() {}\nfn broken( {\n    let = ;\n}\n");
        assert!(!d.is_empty());
        assert!(d.iter().all(|d| d.line >= 1), "line 0 is valid: {d:?}");
        assert!(
            d.iter()
                .any(|d| d.message.contains("error") || d.message.starts_with("Missing"))
        );
        assert!(d.iter().all(|d| d.len >= 1));
    }

    #[test]
    fn broken_json_and_js_are_reported() {
        assert!(!diagnostics(Language::Json, "{\"a\": ,}").is_empty());
        assert!(!diagnostics(Language::JavaScript, "function f( {\n").is_empty());
        assert!(!diagnostics(Language::TypeScript, "let x: = 1;\n").is_empty());
    }

    #[test]
    fn columns_are_in_chars_not_bytes() {
        let d = diagnostics(Language::Rust, "// ünï\nlet = 1;\n");
        assert!(d.iter().all(|d| d.line <= 1));
        let first = diagnostics(Language::JavaScript, "const ü = ;\n");
        assert!(first.iter().any(|d| d.col <= 10), "{first:?}");
    }

    #[test]
    fn diagnostics_are_capped_and_sorted() {
        let src = "fn a( {\n".repeat(500);
        let d = diagnostics(Language::Rust, &src);
        assert!(d.len() <= MAX_DIAGNOSTICS);
        assert!(
            d.windows(2)
                .all(|w| (w[0].line, w[0].col) <= (w[1].line, w[1].col))
        );
    }

    #[test]
    fn huge_files_are_skipped() {
        let src = "x".repeat(MAX_BYTES + 1);
        assert!(diagnostics(Language::Rust, &src).is_empty());
        assert!(outline(Language::Rust, &src).is_empty());
    }

    #[test]
    fn rust_outline_nests_methods_under_impl() {
        let src = "struct S;\nimpl S {\n    fn m(&self) {}\n}\nfn free() {}\nenum E { A }\nmod inner { fn deep() {} }\n";
        let o = outline(Language::Rust, src);
        let view: Vec<(&str, &str, usize, usize)> = o
            .iter()
            .map(|s| (s.kind, s.name.as_str(), s.line, s.depth))
            .collect();
        assert_eq!(
            view,
            [
                ("struct", "S", 0, 0),
                ("impl", "S", 1, 0),
                ("fn", "m", 2, 1),
                ("fn", "free", 4, 0),
                ("enum", "E", 5, 0),
                ("mod", "inner", 6, 0),
                ("fn", "deep", 6, 1)
            ]
        );
    }

    #[test]
    fn js_ts_go_and_markdown_outlines() {
        let js = outline(
            Language::JavaScript,
            "class A {\n  go() {}\n}\nfunction f() {}\n",
        );
        assert_eq!(
            js.iter()
                .map(|s| (s.kind, s.name.as_str(), s.depth))
                .collect::<Vec<_>>(),
            [("class", "A", 0), ("method", "go", 1), ("function", "f", 0)]
        );
        let ts = outline(
            Language::TypeScript,
            "interface I {}\ntype T = number;\nenum E { A }\n",
        );
        assert_eq!(
            ts.iter().map(|s| s.kind).collect::<Vec<_>>(),
            ["interface", "type", "enum"]
        );
        let go = outline(
            Language::Go,
            "package p\ntype T struct{}\nfunc (t T) M() {}\nfunc F() {}\n",
        );
        assert_eq!(
            go.iter()
                .map(|s| (s.kind, s.name.as_str()))
                .collect::<Vec<_>>(),
            [("type", "T"), ("method", "M"), ("func", "F")]
        );
        let md = outline(Language::Markdown, "# Title\ntext\n## Sub heading\n");
        assert_eq!(
            md.iter()
                .map(|s| (s.kind, s.name.as_str()))
                .collect::<Vec<_>>(),
            [("h1", "Title"), ("h2", "Sub heading")]
        );
        assert!(outline(Language::Json, "{}").is_empty());
    }
}
