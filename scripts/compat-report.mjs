// Runs real extensions from Open VSX through the nanti extension host and reports which ones activate
// and which VS Code APIs they reach for that the host does not provide.
//
// usage: node scripts/compat-report.mjs [--work=<dir>] [--timeout=<ms>] [--json=<file>] publisher.name ...
// Every extension runs in its own process with a throwaway HOME (see probe-extension.mjs); nothing is kept.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (name, dflt) => args.find((a) => a.startsWith(`--${name}=`))?.split("=").slice(1).join("=") ?? dflt;
const ids = args.filter((a) => !a.startsWith("--"));
const work = path.resolve(opt("work", path.join(os.tmpdir(), "nanti-compat")));
const timeout = Number(opt("timeout", "25000"));
const jsonOut = opt("json", "");
if (ids.length === 0) {
  console.error("usage: compat-report.mjs [--work=dir] [--timeout=ms] [--json=file] publisher.name ...");
  process.exit(2);
}
fs.mkdirSync(work, { recursive: true });

async function download(id) {
  const [ns, name] = id.split(".");
  const meta = await fetch(`https://open-vsx.org/api/${ns}/${name}/latest`).then((r) => (r.ok ? r.json() : null));
  if (!meta?.files?.download) return { error: "not on Open VSX" };
  const target = path.join(work, id);
  const marker = path.join(target, ".version");
  if (!(fs.existsSync(marker) && fs.readFileSync(marker, "utf8") === meta.version)) {
    fs.rmSync(target, { recursive: true, force: true });
    const vsix = path.join(work, `${id}.zip`);
    const res = await fetch(meta.files.download);
    if (!res.ok) return { error: `download failed (${res.status})` };
    fs.writeFileSync(vsix, Buffer.from(await res.arrayBuffer()));
    const zip = spawnSync(
      "powershell",
      ["-NoProfile", "-Command", `Add-Type -A System.IO.Compression.FileSystem; [IO.Compression.ZipFile]::ExtractToDirectory('${vsix}', '${target}')`],
      { encoding: "utf8" },
    );
    fs.rmSync(vsix, { force: true });
    if (zip.status !== 0) return { error: `extract failed: ${zip.stderr.trim().slice(0, 120)}` };
    fs.writeFileSync(marker, meta.version);
  }
  return { version: meta.version, dir: path.join(target, "extension"), size: meta.size };
}

function probe(dir) {
  const started = Date.now();
  const run = spawnSync(
    process.execPath,
    [path.join(here, "probe-extension.mjs"), dir, "--discover", `--timeout=${timeout}`],
    { encoding: "utf8", timeout: timeout + 15000, maxBuffer: 32 * 1024 * 1024 },
  );
  const ms = Date.now() - started;
  const text = run.stdout ?? "";
  const at = text.indexOf("REPORT ");
  if (at >= 0) {
    try {
      return { ms, report: JSON.parse(text.slice(at + 7)) };
    } catch {
      /* fall through */
    }
  }
  const stage = text.match(/\{"stage".*\}/)?.[0];
  return { ms, failure: stage ?? (run.error ? String(run.error.message) : (run.stderr || text).trim().split("\n").at(-1)?.slice(0, 160) ?? "no report") };
}

const rows = [];
for (const id of ids) {
  const got = await download(id);
  if (got.error) {
    rows.push({ id, status: "SKIP", note: got.error });
    console.log(`${id}: ${got.error}`);
    continue;
  }
  let manifest;
  try {
    manifest = JSON.parse(fs.readFileSync(path.join(got.dir, "package.json"), "utf8"));
  } catch {
    rows.push({ id, version: got.version, status: "SKIP", note: "no readable package.json" });
    continue;
  }
  const c = manifest.contributes ?? {};
  const declarative = Object.keys(c).filter((k) => ["themes", "iconThemes", "snippets", "grammars", "languages", "productIconThemes", "keybindings"].includes(k));
  if (!manifest.main) {
    rows.push({ id, version: got.version, status: "DECLARATIVE", note: `no code, contributes ${declarative.join(", ") || "nothing"}` });
    console.log(`${id}@${got.version}: declarative (${declarative.join(", ")})`);
    continue;
  }
  const { ms, report, failure } = probe(got.dir);
  if (!report) {
    rows.push({ id, version: got.version, status: "FAIL", ms, note: failure });
    console.log(`${id}@${got.version}: FAIL ${failure}`);
    continue;
  }
  const activated = report.state === "activated";
  rows.push({
    id,
    version: got.version,
    status: activated ? "OK" : "FAIL",
    ms,
    unsupported: report.unsupportedApis,
    note: activated ? "" : JSON.stringify(report.activation?.error ?? report.activation).slice(0, 200),
  });
  console.log(`${id}@${got.version}: ${activated ? "activated" : "FAILED"} in ${ms} ms, ${report.unsupportedApis.length} missing API(s)`);
}

const count = (s) => rows.filter((r) => r.status === s).length;
console.log(`\n${rows.length} extensions: ${count("OK")} activated, ${count("DECLARATIVE")} declarative only, ${count("FAIL")} failed, ${count("SKIP")} skipped`);
const missing = new Map();
for (const r of rows) for (const api of r.unsupported ?? []) missing.set(api, (missing.get(api) ?? 0) + 1);
const top = [...missing].sort((a, b) => b[1] - a[1]).slice(0, 40);
if (top.length) {
  console.log("\nAPIs the host does not provide, by how many extensions reach for them:");
  for (const [api, n] of top) console.log(`  ${String(n).padStart(3)}  ${api}`);
}
if (jsonOut) fs.writeFileSync(jsonOut, JSON.stringify({ rows, missing: Object.fromEntries(missing) }, null, 2));
