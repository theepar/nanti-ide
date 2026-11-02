const vscode = acquireVsCodeApi();
window.addEventListener("message", (e) => {
  if (e.data.type === "ping") {
    document.getElementById("s").textContent = "got ping " + e.data.n;
    vscode.postMessage({ type: "pong", n: e.data.n });
  }
});
vscode.postMessage({ type: "ready" });
fetch(document.body.dataset.secret)
  .then((r) => vscode.postMessage({ type: "secret", status: r.status }))
  .catch((err) => vscode.postMessage({ type: "secret", status: "error: " + err }));
