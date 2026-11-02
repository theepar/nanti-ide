# vscode API compatibility matrix

Source of truth for what the host supports. `SUPPORTED` means covered by an automated test; everything else is `MISSING` and throws `UnsupportedCapability`.

## Claude Code (`Anthropic.claude-code` 2.1.316, win32-x64) — Gate C, 2026-11-02

| Field | Value |
| --- | --- |
| Source | Open VSX, `win32-x64` build (default endpoint returns `alpine-arm64`; always request the platform) |
| Size / SHA-256 | 160417327 bytes / `58e14f6ca775035732433041c390edff77553f243bf6ee23105eccb69fe2758e` (matches Open VSX published checksum) |
| Package safety | 49 entries, 0 absolute or `..` paths (CT-007 style check before extract) |
| Entry | `main: ./extension.js` (3.4 MB bundle), `engines.vscode ^1.94.0` |
| Activation | `onStartupFinished`, `onWebviewPanel:claudeVSCodePanel` |
| Proposed APIs | `enabledApiProposals` empty → no private/proposed API declared (good for R-01) |
| UI | Webview views `claudeVSCodeSidebar`, `claudeVSCodeSidebarSecondary`, `claudeVSCodeSessionsList` → `registerWebviewViewProvider` is mandatory |
| Bundled binary | `resources/native-binary/claude.exe` (AI-12 will execute it) |
| License | All rights reserved; use per vendor terms. **Never commit or redistribute the VSIX** (R-08). |

**Run status: FAIL (not activated).** Probe run 2026-11-02 with `scripts/probe-extension.mjs` (in-process host, temp workspace, no login, no credentials; `claude.exe` was never spawned).

| Mode | Result |
| --- | --- |
| strict | Module load fails at top level: first unsupported API is `vscode.NotebookCellOutputItem.error` (module-level `NotebookCellOutputItem.error(Error("")).mime`). Proves the 3.4 MB bundle parses and runs up to that line. |
| discover (inventory only, stubs, never counts as working) | Unsupported APIs reached: `vscode.NotebookCellOutputItem.error`, `vscode.ExtensionMode.Production`, `vscode.env.remoteName`. Then `activate()` is entered and fails: `ExtensionContext.globalStorageUri` is undefined (`new b5(context)` reads `context.globalStorageUri.fsPath`). |

### After slice 1 (same day, re-run)

Slice 1 added: `ExtensionContext` (`extensionUri`, `extensionMode`, `globalStorageUri`, `storageUri`, `logUri`, persistent `globalState`/`workspaceState`; `secrets` intentionally still unsupported), `Uri.parse/joinPath/with`, `EventEmitter`, `ExtensionMode`, `NotebookCellOutputItem.error`, `env` constants (`appName` is `Nanti`, not VS Code), and `createOutputChannel({log:true})`.

| Mode | Result |
| --- | --- |
| strict | Module loads and `activate()` is entered. First failure: `vscode.workspace.getConfiguration` (called from `migrateOldConfigSettings`). |
| discover | Unsupported APIs reached: `window.onDidChangeActiveTextEditor`, `onDidChangeTextEditorSelection`, `onDidChangeVisibleTextEditors`, `window.tabGroups.all`, `workspace.getConfiguration`, `onDidChangeConfiguration`, `onDidChangeTextDocument`, `onDidChangeWorkspaceFolders`, `onDidCloseTextDocument`, `registerFileSystemProvider`, `registerTextDocumentContentProvider`. Stops at `new Set(window.tabGroups.all)` (the discover stub is not iterable; a real `tabGroups` is needed). |

Status: still **FAIL (not activated)**. Next slice (2): `workspace.getConfiguration` + `onDidChangeConfiguration` (VH-09), `ConfigurationTarget`, `window.tabGroups`, text-document/editor events (VH-04), `registerFileSystemProvider`/`registerTextDocumentContentProvider` (can start as explicit registration with unsupported behavior only if the extension tolerates it; otherwise implement).

### After slice 2 (same day, re-run)

Slice 2 added: `workspace.getConfiguration` (manifest defaults, global/workspace precedence, `get/has/inspect/update`; updates are forwarded to the IDE as `workspace.configuration.update` notifications), `onDidChangeConfiguration` (real events), `ConfigurationTarget`, `window.tabGroups`, editor/document state and events, `registerFileSystemProvider` / `registerTextDocumentContentProvider` (registry only), `workspace.openTextDocument` (file scheme), `RelativePattern`, `StatusBarAlignment`, and `vscode.version`.

