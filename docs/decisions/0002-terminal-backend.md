# ADR 0002: terminal backend (PTY + emulator)

Status: accepted (2026-11-02). Evidence: spike run + `crates/terminal` tests.

## Context

PRD 4.3 asks for ConPTY plus a terminal emulator written in Rust, with the library chosen by a spike.
Candidates: `alacritty_terminal` (used by Zed, includes a Windows ConPTY backend), `portable-pty` + `vte`
(PTY and a parser, but the grid, scrollback, selection and modes would be ours), `termwiz`.

## Spike (throw-away crate, not in the repo)

`alacritty_terminal 0.26` on Windows 11 with `powershell.exe`:
- shell ready in ~250 ms; `Get-Location` shows the requested cwd;
- a 20 000-line flood finished without stalling the reader (scrollback capped at 10 000);
- resize, exit code (`exit 7` -> 7) and Shutdown (no orphan shell) all worked.

Two things the spike got wrong first, both worth recording:
1. **The terminal asks the host questions** (`Event::PtyWrite` for cursor-position reports, size and color
   requests). If the host ignores them, ConPTY-hosted PowerShell blocks forever on startup. The listener must
   answer them.
2. Wide (CJK) characters occupy two grid cells; the second is a spacer cell that must be skipped when turning
   the grid into text.

## Decision

Use `alacritty_terminal` behind a small GUI-free wrapper (`crates/terminal`): spawn, write, paste, resize,
scroll, selection, kill and a `snapshot()` of colored runs per row. The UI never sees alacritty types.

Security-relevant choices:
- OSC 52 (programs reading/writing the clipboard) is **disabled**; clipboard events are dropped.
- Paste in bracketed mode strips embedded `ESC[200~` / `ESC[201~` so pasted text cannot end the bracket and run
  as typed commands; otherwise line breaks become CR.
- Dropping or killing a `Terminal` shuts the PTY down; closing the window kills every shell.

## Consequences

- PowerShell receives `TERM_PROGRAM=nanti` and `COLORTERM=truecolor`. Shell override: `NANTI_SHELL`.
- No mouse reporting, hyperlinks or images yet; see the limits row in docs/m0-status.md.
- The extension-facing terminal API (TR-04) will be built on the same wrapper.
