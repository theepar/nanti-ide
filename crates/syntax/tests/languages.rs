use std::path::Path;

use syntax::{Language, SyntaxHighlighter, Token};

fn kinds(lang: Language, src: &str) -> Vec<Token> {
    let mut h = SyntaxHighlighter::new();
    let lines = h
        .highlight(lang, src)
        .unwrap_or_else(|| panic!("{lang:?} did not load"));
    lines.into_iter().flatten().map(|s| s.token).collect()
}

const SAMPLES: &[(Language, &str, &[Token])] = &[
    (
        Language::Python,
        "def f(x):\n    return \"hi\"  # note\n",
        &[Token::Keyword, Token::String, Token::Comment],
    ),
    (
        Language::C,
        "#include <stdio.h>\nint main(void) { return 0; } // note\n",
        &[Token::Comment],
    ),
    (
        Language::Cpp,
        "class A { public: int x = 1; };\n// note\nstd::string s = \"x\";\n",
        &[Token::Keyword, Token::Comment, Token::String],
    ),
    (
        Language::CSharp,
        "public class A { string s = \"x\"; } // note\n",
        &[Token::Keyword, Token::String, Token::Comment],
    ),
    (
        Language::Java,
        "public class A { String s = \"x\"; } // note\n",
        &[Token::Keyword, Token::String, Token::Comment],
    ),
    (
        Language::Ruby,
        "def f(x)\n  puts \"hi\" # note\nend\n",
        &[Token::Keyword, Token::String, Token::Comment],
    ),
    (
        Language::Php,
        "<?php\nfunction f() { return \"x\"; } // note\n",
        &[Token::Keyword, Token::String, Token::Comment],
    ),
    (
        Language::Html,
        "<div class=\"a\">text</div>\n<!-- note -->\n",
        &[Token::Tag, Token::Comment],
    ),
    (
        Language::Css,
        "a { color: red; } /* note */\n",
        &[Token::Comment],
    ),
    (
        Language::Yaml,
        "key: \"value\" # note\nlist:\n  - 1\n",
        &[Token::String, Token::Comment],
    ),
    (
        Language::Toml,
        "[table]\nk = \"v\" # note\n",
        &[Token::String, Token::Comment],
    ),
    (
        Language::Bash,
        "echo \"hi\" # note\nif true; then ls; fi\n",
        &[Token::String, Token::Comment, Token::Keyword],
    ),
    (
        Language::PowerShell,
        "function F { Write-Host \"x\" } # note\n",
        &[Token::Keyword, Token::String, Token::Comment],
    ),
    (
        Language::Lua,
        "local x = \"hi\" -- note\nfunction f() end\n",
        &[Token::Keyword, Token::String, Token::Comment],
    ),
    (
        Language::Sql,
        "SELECT a FROM t WHERE b = 'x'; -- note\n",
        &[Token::Keyword, Token::String, Token::Comment],
    ),
    (
        Language::Xml,
        "<a b=\"c\">text</a>\n<!-- note -->\n",
        &[Token::Comment],
    ),
    (
        Language::Make,
        "# note\nall: dep\n\techo hi\n",
        &[Token::Comment],
    ),
    (
        Language::CMake,
        "project(x)\nset(A \"b\") # note\n",
        &[Token::Comment],
    ),
    (
        Language::Batch,
        "@echo off\nREM note\necho hi\n",
        &[Token::Comment],
    ),
    (Language::Ini, "[s]\n; note\nk=v\n", &[Token::Comment]),
    (Language::Properties, "k=v\n# note\n", &[Token::Comment]),
    (Language::Diff, "--- a\n+++ b\n@@ -1 +1 @@\n-x\n+y\n", &[]),
    (
        Language::R,
        "f <- function(x) { \"s\" } # note\n",
        &[Token::String, Token::Comment],
    ),
    (
        Language::Zig,
        "const x = \"hi\"; // note\nfn f() void {}\n",
        &[Token::String, Token::Comment],
    ),
    (
        Language::Elixir,
        "defmodule A do\n  def f, do: \"x\" # note\nend\n",
        &[Token::String, Token::Comment],
    ),
    (
        Language::Erlang,
        "-module(a).\nf() -> \"x\". % note\n",
        &[Token::String, Token::Comment],
    ),
    (
        Language::Nix,
        "{ a = \"x\"; # note\n}\n",
        &[Token::String, Token::Comment],
    ),
];

