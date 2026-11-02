# nanti.

> Code now. Not later.

A lightweight, extensible code editor built natively in Rust.

---

## Identity

- **Product Name:** Nanti
- **Brand:** `nanti.`
- **Repository:** `nanti`
- **Executable:** `nanti.exe`
- **Website:** `nanti.dev`
- **Tagline:** *Code now. Not later.*

## Overview

`nanti.` adalah editor kode / IDE desktop berbasis Rust native untuk performa tinggi dan konsumsi memori hemat. Dilengkapi arsitektur hybrid kompatibilitas Open VSX / VS Code extension host (Node.js out-of-process) untuk mendukung integrasi AI modern seperti Codex dan Claude Code tanpa beban shell Electron.

## Development

Prerequisites: Windows 11 x64, Rust (MSVC toolchain, pinned by `rust-toolchain.toml`), Visual Studio C++ build tools + Windows SDK, Node.js >= 22, Git.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd extension-host && npm test
cargo run -p app    # spawns the Node extension host and pings it
cargo run -p ui     # workbench window
# show an extension webview in the right panel (set USERPROFILE/HOME/APPDATA/LOCALAPPDATA to a temp dir first
# when running third-party extensions; see docs/api-compatibility-matrix.md "Side effects")
#   NANTI_EXTENSION=<extension dir> NANTI_VIEW=<view type> [NANTI_WORKSPACE=<dir>] cargo run -p ui
cargo run -p ui-spike --bin webview   # WebView2 + GPUI bridge spike, logs to webview-spike.log
```

Status: see [docs/m0-status.md](docs/m0-status.md).

Lihat [PRD_Rust_Native_IDE_MVP_v1.0.md](PRD_Rust_Native_IDE_MVP_v1.0.md) untuk detail arsitektur teknis dan acceptance test.

