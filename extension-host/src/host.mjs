// Node extension host. Newline-delimited JSON over stdio.
// Requests (IDE -> host): {protocolVersion, requestId, method, params}
// Responses: {requestId, result|error}. Notifications (host -> IDE): {method, params}, no requestId.
// Unsupported methods return an explicit UnsupportedCapability error (PRD 5.4).
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import Module, { createRequire } from "node:module";
import readline from "node:readline";
import { pathToFileURL } from "node:url";
import { createVscode, HOST_VERSION } from "./vscode-api.mjs";
import { createContext } from "./context.mjs";

export const PROTOCOL_VERSION = 1;

// One process-wide hook; `currentVscode` points at the most recently initialized host's API.
// (A real host process has exactly one; tests create several in one process.)
let currentVscode = null;
const origLoad = Module._load;
Module._load = function (request, parent, isMain) {
  if (request === "vscode" && currentVscode) return currentVscode.forFile(parent?.filename);
  return origLoad.call(this, request, parent, isMain);
};

export function createHost({ notify = () => {} } = {}) {
  let vscodeApi = null;
  let workspaceFolders = [];
  let storageRoot = path.join(os.tmpdir(), "nanti-ext-storage");
  /** id -> {dir, manifest, main, state: installed|activated|failed, module, context} */
  const extensions = new Map();

  function ext(id) {
    const e = extensions.get(id);
    if (!e) throw Object.assign(new Error(`unknown extension ${id}`), { code: "UnknownExtension" });
    return e;
  }

  const handlers = {
    initialize(params = {}) {
      workspaceFolders = params.workspaceFolders ?? [];
      storageRoot = params.storageRoot ?? path.join(os.tmpdir(), "nanti-ext-storage");
      vscodeApi = createVscode({
        workspaceFolders,
        notify,
        settings: params.settings ?? {},
        getExtensions: () => [...extensions].map(([id, e]) => ({ id, ...e })),
      });
      currentVscode = vscodeApi;
      return { protocolVersion: PROTOCOL_VERSION, hostVersion: HOST_VERSION };
    },
    ping: (params) => ({ echo: params }),
    "env.openExternal.resolve"({ id, approved }) {
      if (!vscodeApi) throw new Error("host not initialized");
      return { resolved: vscodeApi.resolveExternal(id, approved) };
    },
    shutdown: () => null,

    "extension.load"({ path: dir }) {
      const manifest = JSON.parse(fs.readFileSync(path.join(dir, "package.json"), "utf8"));
      if (!manifest.name || !manifest.publisher) throw new Error("manifest needs name and publisher");
      if (!manifest.main) throw new Error("manifest has no 'main' (not a Node extension)");
      const id = `${manifest.publisher}.${manifest.name}`;
      extensions.set(id, { dir, manifest, main: path.resolve(dir, manifest.main), state: "installed" });
      vscodeApi?.config.registerDefaults(manifest.contributes?.configuration);
      vscodeApi?.registerExtensionDir(dir);
      return {
        id,
        state: "installed",
        activationEvents: manifest.activationEvents ?? [],
        commands: manifest.contributes?.commands ?? [],
      };
    },

    async "extension.activate"({ id }) {
      const e = ext(id);
      if (e.state === "activated") return { id, state: e.state };
      try {
        currentVscode = vscodeApi;
        const req = createRequire(pathToFileURL(e.main));
        delete req.cache[req.resolve(e.main)]; // fresh module per activation
        e.module = req(e.main);
        e.context = createContext(
          { dir: e.dir, id, manifest: e.manifest, isActive: () => e.state === "activated", getExports: () => e.exports },
          { storageRoot, workspaceFolders },
        );
        e.exports = await e.module.activate?.(e.context);
        e.state = "activated"; // only after entry point loaded AND activate() resolved
      } catch (err) {
        e.state = "failed";
        console.error(`[host] activation of ${id} failed: ${err?.stack ?? err}`);
        throw Object.assign(new Error(String(err?.message ?? err)), { code: err?.code ?? "ActivationFailed" });
      }
      return { id, state: e.state };
    },

    async "extension.deactivate"({ id }) {
      const e = ext(id);
      await e.module?.deactivate?.();
      for (const d of e.context?.subscriptions ?? []) d.dispose();
      e.state = "installed";
      return { id, state: e.state };
    },

    "extension.state": ({ id }) => ({ id, state: ext(id).state }),

    async "commands.execute"({ command, args = [] }) {
      if (!vscodeApi) throw new Error("host not initialized");
      return (await vscodeApi.vscode.commands.executeCommand(command, ...args)) ?? null;
    },

    /**
     * IDE delivers a URI callback (OAuth login, VH-07). The authority must be the target extension id;
     * a mismatch is rejected instead of being broadcast to other extensions.
     */
    "workspace.didOpenTextDocument": (params) => vscodeApi?.syncOpenDocument(params) ?? null,
    "workspace.didChangeTextDocument": (params) => vscodeApi?.syncChangeDocument(params) ?? null,
    "workspace.didSaveTextDocument": (params) => vscodeApi?.syncSaveDocument(params) ?? null,
    "workspace.didCloseTextDocument": (params) => vscodeApi?.syncCloseDocument(params) ?? null,
    async "uri.handle"({ uri }) {
      if (!vscodeApi) throw new Error("host not initialized");
      const parsed = vscodeApi.vscode.Uri.parse(uri);
      const target = extensions.get([...extensions.keys()].find((id) => id.toLowerCase() === parsed.authority.toLowerCase()));
      const handler = target && vscodeApi.uriHandlers.get(path.resolve(target.dir));
      if (!handler) throw Object.assign(new Error(`no URI handler for '${parsed.authority}'`), { code: "NoUriHandler" });
      await handler.handleUri(parsed);
      return null;
    },
  };

  async function handle(msg) {
    const { requestId, method, params } = msg;
    if (msg.protocolVersion !== PROTOCOL_VERSION) {
      return {
        requestId,
        error: { code: "ProtocolMismatch", message: `expected ${PROTOCOL_VERSION}, got ${msg.protocolVersion}` },
      };
    }
    const fn = handlers[method] ?? vscodeApi?.hub.handlers[method];
    if (!fn) {
      console.error(`[host] UnsupportedCapability: ${method}`);
      return { requestId, error: { code: "UnsupportedCapability", message: method } };
    }
    try {
      return { requestId, result: await fn(params) };
    } catch (e) {
      return { requestId, error: { code: e?.code ?? "HandlerError", message: String(e?.message ?? e) } };
    }
  }

  return { handle };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  // An extension bug must not take the whole host down (PRD VH-11): log and keep serving.
  process.on("unhandledRejection", (e) => console.error(`[host] unhandled rejection: ${e?.stack ?? e}`));
  process.on("uncaughtException", (e) => console.error(`[host] uncaught exception: ${e?.stack ?? e}`));
  const send = (obj) => process.stdout.write(JSON.stringify(obj) + "\n");
  const host = createHost({ notify: (method, params) => send({ method, params }) });
  const rl = readline.createInterface({ input: process.stdin });
  // Serialize: one request at a time keeps response order deterministic for the IDE.
  let chain = Promise.resolve();
  rl.on("line", (line) => {
    let msg;
    try {
      msg = JSON.parse(line);
    } catch {
      console.error("[host] dropped malformed line");
      return;
    }
    // Consent responses must bypass the serialized queue: an extension command may be
    // awaiting openExternal while its original request is still in that queue.
    if (msg.method === "env.openExternal.resolve") {
      void host.handle(msg).then(send);
      return;
    }
    chain = chain.then(async () => {
      send(await host.handle(msg));
      if (msg.method === "shutdown") rl.close();
    });
  });
  rl.on("close", () => chain.then(() => process.exit(0)));
}