Limitations (do not read as full support):
- Editor/document state is **empty and static** until the IDE pushes state; events exist but never fire yet. `activeTextEditor` is `undefined`, `tabGroups` has one empty group.
- `openTextDocument` reads from **disk**: unsaved IDE edits are invisible to extensions (R-10 data-loss risk for AI edits). Needs the IDE buffer bridge before AI-04/05.
- Registered file-system/content providers are stored but never called yet.

| Mode | Result |
| --- | --- |
| strict | Module loads, `activate()` runs through config, tab groups, file watchers (three `createFileSystemWatcher` calls were caught by the extension), then fails at **`vscode.window.registerWebviewViewProvider`**. This is the expected core API (VH-06). |
| discover (inventory only) | Remaining unsupported: `extensions.all`, `window.createStatusBarItem`, `window.registerUriHandler`, `window.registerWebviewPanelSerializer`, `window.registerWebviewViewProvider`, `workspace.createFileSystemWatcher`. With stubs the activation completes, which proves nothing about behavior and is **not** counted. |

Status: still **FAIL (not activated)**, but the remaining blockers are now the Gate C core: webview view/panel providers, URI handler (login), file watcher, status bar item, `extensions.all`.

**Open decision (needs ADR): `vscode.version`.** It currently returns the host's own version (`0.0.1`), per PRD 5.4/3 #6 (no spoofing). The extension passed its version check with it, but other extensions may gate features on `vscode.version >= 1.x` and degrade or refuse. If that happens, options are: (a) keep honest and accept degraded features, (b) declare an explicit, documented compat level (e.g. "implements the 1.94 API subset listed in this matrix") with the matrix as the evidence. Do not change this silently.

### After slice 3 (same day, re-run)

Slice 3 added (host): `window.registerWebviewViewProvider`, `createWebviewPanel`, `registerWebviewPanelSerializer` (registry only), `webview.asWebviewUri` (→ `https://nanti-resource.localhost/file/<enc path>`), per-extension `localResourceRoots` (default: extension dir + workspace folders), `registerUriHandler` + `uri.handle` routed by extension id (VH-07 partial), `workspace.createFileSystemWatcher` (real `fs.watch`, `.git`/`node_modules` ignored), `window.createStatusBarItem`, `window.showWarningMessage`, `extensions.getExtension/all`, built-in `setContext`, `ExtensionContext.extension`, `ExtensionContext.environmentVariableCollection` (recorded only, not applied to terminals until the terminal API exists).
Slice 3 added (IDE, `crates/ui`): WebView2 child panel in the right-hand area following window resize, `acquireVsCodeApi()` bridge, `nanti-resource` protocol with deny-by-default root enforcement (traversal and symlinks blocked via canonicalize), async host client (reader thread, timeouts).

| Check | Result |
| --- | --- |
| Probe strict (in-process, no stubs) | **`activated`**, zero unsupported APIs reached during activation |
| Probe `--resolve=claudeVSCodeSidebar` | view resolves; 9.3 KB HTML with CSP, script/style via our resource scheme, `localResourceRoots` limited to `webview/` and `resources/`; no errors shown to user |
| Live in `nanti-ui` (real WebView2, isolated profile) | panel renders Claude Code's login screen ("How do you want to log in? Claude.ai Subscription …"); webview↔extension protocol runs (`init`, `get_claude_state`, `launch_claude`, `io_message`, …); no page JS errors |
| Unsupported APIs hit in that live idle session | none |

