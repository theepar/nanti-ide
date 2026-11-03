import test from "node:test";
import assert from "node:assert/strict";
import { apisIn } from "../../scripts/compat-static.mjs";

const sorted = (set) => [...set].sort();

test("finds namespace members and classes through the usual module aliases", () => {
  const ts = `const vscode_1 = require("vscode"); vscode_1.window.showInformationMessage("x"); new vscode_1.Range(0,0,1,1);`;
  assert.deepEqual(sorted(apisIn(ts)), ["Range", "window.showInformationMessage"]);

  const esbuild = `var import_vscode = __toESM(require("vscode")); import_vscode.workspace.findFiles("*"); import_vscode.languages.registerHoverProvider({}, {});`;
  assert.deepEqual(sorted(apisIn(esbuild)), ["languages.registerHoverProvider", "workspace.findFiles"]);

  const minified = `var o=require("vscode");o.commands.registerCommand("a",()=>{});o.TreeItemCollapsibleState.None;`;
  assert.deepEqual(sorted(apisIn(minified)), ["TreeItemCollapsibleState", "commands.registerCommand"]);

  const esm = `import * as vscode from "vscode"; vscode.window.createOutputChannel("x");`;
  assert.deepEqual(sorted(apisIn(esm)), ["window.createOutputChannel"]);
});

test("finds the usage behind a webpack external and behind destructuring", () => {
  const webpack = `
    ({ 7: e => { e.exports = require("vscode") }, 8: (e, t, n) => { const s = n(7); s.window.showErrorMessage("x"); } })`;
  assert.deepEqual(sorted(apisIn(webpack)), ["window.showErrorMessage"]);

  const destructured = `const { window, workspace, Position } = require("vscode"); window.showQuickPick([]); workspace.getConfiguration("a");`;
  assert.deepEqual(sorted(apisIn(destructured)), ["Position", "window.showQuickPick", "workspace.getConfiguration"]);
});

test("does not mistake other packages or the extension's own classes for the VS Code API", () => {
  const other = `const vscode_languageclient_1 = require("vscode-languageclient"); vscode_languageclient_1.CloseAction.Restart; vscode_languageclient_1.LanguageClient;`;
  assert.deepEqual(sorted(apisIn(other)), []);
  const own = `const vscode = require("vscode"); vscode.MyOwnThing; vscode.window.showWarningMessage("x");`;
  assert.deepEqual(sorted(apisIn(own)), ["window.showWarningMessage"], "only names the VS Code API has");
  assert.deepEqual(sorted(apisIn("const a = 1; foo.window.bar();")), []);
});
