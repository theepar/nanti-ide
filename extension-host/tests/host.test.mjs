import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import fs from "node:fs";
import os from "node:os";
import { fileURLToPath } from "node:url";
import { createHost, PROTOCOL_VERSION } from "../src/host.mjs";

const MOCK = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../tests/fixtures/mock-extension");

const req = (requestId, method, params = {}) => ({ protocolVersion: PROTOCOL_VERSION, requestId, method, params });

async function setup(workspaceFolders = []) {
  const notes = [];
  const host = createHost({ notify: (method, params) => notes.push({ method, params }) });
  await host.handle(req(0, "initialize", { workspaceFolders }));
  return { host, notes };
}

test("initialize reports protocol version", async () => {
  const host = createHost();
  const r = await host.handle(req(1, "initialize"));
  assert.equal(r.result.protocolVersion, PROTOCOL_VERSION);
});

test("unknown method yields UnsupportedCapability", async () => {
  const { host } = await setup();
  const r = await host.handle(req(2, "x"));
  assert.equal(r.error.code, "UnsupportedCapability");
});

test("version mismatch is rejected", async () => {
  const { host } = await setup();
  const r = await host.handle({ protocolVersion: 99, requestId: 3, method: "ping", params: {} });
  assert.equal(r.error.code, "ProtocolMismatch");
});

test("load -> activate -> execute command (VH-01, VH-02)", async () => {
  const { host, notes } = await setup();
  const loaded = await host.handle(req(1, "extension.load", { path: MOCK }));
  assert.equal(loaded.result.id, "nanti-test.mock");
  assert.equal(loaded.result.state, "installed");

  const act = await host.handle(req(2, "extension.activate", { id: "nanti-test.mock" }));
  assert.equal(act.result.state, "activated");

  const r = await host.handle(req(3, "commands.execute", { command: "mock.hello", args: ["nanti"] }));
  assert.equal(r.result, "hello nanti");
  assert.deepEqual(notes, [{ method: "window.showInformationMessage", params: { message: "hello nanti" } }]);
});

test("workspace.fs.readFile via extension (VH-03)", async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "nanti-ws-"));
  fs.writeFileSync(path.join(dir, "a.txt"), "héllo");
  const { host } = await setup([dir]);
  await host.handle(req(1, "extension.load", { path: MOCK }));
  await host.handle(req(2, "extension.activate", { id: "nanti-test.mock" }));
  const r = await host.handle(req(3, "commands.execute", { command: "mock.readFile", args: ["a.txt"] }));
  assert.equal(r.result, "héllo");
  fs.rmSync(dir, { recursive: true });
});

test("unsupported vscode API fails loudly, not silently (PRD 5.4)", async () => {
  const { host } = await setup();
  await host.handle(req(1, "extension.load", { path: MOCK }));
  await host.handle(req(2, "extension.activate", { id: "nanti-test.mock" }));
  const r = await host.handle(req(3, "commands.execute", { command: "mock.unsupported" }));
  assert.equal(r.error.code, "UnsupportedCapability");
  assert.match(r.error.message, /vscode\.debug\.startDebugging/);
});

test("deactivate disposes commands", async () => {
  const { host } = await setup();
  await host.handle(req(1, "extension.load", { path: MOCK }));
  await host.handle(req(2, "extension.activate", { id: "nanti-test.mock" }));
  const d = await host.handle(req(3, "extension.deactivate", { id: "nanti-test.mock" }));
  assert.equal(d.result.state, "installed");
  const r = await host.handle(req(4, "commands.execute", { command: "mock.hello", args: ["x"] }));
  assert.match(r.error.message, /not found/);
});

test("activation failure is reported as failed, not activated", async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "nanti-bad-"));
  fs.writeFileSync(
    path.join(dir, "package.json"),
    JSON.stringify({ name: "bad", publisher: "t", main: "./e.js" }),
  );
  fs.writeFileSync(path.join(dir, "e.js"), "throw new Error('boom');");
  const { host } = await setup();
  await host.handle(req(1, "extension.load", { path: dir }));
  const a = await host.handle(req(2, "extension.activate", { id: "t.bad" }));
  assert.equal(a.error.message, "boom");
  const s = await host.handle(req(3, "extension.state", { id: "t.bad" }));
  assert.equal(s.result.state, "failed");
  fs.rmSync(dir, { recursive: true });
});