Not yet exercised (so NOT claimed): login round-trip, sending a prompt, edits/diff, terminals. Known next blockers: `env.openExternal` (needs a user-consent prompt per PRD 7.2 #1 before opening a browser), `env.clipboard`, `window.createTerminal` + terminal events (VH-10, AI-12), `window.showTextDocument`/`applyEdit`/real editor state (AI-04/05), `SecretStorage` (VH-08), `showQuickPick`/`showInputBox`.

Side effects found while probing (and handled):
- On activation the extension writes an IDE lock file to `~/.claude/ide/` so Claude CLI sessions can discover the "IDE". The probe now sandboxes `USERPROFILE/HOME/APPDATA/LOCALAPPDATA`; one stale lock from an unsandboxed run was removed (only that file; the user's own Antigravity locks were untouched).
- The extension itself runs `launch_claude`, which starts the bundled `claude.exe`. Redirecting `USERPROFILE` is NOT a full sandbox for that native binary: it still listed commands/skills that come from the real profile (read-only listing; no credentials entered, no prompt sent). Treat any run of the real extension as touching the real profile.

### First-run findings (before slice 1)

So the first real blockers were: `NotebookCellOutputItem.error` (needs a minimal class), `ExtensionMode` enum, `env.remoteName`, and the missing `ExtensionContext` fields (`globalStorageUri`, likely `storageUri`, `extensionUri`, `extensionMode`, `globalState`, `workspaceState`, `secrets`). Discover mode stops at the first TypeError, so more will surface after each fix; re-run after each slice.

The gap list below is from a static scan of `extension.js` for `.window/.workspace/...` accesses. A static scan can miss aliased or dynamic access and says nothing about whether calls succeed.

Probe note: bundlers (`__toESM`) copy only own keys of the `vscode` module, so unsupported top-level names must exist as own keys (they now do, via a name list in `vscode-api.mjs`); otherwise they appear as `undefined` and fail with a misleading TypeError.

### Gap list (used by the extension, not in the host)

| Namespace | Needed, MISSING |
| --- | --- |
| `window` | `registerWebviewViewProvider`, `createWebviewPanel`, `registerWebviewPanelSerializer`, `registerUriHandler`, `createTerminal`, `terminals`, `activeTerminal`, terminal shell-integration events (`onDidChangeTerminalShellIntegration`, `onDidStartTerminalShellExecution`, `onDidEndTerminalShellExecution`), `activeTextEditor`, `visibleTextEditors`, `showTextDocument`, `tabGroups`, `onDidChangeActiveTextEditor`, `onDidChangeVisibleTextEditors`, `onDidChangeTextEditorSelection`, `createTextEditorDecorationType`, `createStatusBarItem`, `setStatusBarMessage`, `showQuickPick`, `showInputBox`, `showWarningMessage`, `withProgress`, `state`, `activeNotebookEditor` |
| `workspace` | `getConfiguration`, `onDidChangeConfiguration`, `openTextDocument`, `textDocuments`, `onDidOpen/Close/Save/ChangeTextDocument`, `onWillSaveTextDocument`, `applyEdit`, `findFiles`, `asRelativePath`, `getWorkspaceFolder`, `onDidChangeWorkspaceFolders`, `createFileSystemWatcher`, `registerFileSystemProvider`, `registerTextDocumentContentProvider`, `rootPath`, `workspaceFile` |
| `env` | `openExternal`, `clipboard`, `machineId`, `sessionId`, `appName`, `uiKind`, `shell`, `remoteName`, `isTelemetryEnabled`, `onDidChangeTelemetryEnabled` |
| `commands` | `getCommands` |
| `languages` | `getDiagnostics`, `onDidChangeDiagnostics` |
| `extensions` | `getExtension`, `all` |
| `ExtensionContext` | `secrets` (SecretStorage), `globalState`, `workspaceState`, `extensionUri`, `storageUri`, `globalStorageUri` (host context only has `subscriptions`, `extensionPath`) |
| Classes/enums | `EventEmitter`, `ThemeIcon`, `Range`, `Position`, `Selection`, `WorkspaceEdit`, `Uri.parse/joinPath`, `StatusBarAlignment`, … (not visible in the scan; expect more at first activation) |

### Priority order (smallest slice that reaches a visible panel)

1. `ExtensionContext` state + `EventEmitter`/`Uri`/`Range`/`Position` basics, `workspace.getConfiguration` + change events, `extensions.getExtension`, `env.*` constants.
2. `window.registerWebviewViewProvider` + webview message bridge to a WebView2 panel (VH-06).
3. `env.openExternal` + `registerUriHandler` (login round-trip, VH-07), `SecretStorage` (VH-08).
4. Documents/editors: `openTextDocument`, `activeTextEditor`, `applyEdit`, `tabGroups` (AI-04/05).
5. Terminal API (VH-10, AI-12).

## Supported now (tested)

| API | Test |
| --- | --- |
| `commands.registerCommand/executeCommand` | `extension-host/tests/host.test.mjs` |
| `workspace.workspaceFolders`, `workspace.fs.readFile/writeFile/stat` | same |
| `window.showInformationMessage/showErrorMessage`, `createOutputChannel` | same (messages are forwarded to the IDE as notifications) |
| `Disposable`, `Uri.file` | same |
