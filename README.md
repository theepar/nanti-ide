<div align="center">
  <img src="assets/nanti-logo.png" width="128" height="128" alt="nanti. logo" style="border-radius: 28px;" />
  <h1>nanti.</h1>
  <p><strong>Code now. Not later.</strong></p>
  <p><em>An ultra-lightweight, zero-telemetry, local-first code editor built natively in Rust for Windows.</em></p>

  <p>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" /></a>
    <img src="https://img.shields.io/badge/privacy-100%25%20Zero%20Telemetry-success.svg" alt="Zero Telemetry" />
    <img src="https://img.shields.io/badge/footprint-~50MB%20RAM-brightgreen.svg" alt="~50MB RAM" />
    <img src="https://img.shields.io/badge/cold%20start-%3C100ms-blueviolet.svg" alt="<100ms Cold Start" />
    <img src="https://img.shields.io/badge/rust-2021%20edition-orange.svg" alt="Rust 2021" />
    <img src="https://img.shields.io/badge/platform-Windows%2010%20%2F%2011%20x64-0078d4.svg" alt="Platform: Windows" />
    <img src="https://img.shields.io/badge/graphics-GPUI%20(DirectX%20%2F%20Vulkan)-6366f1.svg" alt="Graphics: GPUI" />
  </p>
</div>

---

## Overview

**`nanti.`** is an ultra-fast, native desktop code editor and developer environment built from the ground up in Rust. Designed as an uncompromised, featherweight alternative to resource-heavy Electron editors, `nanti.` is centered on two fundamental engineering commitments: **Absolute Privacy (Zero Telemetry)** and **Featherweight Native Performance (< 100ms startup, ~50MB RAM)**.

---

## Why nanti.? (The Two Core Pillars)

### 🛡️ 1. Absolute Privacy & 100% Local-First
- **Strictly Zero Telemetry:** `nanti.` collects **no metrics, no telemetry pings, no usage analytics, and no crash beacons**. It never phones home.
- **Data Sovereignty:** Your source code, open files, keystrokes, Git credentials, and terminal command history **never leave your machine**.
- **Transparent & Auditable:** 100% open-source under the MIT license with clean-room architecture. Every network and IPC boundary is transparent and inspectable.
- **Explicit Consent Isolation:** Out-of-process extensions are sandboxed; any external URL navigation (`env.openExternal`) prompts the user for explicit confirmation before executing.

### 🍃 2. Featherweight Architecture (No Electron Bloat)
- **Instant Cold Start (< 100 ms):** Launches immediately upon invocation—no splash screens, no loading spinners, and no framework startup delays.
- **Tiny Memory Footprint (~40–70 MB RAM):** Idles at under 70 MB of RAM, leaving maximum system memory available for your compilers, Docker containers, and test suites.
- **Native GPU Rendering (60fps):** Directly driven by Zed's hardware-accelerated GPUI engine (DirectX/Vulkan on Windows). Smooth typing and scrolling with zero web-DOM layout overhead.
- **Zero CPU at Idle:** GPU repaints only occur when content is modified or animations are active. Conserves battery life on laptops.
- **Compact Standalone Binary:** The entire release executable is a compact single file (`~23 MB`).

---

## Performance & Privacy Comparison

| Metric / Dimension | `nanti.` | Typical Electron IDEs (VS Code, Cursor, Windsurf) |
|---|---|---|
| **Cold Startup Time** | **< 100 ms** (instantaneous) | 1.5 s – 5.0 s+ |
| **Idle RAM Usage** | **~40 – 70 MB** | 400 MB – 1.2 GB+ |
| **Telemetry & Tracking** | **0% (Zero Telemetry, Local-First)** | Enabled by default (device IDs, event analytics) |
| **Rendering Pipeline** | **Native GPUI (Direct GPU Text Layout)** | Chromium Blink DOM + Compositor Layers |
| **Binary / Install Size** | **~23 MB standalone executable** | 250 MB – 600 MB+ installed footprint |
| **Code Privacy** | **Air-gap capable; code stays local** | Often paired with proprietary cloud sync & telemetry |

---

## Architecture

