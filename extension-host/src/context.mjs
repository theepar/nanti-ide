import fs from "node:fs";
import path from "node:path";
import { Uri, EXTENSION_MODE, unsupportedValue } from "./vscode-api.mjs";

export class EnvironmentVariableCollection {
  #mutators = new Map();
  persistent = true;
  description = undefined;
  #set(type, name, value, options) {
    this.#mutators.set(name, { value, type, options: options ?? { applyAtProcessCreation: true } });
  }
  replace(name, value, options) {
    this.#set(1, name, value, options);
  }
  append(name, value, options) {
    this.#set(2, name, value, options);
  }
  prepend(name, value, options) {
    this.#set(3, name, value, options);
  }
  get(name) {
    return this.#mutators.get(name);
  }
  forEach(callback, thisArg) {
    for (const [name, mutator] of this.#mutators) callback.call(thisArg, name, mutator, this);
  }
  delete(name) {
    this.#mutators.delete(name);
  }
  clear() {
    this.#mutators.clear();
  }
  getScoped() {
    return this;
  }
  [Symbol.iterator]() {
    return this.#mutators.entries();
  }
}

export class Memento {
  #file;
  #data;
  constructor(file) {
    this.#file = file;
    try {
      this.#data = JSON.parse(fs.readFileSync(file, "utf8"));
    } catch {
      this.#data = {};
    }
  }
  keys() {
    return Object.keys(this.#data);
  }
  get(key, defaultValue) {
    return key in this.#data ? this.#data[key] : defaultValue;
  }
  update(key, value) {
    if (value === undefined) delete this.#data[key];
    else this.#data[key] = value;
    fs.mkdirSync(path.dirname(this.#file), { recursive: true });
    fs.writeFileSync(this.#file, JSON.stringify(this.#data));
    return Promise.resolve();
  }
  setKeysForSync() {}
}

export function createContext(ext, { storageRoot, workspaceFolders, mode = EXTENSION_MODE.Production }) {
  const safeId = ext.id.toLowerCase();
  const globalDir = path.join(storageRoot, "global", safeId);
  const wsKey = workspaceFolders[0] ? Buffer.from(workspaceFolders[0]).toString("hex").slice(0, 32) : null;
  const wsDir = wsKey ? path.join(storageRoot, "workspace", wsKey, safeId) : null;

  const context = {
    subscriptions: [],
    extensionPath: ext.dir,
    extensionUri: Uri.file(ext.dir),
    extensionMode: mode,
    globalStorageUri: Uri.file(globalDir),
    storageUri: wsDir ? Uri.file(wsDir) : undefined,
    logUri: Uri.file(path.join(storageRoot, "logs", safeId)),
    globalState: new Memento(path.join(globalDir, "globalState.json")),
    workspaceState: new Memento(path.join(wsDir ?? path.join(storageRoot, "no-workspace", safeId), "workspaceState.json")),
    asAbsolutePath: (rel) => path.join(ext.dir, rel),
    environmentVariableCollection: new EnvironmentVariableCollection(),
    extension: {
      id: ext.id,
      extensionPath: ext.dir,
      extensionUri: Uri.file(ext.dir),
      packageJSON: ext.manifest ?? {},
      extensionKind: 1,
      get isActive() {
        return ext.isActive?.() ?? false;
      },
      get exports() {
        return ext.getExports?.();
      },
      activate: async () => ext.getExports?.(),
    },
  };
  Object.defineProperty(context, "secrets", {
    enumerable: true,
    get: () => unsupportedValue("vscode.ExtensionContext.secrets"),
  });
  return context;
}