test("unsupported top-level names are own keys so bundler interop does not see undefined", async () => {
  const { createVscode } = await import("../src/vscode-api.mjs");
  const { vscode } = createVscode({ notify() {} });
  const keys = Object.getOwnPropertyNames(vscode);
  for (const k of ["Hover", "TreeItem", "Range", "debug"]) {
    assert.ok(keys.includes(k), k);
  }
  assert.throws(() => vscode.Hover.x(), /UnsupportedCapability/);
});

test("ExtensionContext provides storage URIs, mode, and persistent Memento", async () => {
  const { createContext } = await import("../src/context.mjs");
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "nanti-ctx-"));
  const ws = path.join(root, "proj");
  const mk = () => createContext({ dir: MOCK, id: "Pub.Name" }, { storageRoot: root, workspaceFolders: [ws] });
  const c = mk();
  assert.equal(c.extensionMode, 1);
  assert.ok(c.globalStorageUri.fsPath.startsWith(root));
  assert.ok(c.storageUri.fsPath.startsWith(root));
  assert.equal(c.extensionUri.fsPath, path.resolve(MOCK));
  assert.equal(c.globalState.get("k", 7), 7);
  await c.globalState.update("k", { a: 1 });
  assert.deepEqual(mk().globalState.get("k"), { a: 1 }); // survives a new context (restart)
  await c.globalState.update("k", undefined);
  assert.deepEqual(mk().globalState.keys(), []);
  fs.rmSync(root, { recursive: true });

});

test("secrets stay unsupported until backed by the OS credential store (PRD 7.2)", async () => {
  const { createContext } = await import("../src/context.mjs");
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "nanti-ctx-"));
  const c = createContext({ dir: MOCK, id: "p.n" }, { storageRoot: root, workspaceFolders: [] });
  assert.throws(() => c.secrets.get("token"), /UnsupportedCapability.*secrets/);
  assert.equal(c.storageUri, undefined);
  fs.rmSync(root, { recursive: true });

});

test("basic classes: EventEmitter, Uri, ExtensionMode, NotebookCellOutputItem, env constants", async () => {
  const { createVscode } = await import("../src/vscode-api.mjs");
  const { vscode } = createVscode({ notify() {} });
  const em = new vscode.EventEmitter();
  const seen = [];
  const sub = em.event((v) => seen.push(v));
  em.fire(1);
  sub.dispose();
  em.fire(2);
  assert.deepEqual(seen, [1]);

  const base = vscode.Uri.file(path.resolve(MOCK));
  assert.equal(vscode.Uri.joinPath(base, "extension.js").fsPath, path.join(path.resolve(MOCK), "extension.js"));
  assert.equal(vscode.Uri.parse("https://example.com/a?b=1#c").query, "b=1");
  assert.equal(vscode.ExtensionMode.Production, 1);
  const item = vscode.NotebookCellOutputItem.error(new Error("x"));
  assert.equal(item.mime, "application/vnd.code.notebook.error");
  assert.equal(vscode.env.appName, "Nanti");
  assert.equal(vscode.env.remoteName, undefined);
  assert.equal(await vscode.env.openExternal(), false);
});

test("env.openExternal requires consent and only accepts safe web URLs", async () => {
  const { createVscode } = await import("../src/vscode-api.mjs");
  const notes = [];
  const api = createVscode({ notify: (method, params) => notes.push({ method, params }) });
  const { vscode } = api;
  for (const url of ["javascript:alert(1)", "file:///C:/secret", "https://user:pass@example.com/", "https://example.com/\nmalicious"]) {
    assert.equal(await vscode.env.openExternal(url), false);
  }
  assert.equal(notes.length, 0);
  const allowed = vscode.env.openExternal(vscode.Uri.parse("https://example.com/login?state=abc"));
  assert.equal(notes[0].method, "env.openExternal");
  assert.equal(notes[0].params.url, "https://example.com/login?state=abc");
  assert.equal(api.resolveExternal(notes[0].params.id, true), true);
  assert.equal(await allowed, true);
  assert.equal(api.resolveExternal(notes[0].params.id, true), false);
  const denied = vscode.env.openExternal("http://example.com/");
  assert.equal(api.resolveExternal(notes[1].params.id, false), true);
  assert.equal(await denied, false);
});

