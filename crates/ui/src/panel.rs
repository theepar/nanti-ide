use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use app::host::ExtensionHost;
use gpui::Window;
use serde_json::{Value, json};
use wry::dpi::{LogicalPosition, LogicalSize};
use wry::http::Response;
use wry::{PageLoadEvent, Rect, WebView, WebViewBuilder, WebViewBuilderExtWindows};

use crate::resource::{ORIGIN, ResourceState, serve};

#[derive(Debug)]
pub enum UiEvent {
    Html(String),
    Roots(Vec<PathBuf>),
    Message(Value),
    OpenExternal {
        id: String,
        url: String,
    },
    TerminalCreate {
        id: String,
        name: String,
        shell_path: Option<String>,
        shell_args: Option<Vec<String>>,
        cwd: Option<PathBuf>,
    },
    TerminalSendText {
        id: String,
        text: String,
        add_new_line: bool,
    },
    TerminalShow {
        id: String,
        preserve_focus: bool,
    },
    TerminalDispose {
        id: String,
    },
    StatusItem {
        id: String,
        visible: bool,
        text: String,
    },
}

#[derive(Debug)]
pub enum HostCommand {
    WebviewMessage(Value),
    ResolveExternal {
        id: String,
        approved: bool,
    },
    DocOpen {
        uri: String,
        language_id: String,
        version: u64,
        text: String,
    },
    DocChange {
        uri: String,
        version: u64,
        text: String,
    },
    DocSave {
        uri: String,
    },
    DocClose {
        uri: String,
    },
    Shutdown,
}

pub struct HostConfig {
    pub node: String,
    pub entry: PathBuf,
    pub extension_dir: PathBuf,
    pub view_type: String,
    pub workspace: Option<PathBuf>,
    pub selftest_log: Option<PathBuf>,
}

const THEME_CSS: &str = ":root{color-scheme:dark;--vscode-editor-background:#1e1e1e;--vscode-editor-foreground:#cccccc;--vscode-foreground:#cccccc;--vscode-sideBar-background:#1e1e1e;--vscode-editorWidget-background:#252526;--vscode-editorWidget-foreground:#cccccc;--vscode-editorWidget-border:#454545;--vscode-widget-border:#454545;--vscode-panel-border:#2b2b2b;--vscode-editorHoverWidget-background:#252526;--vscode-editorHoverWidget-foreground:#cccccc;--vscode-editorHoverWidget-border:#454545;--vscode-editorHoverWidget-statusBarBackground:#2c2c2d;--vscode-input-background:#2b2b2b;--vscode-input-foreground:#cccccc;--vscode-input-border:#3c3c3c;--vscode-input-placeholderForeground:#858585;--vscode-button-background:#3a3a3a;--vscode-button-foreground:#ffffff;--vscode-button-hoverBackground:#4a4a4a;--vscode-button-border:transparent;--vscode-button-secondaryBackground:#3a3d41;--vscode-button-secondaryHoverBackground:#45494e;--vscode-focusBorder:#4a4a4a;--vscode-contrastBorder:transparent;--vscode-textLink-foreground:#3794ff;--vscode-textLink-activeForeground:#3794ff;--vscode-editorLink-activeForeground:#4e94ce;--vscode-textCodeBlock-background:#2b2b2b;--vscode-textPreformat-foreground:#d7ba7d;--vscode-textPreformat-background:#3c3c3c;--vscode-descriptionForeground:#9d9d9d;--vscode-disabledForeground:#6b6b6b;--vscode-errorForeground:#f48771;--vscode-icon-foreground:#c5c5c5;--vscode-toolbar-hoverBackground:#2a2d2e;--vscode-list-hoverBackground:#2a2d2e;--vscode-list-activeSelectionBackground:#37373d;--vscode-list-activeSelectionForeground:#ffffff;--vscode-menu-background:#252526;--vscode-menu-foreground:#cccccc;--vscode-menu-border:#454545;--vscode-menu-selectionBackground:#37373d;--vscode-menu-selectionForeground:#ffffff;--vscode-scrollbarSlider-background:#4242427f;--vscode-scrollbarSlider-hoverBackground:#5a5a5a99;--vscode-scrollbarSlider-activeBackground:#7a7a7a99;--vscode-badge-background:#4d4d4d;--vscode-badge-foreground:#ffffff;--vscode-banner-background:#2b2a1f;--vscode-progressBar-background:#b8b8b8;--vscode-list-focusBackground:#37373d;--vscode-list-focusForeground:#ffffff;--vscode-list-inactiveSelectionBackground:#37373d;--vscode-quickInputList-focusBackground:#37373d;--vscode-inputOption-activeBorder:#8a8a8a;--vscode-inputOption-activeBackground:#4a4a4a;--vscode-inputOption-hoverBackground:#3a3a3a;--vscode-toolbar-activeBackground:#3a3a3a;--vscode-notifications-background:#252526;--vscode-diffEditor-insertedTextBackground:#9ccc2c33;--vscode-diffEditor-removedTextBackground:#ff000033;--vscode-diffEditor-insertedLineBackground:#9bb95533;--vscode-diffEditor-removedLineBackground:#ff000033;--vscode-gitDecoration-addedResourceForeground:#81b88b;--vscode-gitDecoration-deletedResourceForeground:#c74e39;--vscode-problemsErrorIcon-foreground:#f14c4c;--vscode-problemsWarningIcon-foreground:#cca700;--vscode-problemsInfoIcon-foreground:#3794ff;--vscode-inputValidation-infoBorder:#3794ff;--vscode-charts-blue:#3794ff;--vscode-terminal-ansiBlack:#000000;--vscode-terminal-ansiRed:#cd3131;--vscode-terminal-ansiGreen:#0dbc79;--vscode-terminal-ansiYellow:#e5e510;--vscode-terminal-ansiBlue:#2472c8;--vscode-terminal-ansiMagenta:#bc3fbc;--vscode-terminal-ansiCyan:#11a8cd;--vscode-terminal-ansiWhite:#e5e5e5;--vscode-terminal-ansiBrightBlack:#666666;--vscode-keybindingLabel-background:#8080802b;--vscode-keybindingLabel-foreground:#cccccc;--vscode-keybindingLabel-border:#3333337f;--vscode-keybindingLabel-bottomBorder:#4444447f;--vscode-editorGhostText-foreground:#ffffff56;--vscode-editorLineNumber-foreground:#6e7681;--vscode-editor-findMatchBackground:#515c6a;--vscode-editor-findRangeHighlightBackground:#3a3d4166;--vscode-font-family:'Segoe UI',system-ui,sans-serif;--vscode-font-size:13px;--vscode-font-weight:normal;--vscode-editor-font-family:Consolas,'Courier New',monospace;--vscode-editor-font-size:14px;}html,body{background:#1e1e1e;color:#cccccc;}";

