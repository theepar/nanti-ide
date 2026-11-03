// A static estimate of how well real extensions fit the nanti extension host. It downloads each extension from
// Open VSX, READS its JavaScript (nothing of it is ever run), finds the `vscode.*` APIs it mentions and checks
// each one against what the host provides. A mention is not a call, so this is an estimate, not a guarantee;
// compat-report.mjs actually activates the extensions.
//
// usage: node scripts/compat-static.mjs [--work=<dir>] [--json=<file>] [--list=<file>] publisher.name ...
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { fetchExtension } from "./lib/openvsx.mjs";
import { createVscode, setDiscoveryCollector } from "../extension-host/src/vscode-api.mjs";

const NAMESPACES = ["window", "workspace", "commands", "languages", "env", "extensions", "debug", "tasks", "scm", "authentication", "tests", "notebooks", "chat", "lm", "l10n"];
// The classes and enums at the top of the VS Code API; other capitalised names belong to the extension itself.
const TOP_LEVEL = new Set(
  (
    "Position Range Selection Location LocationLink Uri EventEmitter Disposable TextEdit WorkspaceEdit SnippetTextEdit SnippetString " +
    "Diagnostic DiagnosticSeverity DiagnosticTag DiagnosticRelatedInformation ThemeColor ThemeIcon MarkdownString TreeItem TreeItemCollapsibleState " +
    "StatusBarAlignment ViewColumn ProgressLocation ConfigurationTarget CancellationTokenSource CancellationError FileType FileSystemError " +
    "FileChangeType EndOfLine TextEditorRevealType TextEditorSelectionChangeKind TextEditorCursorStyle TextEditorLineNumbersStyle " +
    "DecorationRangeBehavior OverviewRulerLane CompletionItem CompletionItemKind CompletionList CompletionItemTag CompletionTriggerKind " +
    "Hover CodeLens CodeAction CodeActionKind CodeActionTriggerKind DocumentLink DocumentSymbol SymbolKind SymbolInformation SymbolTag " +
    "SignatureHelp SignatureInformation ParameterInformation InlayHint InlayHintKind FoldingRange FoldingRangeKind Color ColorInformation " +
    "ColorPresentation SelectionRange SemanticTokens SemanticTokensBuilder SemanticTokensLegend DocumentHighlight DocumentHighlightKind " +
    "CallHierarchyItem TypeHierarchyItem EvaluatableExpression InlineValueText InlineValueVariableLookup InlineValueEvaluatableExpression " +
    "InlineCompletionItem InlineCompletionList InlineCompletionTriggerKind ExtensionMode ExtensionKind ColorThemeKind QuickPickItemKind " +
    "QuickInputButtons InputBoxValidationSeverity RelativePattern Task TaskGroup TaskScope TaskPanelKind TaskRevealKind ShellExecution " +
    "ProcessExecution CustomExecution ShellQuoting DebugAdapterExecutable DebugAdapterServer DebugAdapterNamedPipeServer " +
    "DebugAdapterInlineImplementation DebugConfigurationProviderTriggerKind SourceBreakpoint FunctionBreakpoint Breakpoint " +
    "NotebookCellKind NotebookCellData NotebookData NotebookRange NotebookEdit NotebookCellOutput NotebookCellOutputItem " +
    "NotebookControllerAffinity NotebookEditorRevealType CommentMode CommentThreadCollapsibleState CommentThreadState " +
    "TabInputText TabInputTextDiff TabInputCustom TabInputWebview TabInputNotebook TabInputTerminal TerminalLocation TerminalExitReason " +
    "TerminalLink TerminalProfile TestRunProfileKind TestRunRequest TestMessage TestTag LanguageStatusSeverity LogLevel " +
    "UIKind ExternalUriOpenerPriority DataTransfer DataTransferItem DocumentDropEdit DocumentPasteEdit " +
    "LanguageModelChatMessage LanguageModelChatMessageRole LanguageModelTextPart LanguageModelToolResult LanguageModelError"
  ).split(" "),
);
const IDENT = "[A-Za-z_$][\\w$]*";
const escapeRe = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** The `vscode.*` APIs a piece of JavaScript mentions, as "namespace.member" or a top-level name. */
export function apisIn(text) {
  const used = new Set();
  const aliases = new Set(["vscode"]);
  // var o = require("vscode"), var o = __toESM(require("vscode")), import * as o from "vscode", ...
  for (const m of text.matchAll(new RegExp(`(${IDENT})\\s*=\\s*(?:[\\w$.]+\\(\\s*)?require\\(\\s*["']vscode["']\\s*\\)`, "g"))) aliases.add(m[1]);
  for (const m of text.matchAll(new RegExp(`import\\s+(?:\\*\\s+as\\s+)?(${IDENT})\\s+from\\s*["']vscode["']`, "g"))) aliases.add(m[1]);
  // webpack: a module that only re-exports require("vscode"), and the variables that load that module by its id
  for (const m of text.matchAll(/(\d+|"[^"]+")\s*:\s*[^{}]{0,60}\{[^{}]{0,60}exports\s*=\s*require\(\s*["']vscode["']\s*\)/g)) {
    const key = escapeRe(m[1]);
    for (const u of text.matchAll(new RegExp(`(${IDENT})\\s*=\\s*(?:\\(\\s*0\\s*,\\s*)?[\\w$.]+\\(\\s*${key}\\s*\\)`, "g"))) aliases.add(u[1]);
  }
  // Names a bundler gives the module itself (not packages that merely have "vscode" in their name)
  for (const m of text.matchAll(/\b((?:import_|_)?vscode\d*|vscode__WEBPACK_IMPORTED_MODULE_\d+__)\.[A-Za-z_]/g)) aliases.add(m[1]);

  const note = (first, second) => {
    if (NAMESPACES.includes(first)) {
      if (second) used.add(`${first}.${second}`);
    } else if (TOP_LEVEL.has(first)) used.add(first);
  };
  for (const alias of aliases) {
    const re = new RegExp(`(?<![\\w$.])${escapeRe(alias)}\\.([A-Za-z_]\\w*)(?:\\.([A-Za-z_]\\w*))?`, "g");
    for (const m of text.matchAll(re)) note(m[1], m[2]);
  }
  // const { window, workspace, Range } = require("vscode")
  for (const m of text.matchAll(/\{([^{}]*)\}\s*=\s*(?:__toESM\()?\(?require\(["']vscode["']\)/g)) {
    for (const n of m[1].split(",").map((x) => x.split(":").pop().trim()).filter(Boolean)) {
      if (TOP_LEVEL.has(n)) used.add(n);
      else if (NAMESPACES.includes(n)) {
        for (const use of text.matchAll(new RegExp(`(?<![\\w$.])${n}\\.([A-Za-z_]\\w*)`, "g"))) used.add(`${n}.${use[1]}`);
      }
    }
  }
  return used;
}

function* jsFiles(dir, depth = 0) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (e.name === "node_modules" && depth > 0) continue;
    const p = path.join(dir, e.name);
    if (e.isDirectory()) {
      if (depth < 6) yield* jsFiles(p, depth + 1);
    } else if (/\.(c|m)?js$/.test(e.name) && fs.statSync(p).size < 12 * 1024 * 1024) yield p;
  }
}

async function main() {
  const args = process.argv.slice(2);
  const opt = (name, dflt) => args.find((a) => a.startsWith(`--${name}=`))?.split("=").slice(1).join("=") ?? dflt;
  const listFile = opt("list", "");
  const ids = [
    ...args.filter((a) => !a.startsWith("--")),
    ...(listFile ? fs.readFileSync(path.resolve(listFile), "utf8").split(/\r?\n/).map((l) => l.replace(/#.*/, "").trim()).filter(Boolean) : []),
  ];
  const work = path.resolve(opt("work", path.join(os.tmpdir(), "nanti-compat")));
  const jsonOut = opt("json", "");
  if (ids.length === 0) {
    console.error("usage: compat-static.mjs [--work=dir] [--json=file] [--list=file] publisher.name ...");
    process.exit(2);
  }
  fs.mkdirSync(work, { recursive: true });

  // What the host provides: asking for an API in discovery mode records the ones it does not have.
  const missing = new Set();
  setDiscoveryCollector(missing);
  const origError = console.error;
  console.error = () => {};
  const { vscode } = createVscode({ notify() {} });
  const provided = (namespace, member) => {
    const before = missing.size;
    try {
      const target = namespace ? vscode[namespace] : vscode;
      target?.[member]?.__nantiProbe;
    } catch {
      return false;
    }
    return missing.size === before;
  };

  const rows = [];
  for (const id of ids) {
    const got = await fetchExtension(id, work);
    if (got.error) {
      rows.push({ id, status: "SKIP", note: got.error });
      origError(`${id}: ${got.error}`);
      continue;
    }
    let manifest;
    try {
      manifest = JSON.parse(fs.readFileSync(path.join(got.dir, "package.json"), "utf8"));
    } catch {
      rows.push({ id, status: "SKIP", note: "no readable package.json" });
      continue;
    }
    if (!manifest.main) {
      const kinds = Object.keys(manifest.contributes ?? {}).join(", ") || "nothing";
      rows.push({ id, version: got.version, status: "DECLARATIVE", note: kinds });
      continue;
    }
    const used = new Set();
    for (const file of jsFiles(got.dir)) for (const api of apisIn(fs.readFileSync(file, "utf8"))) used.add(api);
    const apis = [...used].sort();
    const gaps = apis.filter((a) => {
      const [first, second] = a.split(".");
      return second ? !provided(first, second) : !provided(undefined, first);
    });
    rows.push({ id, version: got.version, status: gaps.length === 0 ? "COVERED" : "GAPS", used: apis.length, gaps });
  }
  console.error = origError;

  for (const r of rows) {
    const detail = r.status === "COVERED" || r.status === "GAPS" ? `${r.used} APIs used, ${r.gaps.length} missing` : r.note ?? "";
    console.log(`${r.status.padEnd(11)} ${(r.id + "@" + (r.version ?? "?")).padEnd(52)} ${detail}`);
  }
  const count = (s) => rows.filter((r) => r.status === s).length;
  console.log(`\n${rows.length} extensions: ${count("COVERED")} covered, ${count("GAPS")} with gaps, ${count("DECLARATIVE")} declarative only, ${count("SKIP")} skipped`);
  const tally = new Map();
  for (const r of rows) for (const g of r.gaps ?? []) tally.set(g, (tally.get(g) ?? 0) + 1);
  const top = [...tally].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).slice(0, 45);
  if (top.length) {
    console.log("\nMissing APIs, by how many extensions mention them:");
    for (const [api, n] of top) console.log(`  ${String(n).padStart(3)}  ${api}`);
  }
  if (jsonOut) fs.writeFileSync(jsonOut, JSON.stringify({ rows, missing: Object.fromEntries(tally) }, null, 2));
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) await main();
