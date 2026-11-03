import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import fs from "node:fs";
import os from "node:os";
import { createVscode } from "../src/vscode-api.mjs";

test("TextDocument: ranges, lines, offsets, words; an edit keeps the same object (VH-04)", async () => {
  const api = createVscode({ notify() {} });
  const uri = "file:///C:/w/a.ts";
  api.syncOpenDocument({ uri, languageId: "typescript", version: 1, text: "let foo = 1;\r\nbar(foo)\n" });
  const doc = await api.vscode.workspace.openTextDocument(uri);
  const { Position, Range } = api.vscode;
  assert.equal(doc.lineCount, 3);
  assert.equal(doc.eol, 2);
  assert.equal(doc.lineAt(0).text, "let foo = 1;");
  assert.equal(doc.lineAt(1).text, "bar(foo)");
  assert.equal(doc.lineAt(2).text, "");
  assert.equal(doc.lineAt(1).firstNonWhitespaceCharacterIndex, 0);
  assert.throws(() => doc.lineAt(9), /Illegal value/);
  assert.equal(doc.offsetAt(new Position(1, 4)), 18);
  assert.deepEqual(doc.positionAt(18), new Position(1, 4));
  assert.equal(doc.getText(new Range(0, 4, 1, 3)), "foo = 1;\r\nbar");
  assert.deepEqual(doc.validatePosition(new Position(99, 99)), new Position(2, 0));
  assert.deepEqual(doc.getWordRangeAtPosition(new Position(0, 5)), new Range(0, 4, 0, 7));
  assert.equal(doc.getWordRangeAtPosition(new Position(0, 9)), undefined);
  assert.deepEqual(doc.getWordRangeAtPosition(new Position(1, 5), /\w+/), new Range(1, 4, 1, 7));

  api.syncChangeDocument({ uri, version: 2, text: "x\ny\nz" });
  assert.equal(doc.lineCount, 3);
  assert.equal(doc.getText(new Range(1, 0, 2, 1)), "y\nz");
  assert.equal(doc.version, 2);
  assert.equal(doc.isDirty, true);
});

test("value types: Position and Range order, Selection, TextEdit, Diagnostic, enums, cancellation", () => {
  const { vscode: v } = createVscode({ notify() {} });
  const a = new v.Position(1, 2);
  assert.ok(a.isBefore(new v.Position(1, 3)) && a.isAfter(new v.Position(0, 9)) && a.isEqual(new v.Position(1, 2)));
  assert.deepEqual(a.translate(1, -1), new v.Position(2, 1));
  assert.throws(() => new v.Position(-1, 0), /non-negative/);
  const r = new v.Range(new v.Position(3, 0), new v.Position(1, 0));
  assert.deepEqual(r.start, new v.Position(1, 0), "a reversed range is put in order");
  assert.ok(r.contains(new v.Position(2, 4)) && !r.contains(new v.Position(4, 0)));
  assert.deepEqual(r.intersection(new v.Range(2, 0, 9, 0)), new v.Range(2, 0, 3, 0));
  assert.equal(r.intersection(new v.Range(8, 0, 9, 0)), undefined);
  const sel = new v.Selection(new v.Position(2, 0), new v.Position(1, 0));
  assert.ok(sel.isReversed && sel.start.line === 1);
  assert.equal(v.TextEdit.replace(r, "x").newText, "x");
  assert.equal(v.TextEdit.delete(r).newText, "");
  assert.equal(new v.Diagnostic(r, "bad").severity, v.DiagnosticSeverity.Error);
  assert.equal(v.DiagnosticSeverity[1], "Warning");
  assert.equal(v.EndOfLine.CRLF, 2);
  assert.equal(new v.MarkdownString("a").appendMarkdown("**b**").value, "a**b**");
  assert.equal(v.ThemeIcon.File.id, "file");
  assert.equal(v.FileSystemError.FileNotFound("x").code, "FileNotFound");
  const src = new v.CancellationTokenSource();
  let fired = 0;
  src.token.onCancellationRequested(() => fired++);
  src.cancel();
  src.cancel();
  assert.equal(src.token.isCancellationRequested, true);
  assert.equal(fired, 1);
});

test("workspace: getWorkspaceFolder, asRelativePath, findFiles and fs helpers", async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "nanti-ws-"));
  fs.mkdirSync(path.join(dir, "src", "deep"), { recursive: true });
  fs.mkdirSync(path.join(dir, "node_modules", "x"), { recursive: true });
  fs.writeFileSync(path.join(dir, "src", "a.ts"), "a");
  fs.writeFileSync(path.join(dir, "src", "deep", "b.ts"), "b");
  fs.writeFileSync(path.join(dir, "src", "c.js"), "c");
  fs.writeFileSync(path.join(dir, "node_modules", "x", "d.ts"), "d");
  const { vscode: v } = createVscode({ notify() {}, workspaceFolders: [dir] });
  const ws = v.workspace;
  const inside = v.Uri.file(path.join(dir, "src", "a.ts"));
  assert.equal(ws.getWorkspaceFolder(inside).uri.fsPath, dir);
  assert.equal(ws.getWorkspaceFolder(v.Uri.file(path.join(os.tmpdir(), "elsewhere.txt"))), undefined);
  assert.equal(ws.asRelativePath(inside), "src/a.ts");
  assert.equal(ws.asRelativePath(inside, true), `${path.basename(dir)}/src/a.ts`);
  assert.equal(ws.rootPath, dir);

  const names = async (include, exclude) =>
    (await ws.findFiles(include, exclude)).map((u) => path.relative(dir, u.fsPath).replace(/\\/g, "/")).sort();
  assert.deepEqual(await names("**/*.ts"), ["src/a.ts", "src/deep/b.ts"], "node_modules is skipped");
  assert.deepEqual(await names("**/*.ts", "**/deep/**"), ["src/a.ts"]);
  assert.deepEqual(await names(new v.RelativePattern(path.join(dir, "src"), "*.js")), ["src/c.js"]);
  assert.equal((await ws.findFiles("**/*.ts", undefined, 1)).length, 1);

  await ws.fs.createDirectory(v.Uri.file(path.join(dir, "made", "x")));
  assert.deepEqual(await ws.fs.readDirectory(v.Uri.file(path.join(dir, "made"))), [["x", 2]]);
  await ws.fs.rename(inside, v.Uri.file(path.join(dir, "src", "z.ts")));
  await assert.rejects(
    ws.fs.rename(v.Uri.file(path.join(dir, "src", "z.ts")), v.Uri.file(path.join(dir, "src", "c.js"))),
    /FileExists/,
  );
  await ws.fs.copy(v.Uri.file(path.join(dir, "src", "z.ts")), v.Uri.file(path.join(dir, "copy.ts")));
  assert.equal(fs.readFileSync(path.join(dir, "copy.ts"), "utf8"), "a");
  await ws.fs.delete(v.Uri.file(path.join(dir, "made")), { recursive: true });
  assert.equal(fs.existsSync(path.join(dir, "made")), false);
  fs.rmSync(dir, { recursive: true });
});
