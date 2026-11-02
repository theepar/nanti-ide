# M0 status (2026-11-02)

Evidence over claims: only items with a run result are PASS.

| Item | Status | Evidence |
| --- | --- | --- |
| Rust workspace, MSVC toolchain pinned | PASS | `cargo clippy -D warnings`, `cargo test` |
| `editor-core` buffer (UTF-8, CRLF, undo/redo, dirty) | PASS (unit) | 4 tests; ED-04 partial, not yet wired to UI input |
| Rust ↔ Node IPC (handshake, ping, notifications, killed host) | PASS | `crates/app/tests/host_ipc.rs` |
| Gate A: GPUI window opens on Windows | PASS | `ui-spike` / `nanti-ui` run, window handle present, rendered (captured own window only) |
| Gate A: IME, clipboard, resize in GPUI text input | NOT_TESTED | `ui-spike` (adapted Zed `input` example) builds and opens; needs manual typing test with an IME |
| Gate A: accessibility (screen reader) | NOT_TESTED | |
| Gate A: WebView2 child of GPUI window + JS↔Rust bridge | PASS (spike) | `ui-spike --bin webview`: JS→Rust→JS→Rust roundtrip logged |
| Gate A: webview layout follows window resize / focus / z-order with GPUI | NOT_TESTED | bounds are fixed in the spike |
| Gate B: mock extension load/activate/deactivate | PASS | Node + Rust e2e tests (VH-01 subset) |
| Gate B: `vscode.commands`, `workspace.fs`, `workspace.workspaceFolders`, `window.show*Message`, output channel | PASS (subset) | Node tests; VH-02, VH-03 partial, VH-05 partial |
| Unsupported API returns `UnsupportedCapability` (no fake success) | PASS | Node + Rust tests |
| VS Code-style workbench layout (`crates/ui`) | PASS (static) | screenshot reviewed; no input, no real file tree, data hard-coded |
| `WebviewViewProvider` shim, URI handler, `SecretStorage` | NOT_TESTED | next, needed for Claude/Codex (VH-06..08) |

## Gate C (Claude Code VSIX)

| Item | Status | Evidence |
| --- | --- | --- |
| AI-07 download + checksum + safe-package check | PASS | SHA-256 matches Open VSX; 49 entries, no traversal. See [api-compatibility-matrix.md](api-compatibility-matrix.md) |
| AI-07 `activated` | FAIL | Probe ran with user approval: strict fails at module load on `vscode.NotebookCellOutputItem.error`; discover mode enters `activate()` then fails on missing `context.globalStorageUri`. Details in the matrix |
| Probe tooling | READY | `scripts/probe-extension.mjs` (strict mode, plus `--discover` inventory mode) |
| Interop regression test (own-key unsupported names) | PASS | `extension-host/tests/host.test.mjs` |
| Slice 1: `ExtensionContext`, `Uri`, `EventEmitter`, `ExtensionMode`, `env` constants, log output channel | PASS (unit, 13/13 Node tests) | Claude Code now loads and enters `activate()`; strict fails next at `workspace.getConfiguration` |
| AI-07 `activated` after slice 1 | FAIL | see matrix "After slice 1" |
| Slice 2: configuration (VH-09), tab groups, doc/editor state+events, providers registry, `openTextDocument` (disk-backed) | PASS (unit, 15/15 Node tests) | limitations listed in the matrix (state static, disk-backed docs) |
| AI-07 `activated` after slice 2 | FAIL | strict fails at `window.registerWebviewViewProvider` (VH-06) |
| Slice 3 host: webview views/panels, URI handler, watcher, status bar item, `extensions`, `setContext` | PASS (unit, 22/22 Node tests) | |
| Rust host client async (reader thread, timeout, closed detection) | PASS | `crates/app/tests/host_ipc.rs` 5/5 incl. webview+URI e2e |
| `nanti-resource` protocol: root enforcement, traversal denied | PASS | `crates/ui` 6 unit tests + live `fetch` of a file outside the root returned 403 in WebView2 |
| Mock webview extension in real WebView2 (HTML, `acquireVsCodeApi`, two-way messages) | PASS | `NANTI_SELFTEST_LOG` run: `PASS webview roundtrip + resource root enforcement` |
| WebView2 child follows window resize | PASS | child HWND rect read before/after resize (x=961,h=748 @1280x800 → x=665,h=549 @984x601). Other DPI scales NOT_TESTED |
| AI-07 Claude Code `activated` | PASS (in-process strict probe) | matrix "After slice 3" |
| AI-08 Claude Code panel opens, not blank | PASS (login screen rendered in `nanti-ui`) | DOM summary in self-test log; reload/restart NOT_TESTED |
| AI-09 login, AI-10..13 | NOT_TESTED | need `env.openExternal` with consent UI, terminals, editor bridge |
| IDE UI interactivity (click handlers) | NOT_IMPLEMENTED | workbench is a static mock-up; first interactive slice in progress |

## Dev environment (this machine)

- rustc/cargo 1.99.1, `stable-x86_64-pc-windows-msvc` (global default is `-gnu`; `rust-toolchain.toml` overrides per repo — GPUI needs MSVC)
- VS 18 Community, MSVC 14.51, Windows SDK 10.0.26100
- Node 26.12.1, Go 1.27.0, Flutter at `C:\flutter`, `adb` on PATH
- WebView2 runtime 156.0.4314.58
- Pinned: `gpui =0.2.2`, `wry =0.57.1`

## Next exact blocker

1. Make the IDE UI interactive (M1): real explorer, clickable tabs/activity bar/panel tabs, open file into the editor. Today nothing in `crates/ui` handles clicks.
2. Gate C continued: `env.openExternal` behind a user-consent prompt (login, AI-09), terminal API (AI-12), editor/document bridge so extensions see IDE buffers instead of disk (AI-04/05).
3. Decide the `vscode.version` ADR (see matrix).