test("host dispatches env.openExternal consent replies while caller awaits", async () => {
  const { createVscode } = await import("../src/vscode-api.mjs");
  const notes = [];
  const api = createVscode({ notify: (method, params) => notes.push({ method, params }) });
  const waiting = api.vscode.env.openExternal("https://example.com/");
  const host = createHost();
  await host.handle(req(0, "initialize"));
  const unknown = await host.handle(req(1, "env.openExternal.resolve", { id: "missing", approved: true }));
  assert.equal(unknown.result.resolved, false);
  // A direct host request tests the response contract; the actual extension uses its own API instance.
  assert.equal(api.resolveExternal(notes[0].params.id, false), true);
  assert.equal(await waiting, false);
});

test("createOutputChannel({log:true}) returns a LogOutputChannel", async () => {
  const { createVscode } = await import("../src/vscode-api.mjs");
  const notes = [];
  const { vscode } = createVscode({ notify: (m, p) => notes.push([m, p]) });
  const plain = vscode.window.createOutputChannel("p");
  assert.equal(plain.info, undefined);
  const log = vscode.window.createOutputChannel("l", { log: true });
  log.info("hi", 1);
  log.error("bad");
  assert.deepEqual(notes.map(([, p]) => p.line), ["[info] hi 1", "[error] bad"]);
});

test("workspace.getConfiguration: defaults from manifest, precedence, update + events (VH-09)", async () => {
  const notes = [];
  const host = createHost({ notify: (method, params) => notes.push({ method, params }) });
  await host.handle(req(0, "initialize", { settings: { global: { "ext.a": 1 }, workspace: { "ext.b": 2 } } }));
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "nanti-cfg-"));
  fs.writeFileSync(
    path.join(dir, "package.json"),
    JSON.stringify({
      name: "cfg", publisher: "t", main: "./e.js",
      contributes: { configuration: { properties: { "ext.a": { default: 10 }, "ext.c": { default: "dflt" }, "ext.nested.x": { default: true } } } },
    }),
  );
  fs.writeFileSync(
    path.join(dir, "e.js"),
    `const vscode = require("vscode");
     exports.activate = (ctx) => {
       globalThis.__cfg = vscode.workspace.getConfiguration("ext");
       globalThis.__events = [];
       ctx.subscriptions.push(vscode.workspace.onDidChangeConfiguration((e) => globalThis.__events.push(e.affectsConfiguration("ext.c"))));
     };`,
  );
  await host.handle(req(1, "extension.load", { path: dir }));
  const a = await host.handle(req(2, "extension.activate", { id: "t.cfg" }));
  assert.equal(a.result.state, "activated");
  const cfg = globalThis.__cfg;
  assert.equal(cfg.get("a"), 1); // global overrides manifest default
  assert.equal(cfg.get("b"), 2);
  assert.equal(cfg.get("c"), "dflt");
  assert.equal(cfg.get("missing", "fallback"), "fallback");
  assert.deepEqual(cfg.get("nested"), { x: true });
  assert.deepEqual(cfg.inspect("a"), { key: "ext.a", defaultValue: 10, globalValue: 1, workspaceValue: undefined });
  await cfg.update("c", "new", 2); // Workspace
  assert.equal(cfg.get("c"), "new");
  assert.deepEqual(globalThis.__events, [true]);
  assert.deepEqual(notes.at(-1), { method: "workspace.configuration.update", params: { key: "ext.c", value: "new", scope: "workspace" } });
  await cfg.update("c", undefined, 2);
  assert.equal(cfg.get("c"), "dflt");
  fs.rmSync(dir, { recursive: true });
});

