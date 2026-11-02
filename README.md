<div align="center">
  <img src="assets/nanti-logo.png" width="128" height="128" alt="nanti. logo" style="border-radius: 28px;" />
  <h1>nanti.</h1>
  <p><strong>Code now. Not later.</strong></p>
  <p><em>A lightweight, extensible native code editor built in Rust.</em></p>
</div>

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


## Credits & Licenses

`nanti.` dibangun di atas karya open-source. Terima kasih kepada para pembuatnya. Semua komponen dipakai **apa adanya dari crates.io, tidak dimodifikasi**, dengan lisensi aslinya tetap utuh; tidak ada kode yang di-vendor, di-patch, atau disalin dari produk lain. Kode `nanti.` sendiri (editor, find/replace, Git, layout, palette, shim extension host, dsb.) ditulis dari nol untuk proyek ini.

| Komponen | Dipakai untuk | Lisensi | Hak cipta / sumber |
|---|---|---|---|
| [GPUI](https://github.com/zed-industries/zed) | UI native | Apache-2.0 | Zed Industries, Inc. |
| [alacritty_terminal](https://github.com/alacritty/alacritty) | emulator terminal + ConPTY | Apache-2.0 | The Alacritty Project |
| [tree-sitter](https://github.com/tree-sitter/tree-sitter) + `tree-sitter-highlight` | syntax highlighting | MIT | Copyright (c) 2018 Max Brunsfeld |
| [tree-sitter-rust](https://github.com/tree-sitter/tree-sitter-rust) | grammar | MIT | Copyright (c) 2017 Maxim Sokolov |
| [tree-sitter-javascript](https://github.com/tree-sitter/tree-sitter-javascript), [tree-sitter-go](https://github.com/tree-sitter/tree-sitter-go) | grammar | MIT | Copyright (c) 2014 Max Brunsfeld |
| [tree-sitter-json](https://github.com/tree-sitter/tree-sitter-json) | grammar | MIT | Copyright (c) 2014 Max Brunsfeld |
| [tree-sitter-typescript](https://github.com/tree-sitter/tree-sitter-typescript) | grammar | MIT | Copyright (c) 2017 Max Brunsfeld |
| [tree-sitter-md](https://github.com/tree-sitter-grammars/tree-sitter-markdown) | grammar | MIT | Copyright (c) 2021 Matthias Deiml |
| [tree-sitter-dart](https://github.com/nielsenko/tree-sitter-dart) | grammar | MIT | Copyright (c) 2026 Kasper Overgård Nielsen |
| [wry](https://github.com/tauri-apps/wry) | WebView2 untuk panel extension | Apache-2.0 OR MIT | Tauri Programme within The Commons Conservancy |
| [ropey](https://github.com/cessen/ropey) | struktur teks (rope) | MIT | Nathan Vegdahl |
| [regex](https://github.com/rust-lang/regex), [serde](https://github.com/serde-rs/serde), [serde_json](https://github.com/serde-rs/json), [thiserror](https://github.com/dtolnay/thiserror) | utilitas | MIT OR Apache-2.0 | para kontributor masing-masing |
| [unicode-segmentation](https://github.com/unicode-rs/unicode-segmentation), [url](https://github.com/servo/rust-url) | grapheme cluster, validasi URL | MIT OR Apache-2.0 | The Rust Project Developers / Servo |

Daftar lengkap seluruh dependensi (846 crate beserta lisensinya): [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Untuk rilis, jalankan `node scripts/gen-third-party-notices.mjs --full` dan sertakan `THIRD_PARTY_LICENSES.txt` (salinan verbatim teks lisensi tiap paket) bersama binary. Skrip ini juga **menggagalkan build** jika ada dependensi dengan lisensi di luar daftar bebas-pakai (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0 tanpa modifikasi, dll.).

Aturan dan hasil pemeriksaan hukum: [docs/legal-compliance.md](docs/legal-compliance.md).

Ekstensi pihak ketiga (misalnya Claude Code, Codex) tidak disertakan dan tidak didistribusikan; pengguna memasangnya sendiri, tunduk pada lisensi masing-masing. `nanti.` tidak berafiliasi dengan Microsoft, Zed Industries, Anthropic, OpenAI, atau Google. "Visual Studio Code" adalah merek dagang Microsoft; disebut hanya untuk menjelaskan kompatibilitas API.
