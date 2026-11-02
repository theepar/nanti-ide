# ADR 0001: syntax highlighting with tree-sitter

Status: accepted (2026-11-02). Evidence: spike run + `crates/syntax` tests.

## Context

PRD 4.3 lists tree-sitter as a "kandidat kuat" and requires a feasibility check before commitment. The
alternative considered was a hand-written lexer per language (no native dependencies, but weaker accuracy
and a lot of language-specific code for Dart/TSX/Markdown).

## Spike (throw-away crate, not in the repo)

- `tree-sitter-highlight 0.27` with grammars for Rust 0.24, JavaScript 0.25, TypeScript/TSX 0.23, Go 0.25,
  JSON 0.24, Dart 0.3 and Markdown 0.5.
- Full cold build including the C grammars with the MSVC toolchain: about 20 s.
- Each grammar produced sensible captures on sample code. A 2340-line / 43 KB Rust file re-highlighted
  from scratch in 13 ms (release).
- Finding: the TypeScript/TSX highlight query only adds TS-specific rules. The JavaScript query (and the JSX
  query for TSX) must be concatenated in front of it, otherwise `function`, `return`, `const` are not
  colored.
- Finding: Markdown is block-level only (inline emphasis needs the inline grammar plus injections).
- Finding: JSON highlights keys and values the same way (no key/value distinction in the shipped query).

## Decision

Use tree-sitter through the `syntax` crate (kept separate so `editor-core` stays pure Rust). The
editor consumes per-line byte spans and never sees tree-sitter types. Per-file results are cached by buffer
version; files above 1.5 MB are left unhighlighted; dev builds compile the parser crates with `opt-level = 3`.

## Consequences

- The workspace now needs a C compiler (already required for GPUI's Windows toolchain: MSVC).
- Highlighting is syntactic only. Semantic highlighting would need LSP semantic tokens (ED-10).
- Each edit re-highlights the whole file. If this becomes a problem on large files, use tree-sitter's
  incremental `Tree::edit` (needs the lower-level `tree-sitter` API instead of `tree-sitter-highlight`).
- Markdown inline styling and JSON key coloring are known gaps, listed in the status table.