const INIT_SCRIPT: &str = r#"
(() => {
  let state = null;
  try { state = JSON.parse(sessionStorage.getItem("__nanti_state")); } catch {}
  const api = {
    postMessage: (message) => window.ipc.postMessage(JSON.stringify({ type: "message", message })),
    getState: () => state,
    setState: (s) => { state = s; try { sessionStorage.setItem("__nanti_state", JSON.stringify(s)); } catch {} return s; },
  };
  let acquired = false;
  window.acquireVsCodeApi = () => {
    if (acquired) throw new Error("An instance of the VS Code API has already been acquired");
    acquired = true;
    return api;
  };
  const applyTheme = () => {
    const root = document.documentElement;
    if (!root || document.getElementById("__nanti_theme")) return;
    const style = document.createElement("style");
    style.id = "__nanti_theme";
    style.textContent = "__THEME_CSS__";
    (document.head || root).appendChild(style);
    root.setAttribute("data-vscode-theme-kind", "vscode-dark");
    root.setAttribute("data-vscode-theme-id", "Nanti Dark");
    if (document.body) document.body.classList.add("vscode-dark");
  };
  applyTheme();
  document.addEventListener("readystatechange", applyTheme);
  document.addEventListener("DOMContentLoaded", applyTheme);
  window.__nantiDeliver = (message) => window.dispatchEvent(new MessageEvent("message", { data: message }));
  // Page errors are surfaced to the host log (diagnostics only; the host ignores them unless asked).
  const diag = (kind, message) => window.ipc.postMessage(JSON.stringify({ type: "diag", kind, message: String(message).slice(0, 300) }));
  window.addEventListener("error", (e) => diag("error", e.message + " @" + (e.filename || "").slice(-60) + ":" + e.lineno));
  window.addEventListener("unhandledrejection", (e) => diag("rejection", e.reason && e.reason.message || e.reason));
})();
"#;