```mermaid
graph TD
    subgraph Core ["nanti.exe (Rust / GPUI Core)"]
        UI["Workbench UI (GPUI)"]
        Editor["Editor Engine (Ropey Buffer)"]
        Syntax["Syntax Highlighter (Tree-sitter)"]
        Terminal["Terminal Panel (ConPTY)"]
        LSP["LSP Client (JSON-RPC 2.0)"]
        GitEngine["Git Engine (Background CLI)"]
    end

    subgraph ExtHost ["Extension Host (Node.js Out-of-Process)"]
        Host["Host Controller (host.mjs)"]
        Shim["VS Code API Shim (vscode-api.mjs)"]
        WebviewBridge["WebView2 / Wry Bridge"]
    end

    subgraph Servers ["Language Servers & Tools"]
        RustAnalyzer["rust-analyzer"]
        Gopls["gopls"]
        Shell["PowerShell / Command Prompt"]
    end

    UI --> Editor
    UI --> Syntax
    UI --> Terminal
    UI <--> |IPC JSON-RPC| ExtHost
    Terminal <--> |ConPTY| Shell
    LSP <--> |stdio| RustAnalyzer
    LSP <--> |stdio| Gopls
    UI --> GitEngine
```

---

## Key Features

### ⚡ Ultra-Fast Native Core
- Starts in under **100 ms** and uses **~40–70 MB RAM** at idle.
- UTF-8 and CRLF line ending preservation with piece-tree/rope data structures (`ropey`).
- Atomic file writes (temporary file + rename) with external modification conflict detection.

### 🧠 Language Intelligence & Syntax
- **Tree-sitter Grammars:** Built-in syntax highlighting for Rust, TypeScript, JavaScript, Go, JSON, Dart, and Markdown.
- **Live LSP Diagnostics:** Connected to native language servers (`rust-analyzer`, `gopls`) with real-time squiggle underlines and line-number error markers.
- **Problems Panel:** Dedicated tab aggregating syntax and semantic diagnostics across open files.

### 💻 ConPTY Integrated Terminal
- Full Windows ConPTY backend launching PowerShell or custom shells.
- Vertical side-tabs for switching between multiple concurrent terminal sessions.
- Process exit code tracking, clean interrupt (`Ctrl+C`), and IME composition pre-edit display.

### 🧩 Extension Compatibility & Open VSX
- Out-of-process Node.js runtime executing extension logic in the background.
- Direct search against the **Open VSX Marketplace** with one-click download and automated `.vsix` extraction.
- Local `.vsix` package installation via native Windows file dialogs.
- Sandboxed WebView2 webview panels for rich extension interfaces.

### 🔍 Search, Navigation & Command Palette
- **Fuzzy Quick Open (`Ctrl+P`):** Fast file indexing across repositories.
- **Command Palette (`Ctrl+Shift+P`):** Instant access to every IDE action.
- **Fast Find & Replace (`Ctrl+F` / `Ctrl+H`):** Regex support, match highlighting, and case/whole-word toggling.
- **Workspace Search (`Ctrl+Shift+F`):** Rapid multi-file regex search grouped by file.

### 🎨 Modern Dark Theme
- Tailored dark palette (`#181818` / `#1e1e1e`) aligned with VS Code Dark Modern.
- Subtle active line highlight (`rgba(255, 255, 255, 0.04)`) and highlighted active line numbers.
- Embedded SVG vector icons for activity bar, explorer items, and layout controls.

---

## Getting Started

### Prerequisites
- **Operating System:** Windows 10 / 11 (64-bit)
- **Rust Toolchain:** Stable MSVC (`stable-x86_64-pc-windows-msvc`), managed by `rust-toolchain.toml`
- **Build Tools:** Visual Studio C++ Build Tools & Windows SDK
- **Node.js:** Node.js >= 20 (for the out-of-process extension host)
- **Git:** Git for Windows (accessible on `PATH`)

### Building from Source

1. **Clone the repository:**
   ```sh
   git clone https://github.com/theepar/nanti-ide.git
   cd nanti-ide
   ```

2. **Verify tests and environment:**
   ```sh
   cargo test --workspace
   cd extension-host && npm test && cd ..
   ```

3. **Run in development mode:**
   ```sh
   cargo run -p ui
   ```

4. **Build optimized release binary:**
   ```sh
   cargo build --release -p ui
   ```
   The standalone executable will be located at `target/release/nanti.exe`.

---

## Keyboard Shortcuts

