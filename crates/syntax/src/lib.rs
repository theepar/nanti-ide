use std::collections::HashMap;
use std::ops::Range;
use std::path::Path;

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

mod analysis;
pub use analysis::{diagnostics, outline, Diagnostic, Symbol};

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
    Python,
    C,
    Cpp,
    CSharp,
    Java,
    Ruby,
    Php,
    Html,
    Css,
    Yaml,
    Toml,
    Bash,
    PowerShell,
    Lua,
    Sql,
    Xml,
    Make,
    CMake,
    Batch,
    Ini,
    Properties,
    Diff,
    R,
    Zig,
    Elixir,
    Erlang,
    Nix,
}

impl Language {
    pub const ALL: [Language; 35] = [
        Language::Rust,
        Language::JavaScript,
        Language::TypeScript,
        Language::Tsx,
        Language::Go,
        Language::Json,
        Language::Dart,
        Language::Markdown,
        Language::Python,
        Language::C,
        Language::Cpp,
        Language::CSharp,
        Language::Java,
        Language::Ruby,
        Language::Php,
        Language::Html,
        Language::Css,
        Language::Yaml,
        Language::Toml,
        Language::Bash,
        Language::PowerShell,
        Language::Lua,
        Language::Sql,
        Language::Xml,
        Language::Make,
        Language::CMake,
        Language::Batch,
        Language::Ini,
        Language::Properties,
        Language::Diff,
        Language::R,
        Language::Zig,
        Language::Elixir,
        Language::Erlang,
        Language::Nix,
    ];

    pub fn from_path(path: &Path) -> Option<Self> {
        if let Some(lang) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(Self::from_file_name)
        {
            return Some(lang);
        }
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
            "py" | "pyw" | "pyi" => Language::Python,
            "c" | "h" => Language::C,
            "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" | "ino" => Language::Cpp,
            "cs" => Language::CSharp,
            "java" => Language::Java,
            "rb" | "rake" | "gemspec" => Language::Ruby,
            "php" | "phtml" => Language::Php,
            "html" | "htm" | "xhtml" => Language::Html,
            "css" => Language::Css,
            "yaml" | "yml" => Language::Yaml,
            "toml" => Language::Toml,
            "sh" | "bash" | "zsh" => Language::Bash,
            "ps1" | "psm1" | "psd1" => Language::PowerShell,
            "lua" => Language::Lua,
            "sql" => Language::Sql,
            "xml" | "xsl" | "xslt" | "svg" | "plist" | "csproj" | "props" | "targets" => {
                Language::Xml
            }
            "mk" | "mak" => Language::Make,
            "cmake" => Language::CMake,
            "bat" | "cmd" => Language::Batch,
            "ini" | "cfg" => Language::Ini,
            "properties" => Language::Properties,
            "diff" | "patch" => Language::Diff,
            "r" => Language::R,
            "zig" => Language::Zig,
            "ex" | "exs" => Language::Elixir,
            "erl" | "hrl" => Language::Erlang,
            "nix" => Language::Nix,
            _ => return None,
        })
    }

    fn from_file_name(name: &str) -> Option<Self> {
        Some(match name {
            "Makefile" | "makefile" | "GNUmakefile" => Language::Make,
            "CMakeLists.txt" => Language::CMake,
            "Gemfile" | "Rakefile" | "Guardfile" => Language::Ruby,
            ".bashrc" | ".bash_profile" | ".zshrc" | ".profile" => Language::Bash,
            "Cargo.lock" => Language::Toml,
            _ => return None,
        })
    }

    pub fn reports_syntax_errors(self) -> bool {
        matches!(
            self,
            Language::Rust
                | Language::JavaScript
                | Language::TypeScript
                | Language::Tsx
                | Language::Go
                | Language::Json
                | Language::Dart
                | Language::Python
                | Language::C
                | Language::Cpp
                | Language::CSharp
                | Language::Java
                | Language::Ruby
                | Language::Css
                | Language::Yaml
                | Language::Toml
                | Language::Lua
                | Language::Zig
        )
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
            Language::Tsx => "TypeScript React",
            Language::Go => "Go",
            Language::Json => "JSON",
            Language::Dart => "Dart",
            Language::Markdown => "Markdown",
            Language::Python => "Python",
            Language::C => "C",
            Language::Cpp => "C++",
            Language::CSharp => "C#",
            Language::Java => "Java",
            Language::Ruby => "Ruby",
            Language::Php => "PHP",
            Language::Html => "HTML",
            Language::Css => "CSS",
            Language::Yaml => "YAML",
            Language::Toml => "TOML",
            Language::Bash => "Shell Script",
            Language::PowerShell => "PowerShell",
            Language::Lua => "Lua",
            Language::Sql => "SQL",
            Language::Xml => "XML",
            Language::Make => "Makefile",
            Language::CMake => "CMake",
            Language::Batch => "Batch",
            Language::Ini => "INI",
            Language::Properties => "Properties",
            Language::Diff => "Diff",
            Language::R => "R",
            Language::Zig => "Zig",
            Language::Elixir => "Elixir",
            Language::Erlang => "Erlang",
            Language::Nix => "Nix",
        }
    }
}

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
    pub range: Range<usize>,
    pub token: Token,
}

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

