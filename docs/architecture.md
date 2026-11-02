# nanti. Workspace Architecture

`nanti.` is organized as a high-performance Rust monorepo (Cargo workspace) with dedicated, single-responsibility crates and an isolated Node.js extension runtime.

```
nanti-ide/
├── .github/workflows/       # Automated CI (cargo test, clippy, fmt, release)
├── assets/                  # Branding, icon sets, logos
├── crates/                  # Modular Rust workspace crates
│   ├── app/                 # Headless daemon / CLI runner & extension host test harness
│   ├── editor-core/         # High-performance text engine (Ropey, transactions, undo/redo, cursor)
│   ├── git/                 # Git integration & source control operations
│   ├── syntax/              # Tree-sitter parsers, syntax highlighting & 50+ language definitions
│   ├── terminal/            # Native terminal emulator integration (Alacritty PTY & VT100)
│   └── ui/                  # Native GPUI graphical workbench, splitters, layout, editor UI
├── docs/                    # Technical specs, architecture records & compliance
│   ├── decisions/           # Architecture Decision Records (ADRs)
│   ├── api-compatibility-matrix.md
│   ├── legal-compliance.md
│   ├── m0-status.md
│   └── PRD_Rust_Native_IDE_MVP_v1.0.md
├── extension-host/          # Node.js bridge for running VS Code extensions (Claude Code, Codex)
├── scripts/                 # Build automation and license generation tools
└── tests/fixtures/          # Mock extensions and integration test fixtures
```

## Crate Responsibilities

| Crate | Responsibility | Dependencies |
| :--- | :--- | :--- |
| **`editor-core`** | Text buffer manipulation, rope data structure, UTF-8/UTF-16 cursor offsets, transactions, undo/redo stacks, find/replace. | `ropey`, `serde` |
| **`syntax`** | Grammar loading, query caching, token-to-color mapping, 50+ programming language detection. | `tree-sitter`, `tree-sitter-highlight` |
| **`git`** | Safe git process execution, staging, unstaging, diff generation, branch switching. | Standard library |
| **`terminal`** | Embedded PTY management, VT100 escape sequence processing, raw terminal streams. | `alacritty_terminal` |
| **`ui`** | GPUI application shell: Activity bar, Explorer, Source Control, Extensions, Status bar, Terminal panel, Command palette (`Ctrl+Shift+P`). | `gpui`, `wry`, `editor-core`, `syntax`, `git`, `terminal` |
| **`app`** | Lightweight command-line binary launcher. | `editor-core`, `serde` |
| **`extension-host`** | Sandboxed Node.js environment implementing the VS Code extension host API. | Node.js |

## Key Design Principles
1. **Zero Unnecessary Dependencies**: Every external dependency is audited and catalogued in `THIRD_PARTY_NOTICES.md`.
2. **Native Performance**: Core editing and layout are written purely in Rust on top of GPUI, avoiding web tech overhead.
3. **Fail-Closed Isolation**: Extension host runs in an isolated process; slow extensions or network lags never freeze the editor UI thread.
