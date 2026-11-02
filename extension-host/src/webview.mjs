// Webview views/panels (VH-06). The host only models the extension-facing API and forwards state
// to the IDE as notifications; the IDE owns the real WebView2 surface and the resource protocol.
//
// host -> IDE notifications:
//   webview.viewProviderRegistered {viewType} / webview.viewProviderUnregistered {viewType}
//   webview.panelCreated {handle, viewType, title, showOptions, options}
//   webview.setHtml {handle, html} / webview.setOptions {handle, options} / webview.postMessage {handle, message}
//   webview.setTitle {handle, title} / webview.reveal {handle} / webview.disposed {handle}
// IDE -> host requests: webview.resolveView {viewType}, webview.message {handle, message},
//   webview.dispose {handle}, webview.visibility {handle, visible}

/** Origin the IDE serves webview content and resources from (wry custom protocol, https scheme). */
export const WEBVIEW_ORIGIN = "https://nanti-resource.localhost";

export function createWebviewHub({ notify, Disposable, EventEmitter, Uri, workspaceFolders }) {
  const viewProviders = new Map(); // viewType -> {provider, ownerDir, options}
  const serializers = new Map(); // viewType -> serializer (registry only; restore not wired)
  const live = new Map(); // handle -> {kind, webview, owner, dispose, setVisible}
  let seq = 0;

  function makeWebview(handle, ownerDir, initialOptions = {}) {
    const onMessage = new EventEmitter();
    let html = "";
    let options = initialOptions;
    const effectiveRoots = () =>
      (
        options.localResourceRoots ?? [...(ownerDir ? [Uri.file(ownerDir)] : []), ...workspaceFolders.map((p) => Uri.file(p))]
      ).map((u) => u.fsPath);
    const wireOptions = () => ({
      enableScripts: !!options.enableScripts,
      enableForms: options.enableForms ?? !!options.enableScripts,
      retainContextWhenHidden: !!options.retainContextWhenHidden,
      localResourceRoots: effectiveRoots(),
    });
    const webview = {
      get html() {
        return html;
      },
      set html(value) {
        html = String(value);
        notify("webview.setHtml", { handle, html });
      },
      get options() {
        return options;
      },
      set options(value) {
        options = value ?? {};
        notify("webview.setOptions", { handle, options: wireOptions() });
      },
      onDidReceiveMessage: onMessage.event,
      // Resolves true once handed to the IDE; delivery to a hidden webview is the IDE's concern.
      postMessage(message) {
        notify("webview.postMessage", { handle, message });
        return Promise.resolve(true);
      },
      asWebviewUri(uri) {
        if (uri.scheme !== "file") throw new Error("asWebviewUri supports file URIs only");
        return Uri.parse(`${WEBVIEW_ORIGIN}/file/${encodeURIComponent(uri.fsPath)}`);
      },
      cspSource: WEBVIEW_ORIGIN,
    };
    return { webview, fireMessage: (m) => onMessage.fire(m), wireOptions };
  }

  function forOwner(ownerDir) {
    return {
      registerWebviewViewProvider(viewType, provider, options = {}) {
        if (viewProviders.has(viewType)) throw new Error(`View provider for '${viewType}' already registered`);
        viewProviders.set(viewType, { provider, ownerDir, options });
        notify("webview.viewProviderRegistered", {
          viewType,
          retainContextWhenHidden: !!options.webviewOptions?.retainContextWhenHidden,
        });
        return new Disposable(() => {
          viewProviders.delete(viewType);
          notify("webview.viewProviderUnregistered", { viewType });
        });
      },

      createWebviewPanel(viewType, title, showOptions, options = {}) {
        const handle = `panel-${++seq}`;
        const { webview, fireMessage, wireOptions } = makeWebview(handle, ownerDir, options);
        const disposeEmitter = new EventEmitter();
        const stateEmitter = new EventEmitter();
        let panelTitle = title;
        let disposed = false;
        const panel = {
          viewType,
          webview,
          options,
          viewColumn: typeof showOptions === "object" ? showOptions?.viewColumn : showOptions,
          visible: true,
          active: true,
          get title() {
            return panelTitle;
          },
          set title(v) {
            panelTitle = String(v);
            notify("webview.setTitle", { handle, title: panelTitle });
          },
          onDidDispose: disposeEmitter.event,
          onDidChangeViewState: stateEmitter.event,
          reveal() {
            notify("webview.reveal", { handle });
          },
          dispose() {
            if (disposed) return;
            disposed = true;
            live.delete(handle);
            notify("webview.disposed", { handle });
            disposeEmitter.fire();
          },
        };
        live.set(handle, {
          kind: "panel",
          fireMessage,
          dispose: () => panel.dispose(),
          setVisible: (visible) => {
            panel.visible = visible;
            stateEmitter.fire({ webviewPanel: panel });
          },
        });
        notify("webview.panelCreated", {
          handle,
          viewType,
          title,
          showOptions: typeof showOptions === "object" ? { viewColumn: showOptions?.viewColumn ?? 1, preserveFocus: !!showOptions?.preserveFocus } : { viewColumn: showOptions ?? 1 },
          options: wireOptions(),
        });
        return panel;
      },

      registerWebviewPanelSerializer(viewType, serializer) {
        if (serializers.has(viewType)) throw new Error(`Serializer for '${viewType}' already registered`);
        serializers.set(viewType, serializer);
        return new Disposable(() => serializers.delete(viewType));
      },
    };
  }

  const handlers = {
    /** IDE asks the host to create the view for a registered viewType and call the provider. */
    async "webview.resolveView"({ viewType }) {
      const entry = viewProviders.get(viewType);
      if (!entry) throw Object.assign(new Error(`no view provider for ${viewType}`), { code: "UnknownView" });
      const handle = `view-${++seq}`;
      const { webview, fireMessage, wireOptions } = makeWebview(handle, entry.ownerDir, {});
      const disposeEmitter = new EventEmitter();
      const visEmitter = new EventEmitter();
      const view = {
        viewType,
        webview,
        title: undefined,
        description: undefined,
        visible: true,
        onDidDispose: disposeEmitter.event,
        onDidChangeVisibility: visEmitter.event,
        show() {
          notify("webview.reveal", { handle });
        },
      };
      live.set(handle, {
        kind: "view",
        fireMessage,
        dispose: () => {
          live.delete(handle);
          disposeEmitter.fire();
        },
        setVisible: (visible) => {
          view.visible = visible;
          visEmitter.fire();
        },
      });
      notify("webview.setOptions", { handle, options: wireOptions() });
      const token = { isCancellationRequested: false, onCancellationRequested: new EventEmitter().event };
      await entry.provider.resolveWebviewView(view, { state: undefined }, token);
      return { handle };
    },

    "webview.message"({ handle, message }) {
      const w = live.get(handle);
      if (!w) throw Object.assign(new Error(`unknown webview ${handle}`), { code: "UnknownWebview" });
      w.fireMessage(message);
      return null;
    },

    "webview.dispose"({ handle }) {
      live.get(handle)?.dispose();
      return null;
    },

    "webview.visibility"({ handle, visible }) {
      live.get(handle)?.setVisible(!!visible);
      return null;
    },

    "webview.list": () => ({
      viewTypes: [...viewProviders.keys()],
      serializers: [...serializers.keys()],
      live: [...live.keys()],
    }),
  };

  return { forOwner, handlers };
}