fn build_first(
    language: tree_sitter::Language,
    name: &str,
    queries: &[&str],
) -> Option<HighlightConfiguration> {
    queries
        .iter()
        .find_map(|q| build(language.clone(), name, q, "", ""))
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
        Language::Python => build(
            tree_sitter_python::LANGUAGE.into(),
            "python",
            tree_sitter_python::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::C => build(
            tree_sitter_c::LANGUAGE.into(),
            "c",
            tree_sitter_c::HIGHLIGHT_QUERY,
            "",
            "",
        ),
        Language::Cpp => build(
            tree_sitter_cpp::LANGUAGE.into(),
            "cpp",
            &format!(
                "{}\n{}",
                tree_sitter_c::HIGHLIGHT_QUERY,
                tree_sitter_cpp::HIGHLIGHT_QUERY
            ),
            "",
            "",
        )
        .or_else(|| {
            build(
                tree_sitter_cpp::LANGUAGE.into(),
                "cpp",
                tree_sitter_cpp::HIGHLIGHT_QUERY,
                "",
                "",
            )
        }),
        Language::CSharp => build(
            tree_sitter_c_sharp::LANGUAGE.into(),
            "csharp",
            tree_sitter_c_sharp::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Java => build(
            tree_sitter_java::LANGUAGE.into(),
            "java",
            tree_sitter_java::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Ruby => build(
            tree_sitter_ruby::LANGUAGE.into(),
            "ruby",
            tree_sitter_ruby::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Php => build(
            tree_sitter_php::LANGUAGE_PHP.into(),
            "php",
            tree_sitter_php::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Html => build(
            tree_sitter_html::LANGUAGE.into(),
            "html",
            tree_sitter_html::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Css => build(
            tree_sitter_css::LANGUAGE.into(),
            "css",
            tree_sitter_css::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Yaml => build(
            tree_sitter_yaml::LANGUAGE.into(),
            "yaml",
            tree_sitter_yaml::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Toml => build(
            tree_sitter_toml_ng::LANGUAGE.into(),
            "toml",
            tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Bash => build(
            tree_sitter_bash::LANGUAGE.into(),
            "bash",
            tree_sitter_bash::HIGHLIGHT_QUERY,
            "",
            "",
        ),
        Language::PowerShell => build(
            tree_sitter_powershell::LANGUAGE.into(),
            "powershell",
            tree_sitter_powershell::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Lua => build(
            tree_sitter_lua::LANGUAGE.into(),
            "lua",
            tree_sitter_lua::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Sql => build_first(
            tree_sitter_sequel::LANGUAGE.into(),
            "sql",
            &[
                &format!(
                    "{}
(comment) @comment",
                    tree_sitter_sequel::HIGHLIGHTS_QUERY
                ),
                tree_sitter_sequel::HIGHLIGHTS_QUERY,
            ],
        ),
        Language::Xml => build(
            tree_sitter_xml::LANGUAGE_XML.into(),
            "xml",
            tree_sitter_xml::XML_HIGHLIGHT_QUERY,
            "",
            "",
        ),
        Language::Make => build_first(
            tree_sitter_make::LANGUAGE.into(),
            "make",
            &[
                &format!(
                    "{}
(comment) @comment",
                    tree_sitter_make::HIGHLIGHTS_QUERY
                ),
                tree_sitter_make::HIGHLIGHTS_QUERY,
            ],
        ),
        Language::CMake => build_first(
            tree_sitter_cmake::LANGUAGE.into(),
            "cmake",
            &[
                &format!(
                    "{}
[(line_comment) (bracket_comment)] @comment",
                    tree_sitter_cmake::HIGHLIGHTS_QUERY
                ),
                tree_sitter_cmake::HIGHLIGHTS_QUERY,
            ],
        ),
        Language::Batch => build(
            tree_sitter_batch::LANGUAGE.into(),
            "batch",
            tree_sitter_batch::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Ini => build_first(
            tree_sitter_ini::LANGUAGE.into(),
            "ini",
            &[
                &format!(
                    "{}
(comment) @comment",
                    tree_sitter_ini::HIGHLIGHTS_QUERY
                ),
                tree_sitter_ini::HIGHLIGHTS_QUERY,
            ],
        ),
        Language::Properties => build(
            tree_sitter_properties::LANGUAGE.into(),
            "properties",
            tree_sitter_properties::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Diff => build(
            tree_sitter_diff::LANGUAGE.into(),
            "diff",
            tree_sitter_diff::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::R => build(
            tree_sitter_r::LANGUAGE.into(),
            "r",
            tree_sitter_r::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Zig => build(
            tree_sitter_zig::LANGUAGE.into(),
            "zig",
            tree_sitter_zig::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Elixir => build(
            tree_sitter_elixir::LANGUAGE.into(),
            "elixir",
            tree_sitter_elixir::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Erlang => build(
            tree_sitter_erlang::LANGUAGE.into(),
            "erlang",
            tree_sitter_erlang::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
        Language::Nix => build(
            tree_sitter_nix::LANGUAGE.into(),
            "nix",
            tree_sitter_nix::HIGHLIGHTS_QUERY,
            "",
            "",
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

#[derive(Default)]
pub struct SyntaxHighlighter {
    configs: HashMap<Language, Option<HighlightConfiguration>>,
    highlighter: Highlighter,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self::default()
    }

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
            text_end -= 1;
        }
        let seg_end = range.end.min(text_end);
        if seg_end > pos {
            lines[line].push(LineSpan {
                range: pos - start..seg_end - start,
                token,
            });
        }
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
        assert_eq!(
            Language::from_path(Path::new("Makefile")),
            Some(Language::Make)
        );
        assert_eq!(Language::from_path(Path::new("LICENSE")), None);
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
