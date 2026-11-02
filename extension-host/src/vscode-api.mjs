// Supported subset of the `vscode` API (PRD 5.4). Anything else throws UnsupportedCapability
// and is logged; never a fake success.
import fs from "node:fs/promises";
import crypto from "node:crypto";
import os from "node:os";
import path from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";
import { watch as fsWatch, existsSync } from "node:fs";
import { ConfigStore, CONFIGURATION_TARGET } from "./configuration.mjs";
import { createWebviewHub } from "./webview.mjs";

/** Glob subset used by file watchers: `**`, `*`, `?`, `{a,b}`; matched against `/`-separated relative paths. */
export function globToRegExp(pattern) {
  let re = "";
  let braces = 0;
  for (let i = 0; i < pattern.length; i++) {
    const c = pattern[i];
    if (c === "*") {
      if (pattern[i + 1] === "*") {
        i++;
        if (pattern[i + 1] === "/") {
          i++;
          re += "(?:.*/)?";
        } else re += ".*";
      } else re += "[^/]*";
    } else if (c === "?") re += "[^/]";
    else if (c === "{") {
      braces++;
      re += "(?:";
    } else if (c === "}" && braces > 0) {
      braces--;
      re += ")";
    } else if (c === "," && braces > 0) re += "|";
    else re += c.replace(/[.+^$()|[\]\\]/g, "\\$&");
  }
  return new RegExp(`^${re}$`);
}

const IGNORED_SEGMENTS = new Set([".git", "node_modules"]);

export class UnsupportedCapability extends Error {
  constructor(name) {
    super(`UnsupportedCapability: ${name}`);
    this.name = "UnsupportedCapability";
    this.code = "UnsupportedCapability";
  }
}

function strict(path, impl) {
  return new Proxy(impl, {
    get(target, prop, receiver) {
      if (typeof prop === "symbol" || prop in target) return Reflect.get(target, prop, receiver);
      if (prop === "then" || prop === "default" || prop === "__esModule") return undefined;
      return unsupported(`${path}.${String(prop)}`);
    },
  });
}

// Discovery mode (compatibility probing only, never on by default): unsupported APIs are
// recorded in `collector` and answered with an inert stub so activation can continue and we
// learn the full set. Results from a run in this mode never count as "working" (PRD 5.5).
let collector = null;
export function setDiscoveryCollector(set) {
  collector = set;
}

function stub(name) {
  const self = new Proxy(function () {}, {
    apply: () => self,
    construct: () => self,
    get(_t, prop) {
      if (prop === "then") return undefined;
      if (prop === Symbol.toPrimitive) return () => "";
      if (typeof prop === "symbol") return undefined;
      return stub(`${name}.${prop}`);
    },
  });
  return self;
}

export const unsupportedValue = (name) => unsupported(name);

// Probing (`vscode.debug`) is allowed; calling it or reaching into it throws.
function unsupported(name) {
  const fail = (n) => {
    console.error(`[host] UnsupportedCapability: ${n}`);
    if (collector) {
      collector.add(n);
      return stub(n);
    }
    throw new UnsupportedCapability(n);
  };
  return new Proxy(function () {}, {
    apply: () => fail(name),
    construct: () => fail(name),
    get(_t, prop) {
      if (typeof prop === "symbol" || prop === "then") return undefined;
      return fail(`${name}.${String(prop)}`);
    },
  });
}

class Disposable {
  constructor(fn) {
    this.fn = fn;
  }
  dispose() {
    const fn = this.fn;
    this.fn = null;
    fn?.();
  }
}