| Shortcut | Command | Action |
|---|---|---|
| `Ctrl+P` | **Quick Open** | Fuzzy search and open workspace files |
| `Ctrl+Shift+P` | **Command Palette** | Access all workbench commands |
| `Ctrl+S` | **Save** | Save active editor buffer |
| `Ctrl+Shift+S` | **Save All** | Save all modified buffers |
| `Ctrl+W` | **Close Tab** | Close active editor tab |
| `Ctrl+Shift+W` | **Close All Tabs** | Close all open tabs (prompts if dirty) |
| `Ctrl+F` | **Find** | Open find bar in active editor |
| `Ctrl+H` | **Replace** | Open find and replace bar |
| `Ctrl+Shift+F` | **Search in Files** | Open workspace search sidebar |
| `Ctrl+\`` | **Toggle Terminal** | Show / hide integrated terminal panel |
| `Ctrl+B` | **Toggle Sidebar** | Show / hide activity sidebar |
| `Ctrl+J` | **Toggle Panel** | Show / hide bottom panel |
| `Ctrl+Alt+B` | **Toggle Right Panel** | Show / hide extension webview panel |
| `Ctrl+Shift+M` | **Maximize Panel** | Maximize / restore bottom panel |
| `Ctrl+/` | **Toggle Comment** | Comment or uncomment selected lines |
| `Alt+Up` / `Alt+Down` | **Move Line** | Move selected line(s) up or down |
| `Ctrl+G` | **Go to Line** | Jump directly to line and column |

---

## Workspace Configuration

`nanti.` supports the following environment variables:

| Variable | Description |
|---|---|
| `NANTI_WORKSPACE` | Path to the default workspace root folder |
| `NANTI_EXTENSION` | Path to an unpacked extension directory for testing |
| `NANTI_VIEW` | Webview view identifier to bind to the right panel |
| `NANTI_NODE` | Custom path to the Node.js binary (defaults to `node` on `PATH`) |
| `NANTI_CONFIG_DIR` | Custom directory for user preferences and layout state |

User settings and window layout states are automatically persisted across sessions in `%APPDATA%\nanti\layout.json`.

---

## Workspace Crates

The repository is organized as a Cargo workspace with distinct responsibilities:

- **`crates/ui`**: Workbench shell, GPUI rendering, menus, tabs, theme, and picker integration.
- **`crates/editor-core`**: Core text rope buffer, cursor arithmetic, selections, and undo/redo stacks.
- **`crates/syntax`**: Tree-sitter parsers, highlights, and symbol outline extraction.
- **`crates/terminal`**: ConPTY session management and terminal emulator state.
- **`crates/git`**: Asynchronous background Git status, diff parser, and staging operations.
- **`crates/app`**: Native process management and IPC client for the extension host.
- **`extension-host`**: Node.js worker implementing the VS Code extension API shim.

---

## Legal, Trademarks & DMCA Compliance

`nanti.` is licensed under the [MIT License](LICENSE).

All bundled components and dependencies are open source and used in compliance with their respective licenses:
- [LEGAL.md](LEGAL.md) — Comprehensive trademark policy, nominative fair use doctrine, marketplace compliance, and DMCA takedown procedures.
- [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) — Dependency license catalog (800+ crates).
- [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt) — Verbatim third-party license texts.
- [docs/legal-compliance.md](docs/legal-compliance.md) — Contributor originality guidelines and clean-room implementation records.

### Trademark Notice & Disclaimers
- "Visual Studio Code" and "VS Code" are registered trademarks of **Microsoft Corporation**.
- "Zed" is a trademark of **Zed Industries, Inc.**
- "Claude" and "Claude Code" are trademarks of **Anthropic PBC**.
- "OpenAI" and "Codex" are trademarks of **OpenAI, Inc.**

`nanti.` is an independent open-source project and is **not affiliated with, sponsored by, or endorsed by Microsoft, Zed Industries, Anthropic, OpenAI, or Google**. Any references to "Visual Studio Code", "VS Code", or extension APIs are made strictly for technical compatibility and identification purposes under the legal doctrine of **Nominative Fair Use**. `nanti.` connects exclusively to the Eclipse Foundation's **Open VSX Registry** and does not access or redistribute software from the Microsoft Visual Studio Marketplace.

For DMCA inquiries or copyright notifications, please consult [LEGAL.md](LEGAL.md) or contact `legal@nanti.dev`.
