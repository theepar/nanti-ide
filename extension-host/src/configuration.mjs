
export const CONFIGURATION_TARGET = Object.freeze({ Global: 1, Workspace: 2, WorkspaceFolder: 3 });

export class ConfigStore {
  #defaults = new Map();
  #global = new Map();
  #workspace = new Map();
  #notify;
  #onChange;

  constructor({ settings = {}, notify, onChange }) {
    this.#notify = notify;
    this.#onChange = onChange;
    for (const [k, v] of Object.entries(settings.global ?? {})) this.#global.set(k, v);
    for (const [k, v] of Object.entries(settings.workspace ?? {})) this.#workspace.set(k, v);
  }

  registerDefaults(configuration) {
    const sections = Array.isArray(configuration) ? configuration : configuration ? [configuration] : [];
    for (const section of sections) {
      for (const [key, prop] of Object.entries(section.properties ?? {})) {
        if (prop && "default" in prop) this.#defaults.set(key, prop.default);
      }
    }
  }

  #lookup(full) {
    if (this.#workspace.has(full)) return this.#workspace.get(full);
    if (this.#global.has(full)) return this.#global.get(full);
    return this.#defaults.get(full);
  }

  #children(full) {
    const prefix = `${full}.`;
    const out = {};
    const keys = new Set([...this.#defaults.keys(), ...this.#global.keys(), ...this.#workspace.keys()]);
    for (const k of keys) if (k.startsWith(prefix)) out[k.slice(prefix.length)] = this.#lookup(k);
    return Object.keys(out).length ? out : undefined;
  }

  section(section) {
    const full = (key) => (section ? `${section}.${key}` : key);
    const store = this;
    return {
      get(key, defaultValue) {
        const f = full(key);
        const v = store.#lookup(f) ?? store.#children(f);
        return v === undefined ? defaultValue : v;
      },
      has(key) {
        const f = full(key);
        return store.#lookup(f) !== undefined || store.#children(f) !== undefined;
      },
      inspect(key) {
        const f = full(key);
        return {
          key: f,
          defaultValue: store.#defaults.get(f),
          globalValue: store.#global.get(f),
          workspaceValue: store.#workspace.get(f),
        };
      },
      update(key, value, target) {
        const f = full(key);
        const scope = target === CONFIGURATION_TARGET.Workspace || target === false ? "workspace" : "global";
        const map = scope === "workspace" ? store.#workspace : store.#global;
        if (value === undefined) map.delete(f);
        else map.set(f, value);
        store.#notify("workspace.configuration.update", { key: f, value: value ?? null, scope });
        store.#onChange({ affectsConfiguration: (s) => f === s || f.startsWith(`${s}.`) || s.startsWith(`${f}.`) });
        return Promise.resolve();
      },
    };
  }
}