fn js_literal(v: &Value) -> String {
    v.to_string()
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

pub struct Panel {
    webview: WebView,
    state: Arc<Mutex<ResourceState>>,
    events: Receiver<UiEvent>,
    to_host: Sender<HostCommand>,
    loaded: Arc<AtomicBool>,
    html_version: u64,
    queued: Vec<Value>,
    last_bounds: Option<(i32, i32, i32, i32)>,
    diag_log: Option<PathBuf>,
    inspect_due: Option<std::time::Instant>,
}

impl Panel {
    pub fn new(
        window: &Window,
        events: Receiver<UiEvent>,
        to_host: Sender<HostCommand>,
        diag_log: Option<PathBuf>,
    ) -> Result<Self, wry::Error> {
        let state = Arc::new(Mutex::new(ResourceState::default()));
        let loaded = Arc::new(AtomicBool::new(false));
        let protocol_state = state.clone();
        let load_flag = loaded.clone();
        let ipc_tx = to_host.clone();
        let ipc_diag = diag_log.clone();
        let webview = WebViewBuilder::new()
            .with_https_scheme(true)
            .with_initialization_script(INIT_SCRIPT.replace("__THEME_CSS__", THEME_CSS))
            .with_background_color((30, 30, 30, 255))
            .with_custom_protocol("nanti-resource".into(), move |_id, request| {
                let served = {
                    let st = protocol_state.lock().unwrap();
                    serve(&st, request.uri().path())
                };
                Response::builder()
                    .status(served.status)
                    .header("Content-Type", served.mime)
                    .header("Cache-Control", "no-store")
                    .body(Cow::Owned(served.body))
                    .unwrap()
            })
            .with_on_page_load_handler(move |event, _url| {
                if matches!(event, PageLoadEvent::Finished) {
                    load_flag.store(true, Ordering::SeqCst);
                }
            })
            .with_ipc_handler(move |req| {
                if let Ok(v) = serde_json::from_str::<Value>(req.body()) {
                    if v["type"] == "message" {
                        let _ = ipc_tx.send(HostCommand::WebviewMessage(v["message"].clone()));
                    } else if v["type"] == "diag" {
                        selftest_log(&ipc_diag, &format!("page {}: {}", v["kind"], v["message"]));
                    }
                }
            })
            .with_bounds(Rect {
                position: LogicalPosition::new(0, 0).into(),
                size: LogicalSize::new(1, 1).into(),
            })
            .build_as_child(window)?;
        Ok(Self {
            webview,
            state,
            events,
            to_host,
            loaded,
            html_version: 0,
            queued: Vec::new(),
            last_bounds: None,
            diag_log,
            inspect_due: None,
        })
    }

    pub fn focus_parent(&self) {
        let _ = self.webview.focus_parent();
    }

    pub fn set_visible(&self, visible: bool) {
        let _ = self.webview.set_visible(visible);
    }

    pub fn set_area(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let b = (x, y, w.max(1), h.max(1));
        if self.last_bounds == Some(b) {
            return;
        }
        self.last_bounds = Some(b);
        let _ = self.webview.set_bounds(Rect {
            position: LogicalPosition::new(b.0, b.1).into(),
            size: LogicalSize::new(b.2, b.3).into(),
        });
    }

    pub fn resolve_external(&self, id: String, approved: bool) {
        let _ = self
            .to_host
            .send(HostCommand::ResolveExternal { id, approved });
    }

    pub fn doc_open(&self, uri: String, language_id: String, version: u64, text: String) {
        let _ = self.to_host.send(HostCommand::DocOpen {
            uri,
            language_id,
            version,
            text,
        });
    }

    pub fn doc_change(&self, uri: String, version: u64, text: String) {
        let _ = self
            .to_host
            .send(HostCommand::DocChange { uri, version, text });
    }

    pub fn doc_save(&self, uri: String) {
        let _ = self.to_host.send(HostCommand::DocSave { uri });
    }

    pub fn doc_close(&self, uri: String) {
        let _ = self.to_host.send(HostCommand::DocClose { uri });
    }

    pub fn pump(&mut self) -> Vec<UiEvent> {
        let mut ui = Vec::new();
        while let Ok(ev) = self.events.try_recv() {
            match ev {
                UiEvent::Roots(roots) => self.state.lock().unwrap().roots = roots,
                UiEvent::Html(html) => {
                    self.state.lock().unwrap().html = html;
                    self.html_version += 1;
                    self.loaded.store(false, Ordering::SeqCst);
                    self.inspect_due = None;
                    let _ = self
                        .webview
                        .load_url(&format!("{ORIGIN}/html?v={}", self.html_version));
                }
                UiEvent::Message(m) => self.queued.push(m),
                other @ (UiEvent::StatusItem { .. }
                | UiEvent::OpenExternal { .. }
                | UiEvent::TerminalCreate { .. }
                | UiEvent::TerminalSendText { .. }
                | UiEvent::TerminalShow { .. }
                | UiEvent::TerminalDispose { .. }) => ui.push(other),
            }
        }
        if self.html_version > 0 && self.loaded.load(Ordering::SeqCst) {
            if self.diag_log.is_some() {
                let due = *self
                    .inspect_due
                    .get_or_insert_with(|| std::time::Instant::now() + Duration::from_secs(4));
                if std::time::Instant::now() >= due {
                    self.inspect_due = Some(std::time::Instant::now() + Duration::from_secs(3600));
                    let log = self.diag_log.clone();
                    let _ = self.webview.evaluate_script_with_callback(
                        "JSON.stringify({title: document.title, bodyChildren: document.body ? document.body.children.length : -1, \
                         textLen: document.body ? document.body.innerText.length : -1, \
                         text: document.body ? document.body.innerText.replace(/\\s+/g, ' ').slice(0, 160) : ''})",
                        move |result| selftest_log(&log, &format!("dom after load: {result}")),
                    );
                }
            }
            for m in self.queued.drain(..) {
                let _ = self
                    .webview
                    .evaluate_script(&format!("window.__nantiDeliver({})", js_literal(&m)));
            }
        }
        ui
    }
}

impl Drop for Panel {
    fn drop(&mut self) {
        let _ = self.to_host.send(HostCommand::Shutdown);
    }
}

fn selftest_log(path: &Option<PathBuf>, line: &str) {
    use std::io::Write;
    if let Some(p) = path
        && let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
    {
        let _ = writeln!(f, "{line}");
    }
}

pub fn spawn_worker(
    cfg: HostConfig,
) -> (Receiver<UiEvent>, Sender<HostCommand>, Sender<HostCommand>) {
    let (ui_tx, ui_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let shutdown_tx = cmd_tx.clone();
    std::thread::spawn(move || {
        let log = cfg.selftest_log.clone();
        let result = run_worker(&cfg, &ui_tx, &cmd_rx);
        if let Err(e) = result {
            eprintln!("extension host worker stopped: {e}");
            selftest_log(&log, &format!("FAIL worker: {e}"));
        }
    });
    (ui_rx, cmd_tx, shutdown_tx)
}

fn run_worker(
    cfg: &HostConfig,
    ui: &Sender<UiEvent>,
    cmds: &Receiver<HostCommand>,
) -> Result<(), app::host::HostError> {
    let log = &cfg.selftest_log;
    let workspace: Vec<&Path> = cfg.workspace.iter().map(PathBuf::as_path).collect();
    let mut host = ExtensionHost::spawn_with_workspace(&cfg.node, &cfg.entry, &workspace)?;
    let id = host.load_extension(&cfg.extension_dir)?;
    let state = host.activate_extension(&id)?;
    selftest_log(log, &format!("extension {id} state={state}"));
    let handle = host.resolve_view(&cfg.view_type)?;
    selftest_log(log, &format!("view resolved handle={handle}"));

    let mut seen_pong = false;
    let mut seen_secret_denied = false;
    let mut pending_webview: Vec<app::host::DeferredResponse> = Vec::new();
    loop {
        while let Ok(cmd) = cmds.try_recv() {
            match cmd {
                HostCommand::Shutdown => return host.shutdown(),
                HostCommand::ResolveExternal { id, approved } => {
                    let _ = host.request(
                        "env.openExternal.resolve",
                        json!({ "id": id, "approved": approved }),
                    )?;
                }
                HostCommand::WebviewMessage(m) => {
                    selftest_log(log, &format!("webview -> host: {m}"));
                    pending_webview.push(host.send_webview_message_deferred(&handle, m)?);
                }
                HostCommand::DocOpen {
                    uri,
                    language_id,
                    version,
                    text,
                } => {
                    let _ = host.request(
                        "workspace.didOpenTextDocument",
                        json!({ "uri": uri, "languageId": language_id, "version": version, "text": text }),
                    )?;
                }
                HostCommand::DocChange { uri, version, text } => {
                    let _ = host.request(
                        "workspace.didChangeTextDocument",
                        json!({ "uri": uri, "version": version, "text": text }),
                    )?;
                }
                HostCommand::DocSave { uri } => {
                    let _ = host.request("workspace.didSaveTextDocument", json!({ "uri": uri }))?;
                }
                HostCommand::DocClose { uri } => {
                    let _ =
                        host.request("workspace.didCloseTextDocument", json!({ "uri": uri }))?;
                }
            }
        }
        let mut index = 0;
        while index < pending_webview.len() {
            match pending_webview[index].try_complete() {
                Some(Ok(_)) => {
                    drop(pending_webview.swap_remove(index));
                }
                Some(Err(error)) => return Err(error),
                None => index += 1,
            }
        }
        let Some(n) = host.recv_notification(Duration::from_millis(20)) else {
            continue;
        };
        let for_us = n.params["handle"] == json!(handle);
        match n.method.as_str() {
            "webview.setHtml" if for_us => {
                selftest_log(log, "html received");
                let _ = ui.send(UiEvent::Html(
                    n.params["html"].as_str().unwrap_or_default().to_owned(),
                ));
            }
            "webview.setOptions" if for_us => {
                let roots = n.params["options"]["localResourceRoots"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(PathBuf::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let _ = ui.send(UiEvent::Roots(roots));
            }
            "webview.postMessage" if for_us => {
                selftest_log(log, &format!("host -> webview: {}", n.params["message"]));
                let _ = ui.send(UiEvent::Message(n.params["message"].clone()));
            }
            "env.openExternal" => {
                if let (Some(id), Some(url)) = (n.params["id"].as_str(), n.params["url"].as_str()) {
                    let _ = ui.send(UiEvent::OpenExternal {
                        id: id.to_owned(),
                        url: url.to_owned(),
                    });
                }
            }
            "window.statusBarItem" => {
                let _ = ui.send(UiEvent::StatusItem {
                    id: n.params["id"].as_str().unwrap_or_default().to_owned(),
                    visible: n.params["visible"].as_bool().unwrap_or(false),
                    text: n.params["text"].as_str().unwrap_or_default().to_owned(),
                });
            }
            "terminal.create" => {
                let id = n.params["id"].as_str().unwrap_or_default().to_owned();
                let name = n.params["name"].as_str().unwrap_or("Terminal").to_owned();
                let shell_path = n.params["shellPath"].as_str().map(str::to_owned);
                let shell_args = n.params["shellArgs"].as_array().map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_owned))
                        .collect()
                });
                let cwd = n.params["cwd"].as_str().map(PathBuf::from);
                let _ = ui.send(UiEvent::TerminalCreate {
                    id,
                    name,
                    shell_path,
                    shell_args,
                    cwd,
                });
            }
            "terminal.sendText" => {
                let id = n.params["id"].as_str().unwrap_or_default().to_owned();
                let text = n.params["text"].as_str().unwrap_or_default().to_owned();
                let add_new_line = n.params["addNewLine"].as_bool().unwrap_or(true);
                let _ = ui.send(UiEvent::TerminalSendText {
                    id,
                    text,
                    add_new_line,
                });
            }
            "terminal.show" => {
                let id = n.params["id"].as_str().unwrap_or_default().to_owned();
                let preserve_focus = n.params["preserveFocus"].as_bool().unwrap_or(false);
                let _ = ui.send(UiEvent::TerminalShow { id, preserve_focus });
            }
            "terminal.dispose" => {
                let id = n.params["id"].as_str().unwrap_or_default().to_owned();
                let _ = ui.send(UiEvent::TerminalDispose { id });
            }
            "window.showInformationMessage" => {
                let msg = n.params["message"].as_str().unwrap_or_default();
                eprintln!("[ext] {msg}");
                selftest_log(log, &format!("info: {msg}"));
                seen_pong |= msg == "pong 1";
                seen_secret_denied |= msg == "secret 403";
                if log.is_some() && seen_pong && seen_secret_denied {
                    selftest_log(log, "PASS webview roundtrip + resource root enforcement");
                    std::process::exit(0);
                }
            }
            _ => {}
        }
    }
}