test("tabGroups, editor state, providers, openTextDocument (VH-04 partial)", async () => {
  const { createVscode } = await import("../src/vscode-api.mjs");
  const { vscode } = createVscode({ notify() {} });
  assert.deepEqual([...vscode.window.tabGroups.all].map((g) => g.tabs), [[]]);
  assert.equal(vscode.window.activeTextEditor, undefined);
  assert.deepEqual(vscode.window.visibleTextEditors, []);
  assert.deepEqual(vscode.workspace.textDocuments, []);
  assert.equal(vscode.ConfigurationTarget.Workspace, 2);

  const d = vscode.workspace.registerFileSystemProvider("nanti-x", {});
  assert.throws(() => vscode.workspace.registerFileSystemProvider("nanti-x", {}), /already registered/);
  d.dispose();
  vscode.workspace.registerFileSystemProvider("nanti-x", {}).dispose();

  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "nanti-doc-"));
  const file = path.join(dir, "a.ts");
  fs.writeFileSync(file, "one\ntwo\r\nthree");
  const opened = [];
  vscode.workspace.onDidOpenTextDocument((x) => opened.push(x.fileName));
  const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(file));
  assert.equal(doc.languageId, "typescript");
  assert.equal(doc.lineCount, 3);
  assert.equal(doc.getText(), "one\ntwo\r\nthree");
  assert.deepEqual(opened, [file]);
  assert.equal(vscode.workspace.textDocuments.length, 1);
  assert.throws(() => doc.getText({}), /UnsupportedCapability/);
  await assert.rejects(vscode.workspace.openTextDocument(vscode.Uri.parse("untitled:x")), /UnsupportedCapability/);
  fs.rmSync(dir, { recursive: true });
});

const MOCKWEB = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../tests/fixtures/mock-webview-extension");

async function setupWeb(workspaceFolders = []) {
  const notes = [];
  const host = createHost({ notify: (method, params) => notes.push({ method, params }) });
  await host.handle(req(0, "initialize", { workspaceFolders }));
  await host.handle(req(1, "extension.load", { path: MOCKWEB }));
  const a = await host.handle(req(2, "extension.activate", { id: "nanti-test.mockweb" }));
  assert.equal(a.result.state, "activated");
  return { host, notes };
}

test("webview view: register, resolve, html, resource URI, two-way messages (VH-06)", async () => {
  const { host, notes } = await setupWeb();
  assert.deepEqual(notes.filter((n) => n.method === "webview.viewProviderRegistered").map((n) => n.params.viewType), ["mockweb.view"]);
  notes.length = 0;

  const r = await host.handle(req(3, "webview.resolveView", { viewType: "mockweb.view" }));
  const handle = r.result.handle;
  const html = notes.find((n) => n.method === "webview.setHtml");
  assert.equal(html.params.handle, handle);
  assert.match(html.params.html, /https:\/\/nanti-resource\.localhost\/file\/.+main\.js/);
  const opts = notes.filter((n) => n.method === "webview.setOptions").at(-1).params.options;
  assert.equal(opts.enableScripts, true);
  assert.deepEqual(opts.localResourceRoots, [path.join(MOCKWEB, "media")]); // not the whole extension dir

  notes.length = 0;
  await host.handle(req(4, "webview.message", { handle, message: { type: "ready" } }));
  assert.deepEqual(notes, [{ method: "webview.postMessage", params: { handle, message: { type: "ping", n: 1 } } }]);
  notes.length = 0;
  await host.handle(req(5, "webview.message", { handle, message: { type: "pong", n: 1 } }));
  assert.deepEqual(notes.map((n) => n.params.message), ["pong 1"]);

  const bad = await host.handle(req(6, "webview.message", { handle: "view-999", message: {} }));
  assert.equal(bad.error.code, "UnknownWebview");
  await host.handle(req(7, "webview.dispose", { handle }));
  assert.equal((await host.handle(req(8, "webview.message", { handle, message: {} }))).error.code, "UnknownWebview");
  const unknown = await host.handle(req(9, "webview.resolveView", { viewType: "nope" }));
  assert.equal(unknown.error.code, "UnknownView");
});

test("webview panel + serializer registry", async () => {
  const { createVscode } = await import("../src/vscode-api.mjs");
  const notes = [];
  const api = createVscode({ notify: (m, p) => notes.push({ method: m, params: p }) });
  const { vscode } = api;
  const panel = vscode.window.createWebviewPanel("t.p", "Title", 1, { enableScripts: true });
  assert.equal(notes[0].method, "webview.panelCreated");
  assert.equal(notes[0].params.title, "Title");
  panel.title = "New";
  panel.webview.html = "<p>x</p>";
  let disposed = 0;
  panel.onDidDispose(() => disposed++);
  panel.dispose();
  panel.dispose();
  assert.equal(disposed, 1);
  assert.deepEqual(notes.map((n) => n.method), ["webview.panelCreated", "webview.setTitle", "webview.setHtml", "webview.disposed"]);
  const d = vscode.window.registerWebviewPanelSerializer("t.p", {});
  assert.throws(() => vscode.window.registerWebviewPanelSerializer("t.p", {}), /already registered/);
  d.dispose();
});

