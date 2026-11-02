const vscode = require("vscode");

function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand("mock.hello", (name) => {
      vscode.window.showInformationMessage(`hello ${name}`);
      return `hello ${name}`;
    }),
    vscode.commands.registerCommand("mock.readFile", async (relative) => {
      const root = vscode.workspace.workspaceFolders[0].uri.fsPath;
      const bytes = await vscode.workspace.fs.readFile(vscode.Uri.file(root + "/" + relative));
      return Buffer.from(bytes).toString("utf8");
    }),
    vscode.commands.registerCommand("mock.unsupported", () => vscode.debug.startDebugging()),
  );
}

function deactivate() {}

module.exports = { activate, deactivate };
