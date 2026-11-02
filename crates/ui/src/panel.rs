//! WebView2 child surface for one extension webview view, plus the host worker thread that feeds it.
//!
//! WebView2 only hosts third-party extension HTML (PRD 4.2); the editor itself stays native.

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

/// What the host worker tells the UI thread.
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

/// What the UI thread tells the host worker.
#[derive(Debug)]
pub enum HostCommand {
    WebviewMessage(Value),
    ResolveExternal { id: String, approved: bool },
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
    /// Log milestones to this file and exit(0) once the mock extension round-trip is proven.
    pub selftest_log: Option<PathBuf>,
}

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
  window.__nantiDeliver = (message) => window.dispatchEvent(new MessageEvent("message", { data: message }));
  // Page errors are surfaced to the host log (diagnostics only; the host ignores them unless asked).
  const diag = (kind, message) => window.ipc.postMessage(JSON.stringify({ type: "diag", kind, message: String(message).slice(0, 300) }));
  window.addEventListener("error", (e) => diag("error", e.message + " @" + (e.filename || "").slice(-60) + ":" + e.lineno));
  window.addEventListener("unhandledrejection", (e) => diag("rejection", e.reason && e.reason.message || e.reason));
})();
"#;

fn js_literal(v: &Value) -> String {
    // JSON is valid JS except for these two separators.
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
    /// When the current document finished loading and has not been inspected yet (diagnostics).
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
            .with_initialization_script(INIT_SCRIPT)
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
                // Only well-formed `{type:"message"}` envelopes reach the extension.
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

    /// Show or hide the native webview surface. It floats above GPUI content, so hiding the
    /// right panel in the layout must hide it too.
    pub fn set_visible(&self, visible: bool) {
        let _ = self.webview.set_visible(visible);
    }

    /// Place the webview over the right-hand panel area (logical pixels).
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

    /// Send a user's browser-opening decision to the extension host.
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
        let _ = self.to_host.send(HostCommand::DocChange {
            uri,
            version,
            text,
        });
    }

    pub fn doc_save(&self, uri: String) {
        let _ = self.to_host.send(HostCommand::DocSave { uri });
    }

    pub fn doc_close(&self, uri: String) {
        let _ = self.to_host.send(HostCommand::DocClose { uri });
    }

    /// Apply pending host events; returns the ones the UI itself must show (status items and consent requests).
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
                | UiEvent::TerminalDispose { .. }) => {
                    ui.push(other)
                }
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

/// Start the Node host on a worker thread: load + activate the extension, resolve its view and
/// pump notifications to the UI. Returns the UI-side channel ends.
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
                    let _ = host.request("workspace.didCloseTextDocument", json!({ "uri": uri }))?;
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
                    a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect()
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
                let _ = ui.send(UiEvent::TerminalShow {
                    id,
                    preserve_focus,
                });
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