test("URI handler routes by extension id and rejects mismatches (VH-07)", async () => {
  const { host, notes } = await setupWeb();
  notes.length = 0;
  const ok = await host.handle(req(3, "uri.handle", { uri: "nanti://nanti-test.mockweb/callback?code=abc&state=s" }));
  assert.equal(ok.error, undefined);
  assert.deepEqual(notes.map((n) => n.params.message), ["uri /callback?code=abc&state=s"]);
  const wrong = await host.handle(req(4, "uri.handle", { uri: "nanti://someone.else/callback" }));
  assert.equal(wrong.error.code, "NoUriHandler");
});

test("setContext, status bar item, extensions registry, showWarningMessage", async () => {
  const { createVscode } = await import("../src/vscode-api.mjs");
  const notes = [];
  const exts = [{ id: "Pub.Name", dir: MOCK, manifest: { name: "n" }, state: "activated", exports: { x: 1 } }];
  const { vscode } = createVscode({ notify: (m, p) => notes.push({ method: m, params: p }), getExtensions: () => exts });
  await vscode.commands.executeCommand("setContext", "k", true);
  assert.deepEqual(notes.pop(), { method: "context.set", params: { key: "k", value: true } });

  const item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 5);
  item.text = "before show"; // not visible yet -> no notification
  assert.equal(notes.length, 0);
  item.show();
  item.text = "hi";
  item.command = "x.y";
  assert.deepEqual(notes.at(-1).params, { id: item.id, visible: true, alignment: 2, priority: 5, text: "hi", tooltip: null, command: "x.y", name: null });

  assert.equal(vscode.extensions.getExtension("pub.name").exports.x, 1);
  assert.equal(vscode.extensions.getExtension("pub.name").isActive, true);
  assert.equal(vscode.extensions.getExtension("nope"), undefined);
  assert.equal(vscode.extensions.all.length, 1);
  await vscode.window.showWarningMessage("w");
  assert.equal(notes.at(-1).method, "window.showWarningMessage");
});

test("glob matching and file watcher events", async () => {
  const { globToRegExp, createVscode } = await import("../src/vscode-api.mjs");
  assert.ok(globToRegExp("**/*.ts").test("a.ts"));
  assert.ok(globToRegExp("**/*.ts").test("src/deep/a.ts"));
  assert.ok(!globToRegExp("**/*.ts").test("a.js"));
  assert.ok(globToRegExp("*.{ts,js}").test("a.js"));
  assert.ok(!globToRegExp("*.ts").test("d/a.ts"));

  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "nanti-watch-"));
  const { vscode } = createVscode({ notify() {}, workspaceFolders: [dir] });
  const seen = [];
  const w = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(dir, "**/*.txt"));
  w.onDidCreate((u) => seen.push(["create", path.basename(u.fsPath)]));
  w.onDidChange((u) => seen.push(["change", path.basename(u.fsPath)]));
  await new Promise((r) => setTimeout(r, 200));
  fs.writeFileSync(path.join(dir, "a.txt"), "1");
  fs.writeFileSync(path.join(dir, "ignored.md"), "1");
  await new Promise((r) => setTimeout(r, 600));
  w.dispose();
  assert.ok(seen.some(([k, n]) => k === "create" && n === "a.txt"), JSON.stringify(seen));
  assert.ok(!seen.some(([, n]) => n === "ignored.md"));
  fs.rmSync(dir, { recursive: true, force: true });
});

test("context.extension exposes the extension's own id, manifest and live state", async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "nanti-self-"));
  fs.writeFileSync(path.join(dir, "package.json"), JSON.stringify({ name: "selfy", publisher: "t", main: "./e.js", marker: 7 }));
  fs.writeFileSync(
    path.join(dir, "e.js"),
    `exports.activate = (ctx) => { globalThis.__ext = ctx.extension; return { api: 1 }; };`,
  );
  const { host } = await setup();
  await host.handle(req(1, "extension.load", { path: dir }));
  assert.equal((await host.handle(req(2, "extension.activate", { id: "t.selfy" }))).result.state, "activated");
  const self = globalThis.__ext;
  assert.equal(self.id, "t.selfy");
  assert.equal(self.packageJSON.marker, 7);
  assert.equal(self.isActive, true);
  assert.deepEqual(self.exports, { api: 1 });
  fs.rmSync(dir, { recursive: true });
});