#[test]
fn every_added_language_loads_and_highlights_its_basics() {
    let mut failures = Vec::new();
    for (lang, src, expected) in SAMPLES {
        let found = kinds(*lang, src);
        assert!(
            !found.is_empty() || expected.is_empty(),
            "{lang:?}: nothing highlighted"
        );
        for want in *expected {
            if !found.contains(want) {
                failures.push(format!("{lang:?} is missing {want:?} (got {:?})", {
                    let mut seen = found.clone();
                    seen.dedup();
                    seen
                }));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn every_language_in_the_table_has_a_sample_or_is_an_original_one() {
    let original = [
        Language::Rust,
        Language::JavaScript,
        Language::TypeScript,
        Language::Tsx,
        Language::Go,
        Language::Json,
        Language::Dart,
        Language::Markdown,
    ];
    for lang in Language::ALL {
        assert!(
            original.contains(&lang) || SAMPLES.iter().any(|(l, _, _)| *l == lang),
            "{lang:?} has no sample"
        );
        assert!(!lang.display_name().is_empty());
    }
    assert_eq!(Language::ALL.len(), 8 + SAMPLES.len());
}

#[test]
fn files_map_to_languages_by_extension_and_by_name() {
    let cases = [
        ("a/script.py", Language::Python),
        ("main.c", Language::C),
        ("main.h", Language::C),
        ("x.cpp", Language::Cpp),
        ("x.hpp", Language::Cpp),
        ("Program.cs", Language::CSharp),
        ("A.java", Language::Java),
        ("a.rb", Language::Ruby),
        ("Gemfile", Language::Ruby),
        ("index.php", Language::Php),
        ("index.html", Language::Html),
        ("site.css", Language::Css),
        ("ci.yml", Language::Yaml),
        ("Cargo.toml", Language::Toml),
        ("Cargo.lock", Language::Toml),
        ("run.sh", Language::Bash),
        (".bashrc", Language::Bash),
        ("x.ps1", Language::PowerShell),
        ("x.lua", Language::Lua),
        ("q.sql", Language::Sql),
        ("a.xml", Language::Xml),
        ("logo.svg", Language::Xml),
        ("Makefile", Language::Make),
        ("rules.mk", Language::Make),
        ("CMakeLists.txt", Language::CMake),
        ("x.cmake", Language::CMake),
        ("run.bat", Language::Batch),
        ("a.ini", Language::Ini),
        ("a.properties", Language::Properties),
        ("fix.patch", Language::Diff),
        ("a.R", Language::R),
        ("a.zig", Language::Zig),
        ("a.exs", Language::Elixir),
        ("a.erl", Language::Erlang),
        ("a.nix", Language::Nix),
        ("lib.RS", Language::Rust),
    ];
    for (path, want) in cases {
        assert_eq!(Language::from_path(Path::new(path)), Some(want), "{path}");
    }
    for plain in [
        "notes.txt",
        "README",
        "archive.zip",
        "noextension",
        ".gitignore",
    ] {
        assert_eq!(Language::from_path(Path::new(plain)), None, "{plain}");
    }
}

#[test]
fn empty_and_broken_input_never_fails_for_any_language() {
    let mut h = SyntaxHighlighter::new();
    for lang in Language::ALL {
        assert!(h.highlight(lang, "").is_some(), "{lang:?} empty");
        let junk = "\u{0}((([[{{ \"unterminated 'x /* <a\n\n\t\u{1F980}\n";
        assert!(h.highlight(lang, junk).is_some(), "{lang:?} junk");
    }
}
