# Legal and originality rules

Not legal advice. This records what the project does to avoid copying and infringement, what was checked,
and what still needs a person (ideally a lawyer) before a public release.

## Rules for every contributor (human or AI)

1. **Write original code.** Do not paste code from VS Code, Zed, Antigravity, Cursor, other IDEs, blog posts or
   Stack Overflow answers. Reading docs/specs to learn behavior is fine; copying expression is not.
2. **Third-party code only with a compatible license** (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0 unmodified).
   No GPL/AGPL/LGPL/SSPL/BUSL or "source available" code in the binary. Record it in `THIRD_PARTY_NOTICES.md`
   (run `node scripts/gen-third-party-notices.mjs`) and keep the original copyright headers of any file taken.
3. **Never commit or redistribute vendor extension packages** (Claude Code, Codex, any `.vsix`): they are
   proprietary. The IDE only runs a copy the user installed themselves. (`.vsix`, `gatec/` are git-ignored.)
4. **Extension source = Open VSX only.** The Visual Studio Marketplace terms restrict it to Microsoft products;
   do not call its API or build a "install from Marketplace" feature. Local `.vsix` import is the user's choice
   and responsibility.
5. **No impersonation.** `env.appName` is `Nanti`, the API version is not spoofed, UI text never says
   "Visual Studio Code". Using the names of other products is limited to factual compatibility statements
   ("VS Code extension API") and never in the product name, logo, icon or marketing as if affiliated.
6. **Own visuals.** Colors, layout metrics and icons here are our own choices; do not import Microsoft's
   Codicon font/icons or the VS Code themes/logos without checking their licenses (Codicons are CC-BY-4.0
   and need attribution; the VS Code name and logo are trademarks).
7. **Fonts:** only system fonts are referenced (Consolas, Segoe UI); none are bundled. If a font is bundled
   later, it must be OFL/Apache licensed and listed in the notices.

## Checked on 2026-11-02

| Item                                                | Result                                                                                                                                                                                                                                                                                                                                                              |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dependency licenses (`cargo metadata`, 800+ crates) | All permissive or dual-licensed with a permissive option. 7 MPL-2.0 crates (file-level copyleft; used unmodified: `cbindgen`, `cssparser`, `cssparser-macros`, `dtoa-short`, `dwrote`, `option-ext`, `selectors`). `r-efi` (MIT OR Apache-2.0 OR LGPL) and `self_cell` (Apache-2.0 OR GPL-2.0) are used under the permissive choice. No GPL-only/proprietary crate. |
| Vendor files tracked in git                         | None (`.vsix`, vendor, extracted extension: no matches). The probe extracted the Claude Code VSIX outside the repo.                                                                                                                                                                                                                                                 |
| Copied third-party text/code                        | None known. Source comments only mention other products to describe behavior ("as in VS Code's terminal"). The VS Code API shim is written from the public API behavior and the PRD matrix, not from Microsoft source.                                                                                                                                              |
| Branding in the product                             | UI shows `nanti.`; `appName` is `Nanti`. Docs say "VS Code-style" only as a description of the layout.                                                                                                                                                                                                                                                              |
| Notice file                                         | `THIRD_PARTY_NOTICES.md` generated (needed when distributing a binary: Apache-2.0 and MIT require the notice and license text to travel with it).                                                                                                                                                                                                                   |

## Project License and Status

- **Project license:** Licensed under the [MIT License](../LICENSE). All code developed natively for `nanti.` is distributed under MIT.
- **Legal & Trademark Policy:** Formalized in [LEGAL.md](../LEGAL.md) (nominative fair use doctrine, marketplace boundaries, clean-room defense, DMCA 512 notice procedure).
- **Third-party notices:** Recorded in [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md) and [THIRD_PARTY_LICENSES.txt](../THIRD_PARTY_LICENSES.txt).

## Pre-Release Legal Verification

- **Marketplace ToS Isolation:** `nanti.` connects strictly to Open VSX (`open-vsx.org`); no requests are sent to Microsoft Visual Studio Marketplace.
- **Vendor Packages:** No proprietary extension packages (e.g. Claude Code, Codex, Copilot) are bundled or redistributed.
- **DMCA Takedown Contact:** Formal Designated Agent notification protocol established in [LEGAL.md](../LEGAL.md).

## Still open (needs a decision or a lawyer)

- **Name "nanti.":** no trademark search done. Check local (DJKI) and international registers and domain
  availability before launch. The pronoun-less name is generic Indonesian ("later"), which may be hard to protect.
- **Full license texts:** `THIRD_PARTY_NOTICES.md` lists license names and sources. For a release build, bundle the
  full texts (for example with `cargo about`) and the notice file in the installer and the "About" screen.
- **Code originality can't be proven by a tool.** The code in this repo was written for this project with AI
  assistance; AI output can resemble public code. Before release run a similarity scan (for example a license
  scanner such as ScanCode or FOSSA) over `crates/` and `extension-host/`.
- **Claude Code / Codex terms:** running their extensions inside a non-Microsoft host depends on each vendor's
  terms. Re-read them (and `env.openExternal`/login flows) before advertising support. The PRD already says
  compatibility is not promised.
- **Patents and WebView2:** the WebView2 runtime is Microsoft's redistributable under its own license; do not bundle
  it except through Microsoft's documented installer.
