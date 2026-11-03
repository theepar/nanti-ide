// Downloads an extension from Open VSX and unpacks it; the files are only read, never run.
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

export async function fetchExtension(id, work) {
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