test("environmentVariableCollection records mutations (not applied to terminals yet)", async () => {
  const { createContext } = await import("../src/context.mjs");
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "nanti-env-"));
  const c = createContext({ dir: MOCK, id: "p.n" }, { storageRoot: root, workspaceFolders: [] });
  const coll = c.environmentVariableCollection;
  coll.replace("A", "1");
  coll.append("B", "2");
  coll.prepend("C", "3");
  assert.deepEqual(coll.get("A").value, "1");
  assert.equal(coll.get("B").type, 2);
  const seen = [];
  coll.forEach((name, m) => seen.push(`${name}=${m.value}`));
  assert.deepEqual(seen, ["A=1", "B=2", "C=3"]);
  coll.delete("A");
  assert.equal(coll.get("A"), undefined);
  assert.equal(coll.getScoped({}), coll);
  fs.rmSync(root, { recursive: true });

});


test("window.createTerminal and terminal operations (TR-04 / VH-10)", async () => {
  const { createVscode } = await import("../src/vscode-api.mjs");
  const notes = [];
  const { vscode } = createVscode({ notify: (method, params) => notes.push({ method, params }) });

  const opened = [];
  const closed = [];
  vscode.window.onDidOpenTerminal((t) => opened.push(t.name));
  vscode.window.onDidCloseTerminal((t) => closed.push(t.name));

  const term = vscode.window.createTerminal("build-term");
  assert.equal(term.name, "build-term");
  assert.equal(vscode.window.terminals.length, 1);
  assert.equal(vscode.window.activeTerminal, term);
  assert.deepEqual(opened, ["build-term"]);

  term.sendText("cargo build", true);
  term.show();
  term.dispose();

  assert.equal(vscode.window.terminals.length, 0);
  assert.deepEqual(closed, ["build-term"]);

  const createNote = notes.find((n) => n.method === "terminal.create");
  assert.ok(createNote);
  assert.equal(createNote.params.name, "build-term");

  const sendNote = notes.find((n) => n.method === "terminal.sendText");
  assert.ok(sendNote);
  assert.equal(sendNote.params.text, "cargo build");
  assert.equal(sendNote.params.addNewLine, true);

  const showNote = notes.find((n) => n.method === "terminal.show");
  assert.ok(showNote);

  const disposeNote = notes.find((n) => n.method === "terminal.dispose");
  assert.ok(disposeNote);
});

test("live editor buffer sync between IDE and host (AI-04 / AI-05 / VH-04)", async () => {
  const { host } = await setup();
  const fileUri = "file:///C:/workspace/test.rs";

  // IDE opens document with unsaved live content
  await host.handle(req(1, "workspace.didOpenTextDocument", {
    uri: fileUri,
    languageId: "rust",
    version: 1,
    text: 'fn main() { println!("live"); }',
  }));

  // Direct host handles
  const { createVscode } = await import("../src/vscode-api.mjs");
  const api = createVscode({ notify() {} });
  api.syncOpenDocument({
    uri: fileUri,
    languageId: "rust",
    version: 1,
    text: 'fn main() { println!("live"); }',
  });
  const doc = await api.vscode.workspace.openTextDocument(fileUri);
  assert.equal(doc.getText(), 'fn main() { println!("live"); }');
  assert.equal(doc.isDirty, false);

  api.syncChangeDocument({
    uri: fileUri,
    version: 2,
    text: 'fn main() { println!("edited"); }',
  });
  assert.equal(doc.getText(), 'fn main() { println!("edited"); }');
  assert.equal(doc.isDirty, true);

  api.syncSaveDocument({ uri: fileUri });
  assert.equal(doc.isDirty, false);

  api.syncCloseDocument({ uri: fileUri });
  assert.equal(doc.isClosed, true);
  assert.equal(api.vscode.workspace.textDocuments.length, 0);

});