const encodePath = (p) => encodeURI(p).replace(/[?#]/g, (c) => (c === "?" ? "%3F" : "%23"));

export const EXTENSION_MODE = Object.freeze({ Production: 1, Development: 2, Test: 3 });

export class Uri {
  constructor(scheme, authority, uriPath, query, fragment) {
    this.scheme = scheme;
    this.authority = authority;
    this.path = uriPath;
    this.query = query;
    this.fragment = fragment;
  }
  get fsPath() {
    if (this.scheme !== "file") return this.path;
    return fileURLToPath(`file://${this.authority}${encodePath(this.path)}`);
  }
  static file(p) {
    const u = pathToFileURL(p);
    return new Uri("file", "", decodeURIComponent(u.pathname), "", "");
  }
  static parse(value) {
    const u = new URL(value);
    return new Uri(u.protocol.replace(/:$/, ""), u.host, decodeURIComponent(u.pathname), u.search.replace(/^\?/, ""), u.hash.replace(/^#/, ""));
  }
  static joinPath(base, ...segments) {
    if (base.scheme === "file") return Uri.file(path.join(base.fsPath, ...segments));
    return new Uri(base.scheme, base.authority, path.posix.join(base.path, ...segments), base.query, base.fragment);
  }
  with(change = {}) {
    return new Uri(
      change.scheme ?? this.scheme,
      change.authority ?? this.authority,
      change.path ?? this.path,
      change.query ?? this.query,
      change.fragment ?? this.fragment,
    );
  }
  toString() {
    if (this.scheme === "file") return `file://${this.authority}${encodePath(this.path)}`;
    return `${this.scheme}://${this.authority}${encodePath(this.path)}${this.query ? "?" + this.query : ""}${this.fragment ? "#" + this.fragment : ""}`;
  }
  toJSON() {
    return { scheme: this.scheme, authority: this.authority, path: this.path, query: this.query, fragment: this.fragment, fsPath: this.fsPath };
  }
}

export class EventEmitter {
  #listeners = new Set();
  event = (listener, thisArgs, disposables) => {
    const l = thisArgs ? listener.bind(thisArgs) : listener;
    this.#listeners.add(l);
    const d = new Disposable(() => this.#listeners.delete(l));
    disposables?.push(d);
    return d;
  };
  fire(data) {
    for (const l of [...this.#listeners]) {
      try {
        l(data);
      } catch (e) {
        console.error(`[host] event listener threw: ${e?.stack ?? e}`);
      }
    }
  }
  dispose() {
    this.#listeners.clear();
  }
}

export const HOST_VERSION = "0.0.1";

export class RelativePattern {
  constructor(base, pattern) {
    this.baseUri = typeof base === "string" ? Uri.file(base) : (base.uri ?? base);
    this.base = this.baseUri.fsPath;
    this.pattern = pattern;
  }
}

export class NotebookCellOutputItem {
  constructor(data, mime) {
    this.data = data;
    this.mime = mime;
  }
  static error(value) {
    const payload = { name: value?.name ?? "Error", message: value?.message ?? "", stack: value?.stack };
    return new NotebookCellOutputItem(Buffer.from(JSON.stringify(payload)), "application/vnd.code.notebook.error");
  }
}

export const TerminalLocation = Object.freeze({
  Panel: 1,
  Editor: 2,
});

export const TerminalExitReason = Object.freeze({
  Unknown: 0,
  Shutdown: 1,
  Process: 2,
  User: 3,
  Extension: 4,
});

// Public top-level `vscode` exports we do not implement. Bundlers (`__toESM`) snapshot only own
// keys, so unlisted names would become `undefined` and fail with a misleading TypeError.
// Each is exposed as an own getter that yields an "unsupported" proxy (throws on use).
const UNSUPPORTED_TOP_LEVEL = [
  "authentication", "chat", "comments", "debug", "env", "extensions", "l10n", "languages", "lm",
  "notebooks", "scm", "tasks", "tests",
  "version", "CallHierarchyItem", "CancellationError", "CancellationTokenSource", "CodeAction",
  "CodeActionKind", "CodeLens", "Color", "ColorInformation", "ColorPresentation", "ColorThemeKind",
  "CommentMode", "CommentThreadCollapsibleState", "CompletionItem", "CompletionItemKind",
  "CompletionList", "ConfigurationTarget", "CustomExecution", "DataTransfer", "DataTransferItem",
  "DebugAdapterExecutable", "DebugAdapterInlineImplementation", "DebugAdapterNamedPipeServer",
  "DebugAdapterServer", "DecorationRangeBehavior", "Diagnostic", "DiagnosticRelatedInformation",
  "DiagnosticSeverity", "DiagnosticTag", "DocumentHighlight", "DocumentHighlightKind", "DocumentLink",
  "DocumentSymbol", "EndOfLine", "EvaluatableExpression", "EventEmitter", "ExtensionKind",
  "ExtensionMode", "ExternalUriOpenerPriority", "FileChangeType", "FileSystemError", "FileType",
  "FoldingRange", "FoldingRangeKind", "FunctionBreakpoint", "Hover", "InlayHint", "InlayHintKind",
  "InlineCompletionItem", "InlineCompletionList", "InlineCompletionTriggerKind",
  "InlineValueEvaluatableExpression", "InlineValueText", "InlineValueVariableLookup",
  "LanguageModelChatMessage", "LanguageModelChatMessageRole", "LanguageModelError",
  "LanguageModelTextPart", "LanguageModelToolCallPart", "LanguageModelToolResult",
  "LanguageModelToolResultPart", "LanguageStatusSeverity", "Location", "LocationLink", "LogLevel",
  "MarkdownString", "NotebookCellData", "NotebookCellKind", "NotebookCellOutput",
  "NotebookCellOutputItem", "NotebookCellStatusBarAlignment", "NotebookCellStatusBarItem",
  "NotebookControllerAffinity", "NotebookData", "NotebookEdit", "NotebookEditorRevealType",
  "NotebookRange", "OverviewRulerLane", "ParameterInformation", "Position", "ProcessExecution",
  "ProgressLocation", "QuickInputButtons", "QuickPickItemKind", "Range", "RelativePattern",
  "Selection", "SelectionRange", "SemanticTokens", "SemanticTokensBuilder", "SemanticTokensLegend",
  "ShellExecution", "SignatureHelp", "SignatureInformation", "SnippetString", "SnippetTextEdit",
  "SourceBreakpoint", "StatusBarAlignment", "SymbolInformation", "SymbolKind", "TabInputCustom",
  "TabInputNotebook", "TabInputNotebookDiff", "TabInputTerminal", "TabInputText", "TabInputTextDiff",
  "TabInputWebview", "Task", "TaskGroup", "TaskPanelKind", "TaskRevealKind", "TaskScope",
  "TerminalLink", "TerminalProfile", "TestMessage",
  "TestRunProfileKind", "TestRunRequest", "TestTag", "TextEdit", "TextEditorCursorStyle",
  "TextEditorLineNumbersStyle", "TextEditorRevealType", "TextEditorSelectionChangeKind", "ThemeColor",
  "ThemeIcon", "TreeItem", "TreeItemCheckboxState", "TreeItemCollapsibleState", "TypeHierarchyItem",
  "UIKind", "ViewColumn", "WorkspaceEdit",
];

const LANGUAGE_BY_EXT = {
  ".js": "javascript", ".mjs": "javascript", ".cjs": "javascript", ".jsx": "javascriptreact",
  ".ts": "typescript", ".tsx": "typescriptreact", ".json": "json", ".md": "markdown",
  ".rs": "rust", ".go": "go", ".dart": "dart", ".toml": "toml", ".yaml": "yaml", ".yml": "yaml",
};

/** One vscode namespace instance per host; `notify` pushes host->IDE notifications. */
export function createVscode({ workspaceFolders = [], notify, settings = {}, getExtensions = () => [] }) {
  const commandTable = new Map();
  const contextKeys = new Map();
  // Built-in command the extensions rely on; keys are forwarded so the IDE can evaluate `when` clauses.
  commandTable.set("setContext", (key, value) => {
    contextKeys.set(key, value);
    notify("context.set", { key, value: value ?? null });
  });
  const hub = createWebviewHub({ notify, Disposable, EventEmitter, Uri, workspaceFolders });
  const uriHandlers = new Map(); // extension dir -> UriHandler
  const extensionDirs = new Set();
  const externalRequests = new Map();
  let externalSeq = 0;
  let statusSeq = 0;

  function openExternal(uri) {
    const value = uri instanceof Uri ? uri.toString() : String(uri ?? "");
    if (/[\u0000-\u0020\u007f]/.test(value)) return Promise.resolve(false);
    let parsed;
    try {
      parsed = new URL(value);
    } catch {
      return Promise.resolve(false);
    }
    if (!["http:", "https:"].includes(parsed.protocol) || !parsed.hostname || parsed.username || parsed.password) {
      return Promise.resolve(false);
    }
    return new Promise((resolve) => {
      const id = `external-${++externalSeq}`;
      const timer = setTimeout(() => {
        externalRequests.delete(id);
        resolve(false);
      }, 120_000);
      timer.unref?.();
      externalRequests.set(id, { resolve, timer });
      try {
        notify("env.openExternal", { id, url: value });
      } catch {
        clearTimeout(timer);
        externalRequests.delete(id);
        resolve(false);
      }
    });
  }

  function resolveExternal(id, approved) {
    const request = externalRequests.get(id);
    if (!request) return false;
    externalRequests.delete(id);
    clearTimeout(request.timer);
    request.resolve(approved === true);
    return true;
  }
  const emitters = {};
  const emitter = (name) => (emitters[name] ??= new EventEmitter());
  const providers = { fileSystem: new Map(), textDocumentContent: new Map() };

  // Editor/document state. Empty until the IDE pushes state (not wired yet): this reports "nothing
  // open", it does not mirror IDE buffers. `openTextDocument` reads from DISK, so unsaved IDE
  // edits are NOT visible to extensions yet (tracked limitation, see api-compatibility-matrix).
  const state = { documents: new Map(), visibleEditors: [], activeEditor: undefined };

  let termSeq = 0;
  const terminals = [];
  let activeTerminal = undefined;

  function syncOpenDocument({ uri, languageId, version, text }) {
    const parsed = typeof uri === "string" ? Uri.parse(uri) : uri;
    const key = parsed.toString();
    const doc = {
      uri: parsed,
      fileName: parsed.fsPath,
      languageId: languageId || LANGUAGE_BY_EXT[path.extname(parsed.fsPath).toLowerCase()] || "plaintext",
      version: version ?? 1,
      isDirty: false,
      isUntitled: false,
      isClosed: false,
      lineCount: text === "" ? 1 : text.split(/\r\n|\r|\n/).length,
      getText(range) {
        if (range !== undefined) throw new UnsupportedCapability("vscode.TextDocument.getText(range)");
        return text;
      },
    };
    state.documents.set(key, doc);
    state.activeEditor = {
      document: doc,
      selection: { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } },
      selections: [],
      visibleRanges: [],
    };
    state.visibleEditors = [state.activeEditor];
    emitter("onDidOpenTextDocument").fire(doc);
    emitter("onDidChangeActiveTextEditor").fire(state.activeEditor);
    return { ok: true };
  }

  function syncChangeDocument({ uri, version, text, contentChanges }) {
    const parsed = typeof uri === "string" ? Uri.parse(uri) : uri;
    const key = parsed.toString();
    let doc = state.documents.get(key);
    if (!doc) {
      return syncOpenDocument({ uri: parsed, version, text });
    }
    doc.version = version ?? (doc.version + 1);
    doc.isDirty = true;
    doc.lineCount = text === "" ? 1 : text.split(/\r\n|\r|\n/).length;
    doc.getText = (range) => {
      if (range !== undefined) throw new UnsupportedCapability("vscode.TextDocument.getText(range)");
      return text;
    };
    emitter("onDidChangeTextDocument").fire({
      document: doc,
      contentChanges: contentChanges || [{ text }],
      reason: undefined,
    });
    return { ok: true };
  }

  function syncSaveDocument({ uri }) {
    const parsed = typeof uri === "string" ? Uri.parse(uri) : uri;
    const key = parsed.toString();
    const doc = state.documents.get(key);
    if (doc) {
      doc.isDirty = false;
      emitter("onDidSaveTextDocument").fire(doc);
    }
    return { ok: true };
  }

  function syncCloseDocument({ uri }) {
    const parsed = typeof uri === "string" ? Uri.parse(uri) : uri;
    const key = parsed.toString();
    const doc = state.documents.get(key);
    if (doc) {
      doc.isClosed = true;
      state.documents.delete(key);
      if (state.activeEditor?.document === doc) {
        state.activeEditor = undefined;
        state.visibleEditors = [];
        emitter("onDidChangeActiveTextEditor").fire(undefined);
      }
      emitter("onDidCloseTextDocument").fire(doc);
    }
    return { ok: true };
  }

  const config = new ConfigStore({
    settings,
    notify,
    onChange: (e) => emitter("onDidChangeConfiguration").fire(e),
  });

  const tabGroup = { tabs: [], isActive: true, viewColumn: 1, activeTab: undefined };
  const tabGroups = strict("vscode.window.tabGroups", {
    all: [tabGroup],
    activeTabGroup: tabGroup,
    onDidChangeTabs: emitter("tabGroups.onDidChangeTabs").event,
    onDidChangeTabGroups: emitter("tabGroups.onDidChangeTabGroups").event,
  });

  const register = (map, kind) => (scheme, provider) => {
    if (map.has(scheme)) throw new Error(`${kind} for scheme '${scheme}' already registered`);
    map.set(scheme, provider);
    return new Disposable(() => map.delete(scheme));
  };

  const commands = strict("vscode.commands", {
    registerCommand(id, handler) {
      if (commandTable.has(id)) throw new Error(`command '${id}' already exists`);
      commandTable.set(id, handler);
      return new Disposable(() => commandTable.delete(id));
    },
    async executeCommand(id, ...args) {
      const h = commandTable.get(id);
      if (!h) throw new Error(`command '${id}' not found`);
      return h(...args);
    },
  });

  const folders = workspaceFolders.map((p, index) => ({
    uri: Uri.file(p),
    name: p.split(/[\\/]/).filter(Boolean).pop(),
    index,
  }));

  const workspace = strict("vscode.workspace", {
    workspaceFolders: folders.length ? folders : undefined,
    getConfiguration: (section) => config.section(section),
    onDidChangeConfiguration: emitter("onDidChangeConfiguration").event,
    onDidChangeWorkspaceFolders: emitter("onDidChangeWorkspaceFolders").event,
    onDidChangeTextDocument: emitter("onDidChangeTextDocument").event,
    onDidOpenTextDocument: emitter("onDidOpenTextDocument").event,
    onDidCloseTextDocument: emitter("onDidCloseTextDocument").event,
    onDidSaveTextDocument: emitter("onDidSaveTextDocument").event,
    onWillSaveTextDocument: emitter("onWillSaveTextDocument").event,
    createFileSystemWatcher(glob, ignoreCreate = false, ignoreChange = false, ignoreDelete = false) {
      const pattern = typeof glob === "string" ? glob : glob.pattern;
      const roots = typeof glob === "string" ? workspaceFolders : [glob.base];
      const re = globToRegExp(pattern);
      const created = new EventEmitter();
      const changed = new EventEmitter();
      const deleted = new EventEmitter();
      const watchers = [];
      for (const root of roots) {
        try {
          watchers.push(
            fsWatch(root, { recursive: true }, (event, filename) => {
              if (!filename) return;
              const rel = String(filename).replace(/\\/g, "/");
              if (rel.split("/").some((s) => IGNORED_SEGMENTS.has(s)) || !re.test(rel)) return;
              const uri = Uri.file(path.join(root, rel));
              if (event === "rename") {
                if (existsSync(uri.fsPath)) {
                  if (!ignoreCreate) created.fire(uri);
                } else if (!ignoreDelete) deleted.fire(uri);
              } else if (!ignoreChange) changed.fire(uri);
            }),
          );
        } catch (e) {
          console.error(`[host] file watcher failed for ${root}: ${e?.message ?? e}`);
        }
      }
      return {
        ignoreCreateEvents: ignoreCreate,
        ignoreChangeEvents: ignoreChange,
        ignoreDeleteEvents: ignoreDelete,
        onDidCreate: created.event,
        onDidChange: changed.event,
        onDidDelete: deleted.event,
        dispose() {
          for (const w of watchers) w.close();
          created.dispose();
          changed.dispose();
          deleted.dispose();
        },
      };
    },
    registerFileSystemProvider: register(providers.fileSystem, "FileSystemProvider"),
    registerTextDocumentContentProvider: register(providers.textDocumentContent, "TextDocumentContentProvider"),
    get textDocuments() {
      return [...state.documents.values()];
    },
    async openTextDocument(arg) {
      const uri = typeof arg === "string" ? (arg.startsWith("file:") ? Uri.parse(arg) : Uri.file(arg)) : arg;
      if (!uri || typeof uri !== "object" || uri.scheme !== "file") {
        throw new UnsupportedCapability("vscode.workspace.openTextDocument(non-file or content)");
      }
      const key = uri.toString();
      if (state.documents.has(key)) {
        return state.documents.get(key);
      }
      const text = await fs.readFile(uri.fsPath, "utf8");
      const doc = {
        uri,
        fileName: uri.fsPath,
        languageId: LANGUAGE_BY_EXT[path.extname(uri.fsPath).toLowerCase()] ?? "plaintext",
        version: (state.documents.get(key)?.version ?? 0) + 1,
        isDirty: false,
        isUntitled: false,
        isClosed: false,
        lineCount: text === "" ? 1 : text.split(/\r\n|\r|\n/).length,
        getText(range) {
          if (range !== undefined) throw new UnsupportedCapability("vscode.TextDocument.getText(range)");
          return text;
        },
      };
      if (!state.documents.has(key)) emitter("onDidOpenTextDocument").fire(doc);
      state.documents.set(key, doc);
      return doc;
    },
    fs: strict("vscode.workspace.fs", {
      async readFile(uri) {
        return new Uint8Array(await fs.readFile(uri.fsPath));
      },
      async writeFile(uri, content) {
        await fs.writeFile(uri.fsPath, content);
      },
      async stat(uri) {
        const s = await fs.stat(uri.fsPath);
        return { type: s.isDirectory() ? 2 : 1, size: s.size, mtime: s.mtimeMs, ctime: s.ctimeMs };
      },
    }),
  });

  const window = strict("vscode.window", {
    tabGroups,
    get terminals() {
      return [...terminals];
    },
    get activeTerminal() {
      return activeTerminal;
    },
    onDidChangeActiveTerminal: emitter("window.onDidChangeActiveTerminal").event,
    onDidOpenTerminal: emitter("window.onDidOpenTerminal").event,
    onDidCloseTerminal: emitter("window.onDidCloseTerminal").event,
    onDidChangeTerminalState: emitter("window.onDidChangeTerminalState").event,
    createTerminal(nameOrOptions, shellPath, shellArgs) {
      const options =
        typeof nameOrOptions === "string"
          ? { name: nameOrOptions, shellPath, shellArgs }
          : (nameOrOptions ?? {});
      const id = `term-${++termSeq}`;
      const name = options.name || "Terminal";
      const cwd =
        options.cwd instanceof Uri
          ? options.cwd.fsPath
          : options.cwd
            ? String(options.cwd)
            : undefined;
      const args =
        typeof options.shellArgs === "string"
          ? [options.shellArgs]
          : options.shellArgs;
      notify("terminal.create", {
        id,
        name,
        shellPath: options.shellPath,
        shellArgs: args,
        cwd,
        env: options.env,
        hideFromUser: Boolean(options.hideFromUser),
      });

      let isDisposed = false;
      let isInteractedWith = false;
      let exitStatus = undefined;
      const term = {
        name,
        processId: Promise.resolve(process.pid),
        creationOptions: Object.freeze({ ...options }),
        get exitStatus() {
          return exitStatus;
        },
        get state() {
          return { isInteractedWith };
        },
        sendText(text, addNewLine = true) {
          if (isDisposed) return;
          isInteractedWith = true;
          notify("terminal.sendText", {
            id,
            text: String(text),
            addNewLine: Boolean(addNewLine),
          });
        },
        show(preserveFocus = false) {
          if (isDisposed) return;
          activeTerminal = term;
          emitter("window.onDidChangeActiveTerminal").fire(term);
          notify("terminal.show", { id, preserveFocus: Boolean(preserveFocus) });
        },
        hide() {
          if (isDisposed) return;
          notify("terminal.hide", { id });
        },
        dispose() {
          if (isDisposed) return;
          isDisposed = true;
          const idx = terminals.indexOf(term);
          if (idx !== -1) terminals.splice(idx, 1);
          if (activeTerminal === term) {
            activeTerminal = terminals[terminals.length - 1];
            emitter("window.onDidChangeActiveTerminal").fire(activeTerminal);
          }
          emitter("window.onDidCloseTerminal").fire(term);
          notify("terminal.dispose", { id });
        },
      };

      if (!options.hideFromUser) {
        terminals.push(term);
        activeTerminal = term;
        emitter("window.onDidOpenTerminal").fire(term);
        emitter("window.onDidChangeActiveTerminal").fire(term);
      }
      return term;
    },
    get activeTextEditor() {
      return state.activeEditor;
    },
    get visibleTextEditors() {
      return state.visibleEditors;
    },
    onDidChangeActiveTextEditor: emitter("onDidChangeActiveTextEditor").event,
    onDidChangeVisibleTextEditors: emitter("onDidChangeVisibleTextEditors").event,
    onDidChangeTextEditorSelection: emitter("onDidChangeTextEditorSelection").event,
    showInformationMessage(message) {
      notify("window.showInformationMessage", { message: String(message) });
      return Promise.resolve(undefined);
    },
    showErrorMessage(message) {
      notify("window.showErrorMessage", { message: String(message) });
      return Promise.resolve(undefined);
    },
    showWarningMessage(message) {
      notify("window.showWarningMessage", { message: String(message) });
      return Promise.resolve(undefined);
    },
    createStatusBarItem(a, b, c) {
      const named = typeof a === "string";
      const id = named ? a : `statusbar-${++statusSeq}`;
      const alignment = (named ? b : a) ?? 1;
      const priority = named ? c : b;
      let visible = false;
      const item = { id, alignment, priority, text: "", tooltip: undefined, command: undefined, name: undefined, color: undefined, backgroundColor: undefined };
      const push = () =>
        notify("window.statusBarItem", {
          id,
          visible,
          alignment,
          priority: priority ?? null,
          text: String(item.text ?? ""),
          tooltip: item.tooltip === undefined ? null : String(item.tooltip?.value ?? item.tooltip),
          command: typeof item.command === "string" ? item.command : (item.command?.command ?? null),
          name: item.name ?? null,
        });
      return new Proxy(item, {
        set(target, prop, value) {
          target[prop] = value;
          if (visible) push();
          return true;
        },
        get(target, prop) {
          if (prop === "show") return () => ((visible = true), push());
          if (prop === "hide") return () => ((visible = false), push());
          if (prop === "dispose") return () => notify("window.statusBarItem.dispose", { id });
          return target[prop];
        },
      });
    },
    // Plain OutputChannel, or LogOutputChannel when `{ log: true }` is passed.
    createOutputChannel(name, options) {
      const line = (text) => notify("window.outputChannel.appendLine", { name, line: String(text) });
      const channel = {
        name,
        append: (text) => notify("window.outputChannel.append", { name, text: String(text) }),
        appendLine: line,
        replace: (text) => notify("window.outputChannel.replace", { name, text: String(text) }),
        clear: () => notify("window.outputChannel.clear", { name }),
        show() {},
        hide() {},
        dispose() {},
      };
      if (options && typeof options === "object" && options.log) {
        const level = (tag) => (message, ...args) => line(`[${tag}] ${[message, ...args].map(String).join(" ")}`);
        Object.assign(channel, {
          logLevel: 2, // Info
          onDidChangeLogLevel: new EventEmitter().event,
          trace: level("trace"),
          debug: level("debug"),
          info: level("info"),
          warn: level("warn"),
          error: level("error"),
        });
      }
      return channel;
    },
  });

  // Browser opening requires an explicit decision from the native IDE; clipboard remains unsupported.
  // appName is honest: this is not VS Code (PRD 3 #6).
  const env = strict("vscode.env", {
    openExternal,
    appName: "Nanti",
    appRoot: process.cwd(),
    language: "en",
    uriScheme: "nanti",
    uiKind: 1,
    remoteName: undefined,
    shell: process.env.ComSpec ?? process.env.SHELL ?? "",
    machineId: crypto.createHash("sha256").update(os.hostname()).digest("hex"),
    sessionId: crypto.randomUUID(),
    isTelemetryEnabled: false,
    isNewAppInstall: false,
    onDidChangeTelemetryEnabled: new EventEmitter().event,
  });

  const toApiExtension = (e) => ({
    id: e.id,
    extensionPath: e.dir,
    extensionUri: Uri.file(e.dir),
    isActive: e.state === "activated",
    packageJSON: e.manifest,
    exports: e.exports,
    extensionKind: 1,
    activate: async () => e.exports,
  });
  const extensions = strict("vscode.extensions", {
    getExtension: (id) => {
      const e = getExtensions().find((x) => x.id.toLowerCase() === String(id).toLowerCase());
      return e ? toApiExtension(e) : undefined;
    },
    get all() {
      return getExtensions().map(toApiExtension);
    },
    onDidChange: emitter("extensions.onDidChange").event,
  });

  const impl = {
    Disposable,
    extensions,
    Uri,
    EventEmitter,
    ExtensionMode: EXTENSION_MODE,
    ConfigurationTarget: CONFIGURATION_TARGET,
    StatusBarAlignment: Object.freeze({ Left: 1, Right: 2 }),
    TerminalLocation,
    TerminalExitReason,
    RelativePattern,
    // Honest: the host's own version, NOT a VS Code version (PRD 5.4: no spoofed API version).
    // Extensions that gate on `vscode.version` will see an old/unknown editor and degrade or fail;
    // whether to declare a compat level is an open ADR (see api-compatibility-matrix).
    version: HOST_VERSION,
    NotebookCellOutputItem,
    commands,
    workspace,
    window,
    env,
  };
  for (const name of UNSUPPORTED_TOP_LEVEL) {
    if (name in impl) continue;
    Object.defineProperty(impl, name, {
      enumerable: true,
      configurable: true,
      get: () => unsupported(`vscode.${name}`),
    });
  }
  // Base `window` also carries owner-less webview methods (default roots = workspace folders only).
  Object.assign(window, hub.forOwner(null));
  const vscode = strict("vscode", impl);

  // Per-extension facade: only `window.*` registrations that need an owner are overridden.
  const norm = (p) => (process.platform === "win32" ? p.toLowerCase() : p);
  const facades = new Map();
  function forFile(filename) {
    if (!filename) return vscode;
    const f = norm(filename);
    const dir = [...extensionDirs].find((d) => f.startsWith(norm(d) + path.sep));
    if (!dir) return vscode;
    if (!facades.has(dir)) {
      const owned = {
        ...hub.forOwner(dir),
        registerUriHandler(handler) {
          if (uriHandlers.has(dir)) throw new Error("a URI handler is already registered for this extension");
          uriHandlers.set(dir, handler);
          return new Disposable(() => uriHandlers.delete(dir));
        },
      };
      const win = new Proxy(window, {
        get: (t, p, r) => (p in owned ? owned[p] : Reflect.get(t, p, r)),
        has: (t, p) => p in owned || p in t,
      });
      facades.set(dir, new Proxy(vscode, { get: (t, p, r) => (p === "window" ? win : Reflect.get(t, p, r)) }));
    }
    return facades.get(dir);
  }

  return {
    vscode,
    forFile,
    registerExtensionDir: (dir) => extensionDirs.add(path.resolve(dir)),
    commandTable,
    config,
    providers,
    hub,
    uriHandlers,
    contextKeys,
    resolveExternal,
    syncOpenDocument,
    syncChangeDocument,
    syncSaveDocument,
    syncCloseDocument,
  };
}
