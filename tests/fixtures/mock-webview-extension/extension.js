const vscode = require("vscode");

exports.activate = (context) => {
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider("mockweb.view", {
      resolveWebviewView(view) {
        const media = vscode.Uri.joinPath(context.extensionUri, "media");
        view.webview.options = { enableScripts: true, localResourceRoots: [media] };
        const script = view.webview.asWebviewUri(vscode.Uri.joinPath(media, "main.js"));
        // A file next to (not inside) the allowed root: the webview must NOT be able to read it.
        const secret = view.webview.asWebviewUri(vscode.Uri.joinPath(context.extensionUri, "secret.txt"));
        view.webview.html =
          '<!doctype html><html><body style="background:#1e1e1e;color:#ddd;font-family:sans-serif" data-secret="' +
          secret + '"><h3 id="s">mock webview</h3><script src="' + script + '"></script></body></html>';
        view.webview.onDidReceiveMessage((m) => {
          if (m.type === "ready") view.webview.postMessage({ type: "ping", n: 1 });
          if (m.type === "pong") vscode.window.showInformationMessage("pong " + m.n);
          if (m.type === "secret") vscode.window.showInformationMessage("secret " + m.status);
        });
      },
    }),
    vscode.window.registerUriHandler({
      handleUri(uri) {
        vscode.window.showInformationMessage("uri " + uri.path + "?" + uri.query);
      },
    }),
  );
};
