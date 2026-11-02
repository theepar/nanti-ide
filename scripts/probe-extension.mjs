// Compatibility probe: load + activate a real extracted extension in the host (in-process).
// Usage: node scripts/probe-extension.mjs <extracted-extension-dir> [--discover] [--timeout=ms]
//   default    strict mode: reports the first unsupported API (what a user would hit)
//   --discover collect every unsupported API reached during activation (inventory only)
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { createHost, PROTOCOL_VERSION } from "../extension-host/src/host.mjs";
import { setDiscoveryCollector } from "../extension-host/src/vscode-api.mjs";

const args = process.argv.slice(2);
const dir = args.find((a) => !a.startsWith("--"));
const discover = args.includes("--discover");
const timeoutMs = Number(args.find((a) => a.startsWith("--timeout="))?.split("=")[1] ?? 30000);
if (!dir) {
  console.error("usage: probe-extension.mjs <dir> [--discover] [--timeout=ms]");
  process.exit(2);
}

// Isolate the vendor code from the real user profile: extensions write to ~/.claude, %APPDATA% etc.
// (Claude Code, for one, drops an IDE lock file under ~/.claude/ide that other CLI sessions may pick up).
const sandboxHome = fs.mkdtempSync(path.join(os.tmpdir(), "nanti-probe-home-"));
for (const [k, sub] of [["USERPROFILE", ""], ["HOME", ""], ["APPDATA", "AppData/Roaming"], ["LOCALAPPDATA", "AppData/Local"]]) {
  const p = path.join(sandboxHome, sub);
  fs.mkdirSync(p, { recursive: true });
  process.env[k] = p;
}

const unsupported = new Set();
if (discover) setDiscoveryCollector(unsupported);

const ws = fs.mkdtempSync(path.join(os.tmpdir(), "nanti-probe-ws-"));
fs.writeFileSync(path.join(ws, "hello.txt"), "probe workspace\n");

const notes = [];
const host = createHost({ notify: (method, params) => notes.push({ method, params }) });
const req = (requestId, method, params = {}) => ({ protocolVersion: PROTOCOL_VERSION, requestId, method, params });

await host.handle(req(1, "initialize", { workspaceFolders: [ws] }));
const loaded = await host.handle(req(2, "extension.load", { path: dir }));
if (loaded.error) {
  console.log(JSON.stringify({ stage: "load", error: loaded.error }));
  process.exit(1);
}

const timer = new Promise((r) => setTimeout(() => r({ error: { code: "Timeout", message: `no result in ${timeoutMs}ms` } }), timeoutMs));
const activated = await Promise.race([host.handle(req(3, "extension.activate", { id: loaded.result.id })), timer]);
const state = await host.handle(req(4, "extension.state", { id: loaded.result.id }));

// Optional: resolve a webview view (AI-08) and summarise what the extension rendered.
let view = null;
const resolveArg = args.find((a) => a.startsWith("--resolve="))?.split("=")[1];
if (resolveArg && state.result?.state === "activated") {
  const listed = (await host.handle(req(5, "webview.list"))).result;
  notes.length = 0;
  const resolved = await Promise.race([host.handle(req(6, "webview.resolveView", { viewType: resolveArg })), timer]);
  await new Promise((r) => setTimeout(r, 3000)); // let the provider/webview settle
  const htmlNote = notes.filter((n) => n.method === "webview.setHtml").at(-1);
  const optsNote = notes.filter((n) => n.method === "webview.setOptions").at(-1);
  const html = htmlNote?.params.html ?? "";
  view = {
    registeredViewTypes: listed.viewTypes,
    resolve: resolved.error ? { error: resolved.error } : { handle: resolved.result.handle },
    htmlBytes: html.length,
    scripts: [...html.matchAll(/<script[^>]*\ssrc="([^"]+)"/g)].map((m) => m[1].slice(0, 120)),
    styles: [...html.matchAll(/<link[^>]*href="([^"]+)"/g)].map((m) => m[1].slice(0, 120)),
    csp: (html.match(/Content-Security-Policy"?\s+content="([^"]+)"/i)?.[1] ?? "").slice(0, 300),
    options: optsNote?.params.options,
    postedToWebview: notes.filter((n) => n.method === "webview.postMessage").map((n) => String(n.params.message?.type ?? typeof n.params.message)),
    errorsShownToUser: notes.filter((n) => n.method === "window.showErrorMessage" || n.method === "window.showWarningMessage").map((n) => String(n.params.message).slice(0, 200)),
    otherNotifications: [...new Set(notes.map((n) => n.method))],
  };
}

const report = {
  view,
  id: loaded.result.id,
  mode: discover ? "discover" : "strict",
  state: state.result?.state,
  activation: activated.error ? { error: activated.error } : activated.result,
  unsupportedApis: [...unsupported].sort(),
  notifications: notes.length,
};
console.log("REPORT " + JSON.stringify(report, null, 2));
fs.rmSync(ws, { recursive: true, force: true });
fs.rmSync(sandboxHome, { recursive: true, force: true });
process.exit(0);
