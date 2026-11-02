
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use serde_json::{Value, json};
use syntax::{Diagnostic, Language};

pub enum LspCommand {
    Init {
        root_uri: String,
    },
    DidOpen {
        uri: String,
        language_id: String,
        version: u64,
        text: String,
    },
    DidChange {
        uri: String,
        version: u64,
        text: String,
    },
    DidSave {
        uri: String,
    },
    DidClose {
        uri: String,
    },
}

#[derive(Debug)]
pub enum LspEvent {
    Diagnostics {
        path: PathBuf,
        items: Vec<Diagnostic>,
    },
}

pub struct LspServer {
    child: Child,
    cmd_tx: Sender<LspCommand>,
    event_rx: Receiver<LspEvent>,
    pub opened: HashSet<PathBuf>,
}

impl Drop for LspServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl LspServer {
    pub fn spawn(lang: Language, root: &Path) -> Option<Self> {
        let (bin, args): (PathBuf, Vec<&str>) = match lang {
            Language::Rust => {
                let p = which_or_cargo("rust-analyzer.exe")?;
                (p, vec![])
            }
            Language::Go => {
                let p = which_or_go("gopls.exe")?;
                (p, vec![])
            }
            _ => return None,
        };

        let mut proc_cmd = Command::new(&bin);
        proc_cmd.args(args);
        proc_cmd.current_dir(root);
        proc_cmd.stdin(Stdio::piped());
        proc_cmd.stdout(Stdio::piped());
        proc_cmd.stderr(Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            proc_cmd.creation_flags(0x0800_0000);
        }

        let mut child = proc_cmd.spawn().ok()?;
        let stdin = child.stdin.take()?;
        let stdout = child.stdout.take()?;

        let (cmd_tx, cmd_rx) = mpsc::channel::<LspCommand>();
        let (event_tx, event_rx) = mpsc::channel::<LspEvent>();

        thread::spawn(move || {
            let mut writer = stdin;
            while let Ok(cmd) = cmd_rx.recv() {
                let msg = match cmd {
                    LspCommand::Init { root_uri } => json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "initialize",
                        "params": {
                            "processId": std::process::id(),
                            "rootUri": root_uri,
                            "capabilities": {
                                "textDocument": {
                                    "synchronization": {
                                        "openClose": true,
                                        "change": 1,
                                        "save": true
                                    },
                                    "publishDiagnostics": {
                                        "relatedInformation": true
                                    }
                                }
                            }
                        }
                    }),
                    LspCommand::DidOpen {
                        uri,
                        language_id,
                        version,
                        text,
                    } => json!({
                        "jsonrpc": "2.0",
                        "method": "textDocument/didOpen",
                        "params": {
                            "textDocument": {
                                "uri": uri,
                                "languageId": language_id,
                                "version": version,
                                "text": text
                            }
                        }
                    }),
                    LspCommand::DidChange { uri, version, text } => json!({
                        "jsonrpc": "2.0",
                        "method": "textDocument/didChange",
                        "params": {
                            "textDocument": {
                                "uri": uri,
                                "version": version
                            },
                            "contentChanges": [{ "text": text }]
                        }
                    }),
                    LspCommand::DidSave { uri } => json!({
                        "jsonrpc": "2.0",
                        "method": "textDocument/didSave",
                        "params": {
                            "textDocument": { "uri": uri }
                        }
                    }),
                    LspCommand::DidClose { uri } => json!({
                        "jsonrpc": "2.0",
                        "method": "textDocument/didClose",
                        "params": {
                            "textDocument": { "uri": uri }
                        }
                    }),
                };

                let body = match serde_json::to_vec(&msg) {
                    Ok(b) => b,
                    Err(_) => continue,
                };
                let header = format!("Content-Length: {}\r\n\r\n", body.len());
                if writer.write_all(header.as_bytes()).is_err() || writer.write_all(&body).is_err() || writer.flush().is_err() {
                    break;
                }

                if matches!(msg.get("method").and_then(|m| m.as_str()), Some("initialize")) {
                    let init_done = json!({
                        "jsonrpc": "2.0",
                        "method": "initialized",
                        "params": {}
                    });
                    if let Ok(b) = serde_json::to_vec(&init_done) {
                        let h = format!("Content-Length: {}\r\n\r\n", b.len());
                        let _ = writer.write_all(h.as_bytes());
                        let _ = writer.write_all(&b);
                        let _ = writer.flush();
                    }
                }
            }
        });

        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut content_length: Option<usize> = None;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        break;
                    }
                    if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
                        if let Ok(len) = rest.trim().parse::<usize>() {
                            content_length = Some(len);
                        }
                    }
                }

                let Some(len) = content_length else { continue };
                let mut body = vec![0u8; len];
                if reader.read_exact(&mut body).is_err() {
                    return;
                }

                let val: Value = match serde_json::from_slice(&body) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                if val.get("method").and_then(|m| m.as_str()) == Some("textDocument/publishDiagnostics") {
                    if let Some(params) = val.get("params") {
                        if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
                            if let Some(path) = uri_to_path(uri) {
                                let mut items = Vec::new();
                                if let Some(arr) = params.get("diagnostics").and_then(|d| d.as_array()) {
                                    for d in arr {
                                        let line = d.pointer("/range/start/line").and_then(|l| l.as_u64()).unwrap_or(0) as usize;
                                        let col = d.pointer("/range/start/character").and_then(|c| c.as_u64()).unwrap_or(0) as usize;
                                        let end_col = d.pointer("/range/end/character").and_then(|c| c.as_u64()).unwrap_or(col as u64 + 1) as usize;
                                        let len = end_col.saturating_sub(col).max(1);
                                        let message = d.get("message").and_then(|m| m.as_str()).unwrap_or("Error").to_owned();

                                        items.push(Diagnostic {
                                            line,
                                            col,
                                            len,
                                            message,
                                        });
                                    }
                                }
                                let _ = event_tx.send(LspEvent::Diagnostics { path, items });
                            }
                        }
                    }
                }
            }
        });

        let server = Self {
            child,
            cmd_tx,
            event_rx,
            opened: HashSet::new(),
        };

        let root_uri = path_to_uri(root);
        let _ = server.cmd_tx.send(LspCommand::Init { root_uri });

        Some(server)
    }

    pub fn did_open(&mut self, path: &Path, lang: Language, version: u64, text: &str) {
        let uri = path_to_uri(path);
        let lang_id = match lang {
            Language::Rust => "rust",
            Language::Go => "go",
            _ => "plaintext",
        };
        self.opened.insert(path.to_path_buf());
        let _ = self.cmd_tx.send(LspCommand::DidOpen {
            uri,
            language_id: lang_id.to_owned(),
            version,
            text: text.to_owned(),
        });
    }

    pub fn did_change(&mut self, path: &Path, version: u64, text: &str) {
        let uri = path_to_uri(path);
        let _ = self.cmd_tx.send(LspCommand::DidChange {
            uri,
            version,
            text: text.to_owned(),
        });
    }

    pub fn did_save(&mut self, path: &Path) {
        let uri = path_to_uri(path);
        let _ = self.cmd_tx.send(LspCommand::DidSave { uri });
    }

    pub fn did_close(&mut self, path: &Path) {
        let uri = path_to_uri(path);
        self.opened.remove(path);
        let _ = self.cmd_tx.send(LspCommand::DidClose { uri });
    }

    pub fn pump_events(&mut self) -> Vec<LspEvent> {
        let mut events = Vec::new();
        while let Ok(ev) = self.event_rx.try_recv() {
            events.push(ev);
        }
        events
    }
}

#[derive(Default)]
pub struct LspManager {
    servers: HashMap<Language, LspServer>,
    unavailable: HashSet<Language>,
}

impl LspManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure_server(&mut self, lang: Language, root: &Path) -> Option<&mut LspServer> {
        if !matches!(lang, Language::Rust | Language::Go) || self.unavailable.contains(&lang) {
            return None;
        }
        if !self.servers.contains_key(&lang) {
            match LspServer::spawn(lang, root) {
                Some(s) => {
                    self.servers.insert(lang, s);
                }
                None => {
                    self.unavailable.insert(lang);
                }
            }
        }
        self.servers.get_mut(&lang)
    }

    pub fn get_mut(&mut self, lang: Language) -> Option<&mut LspServer> {
        self.servers.get_mut(&lang)
    }

    pub fn shutdown_all(&mut self) {
        self.servers.clear();
        self.unavailable.clear();
    }

    pub fn pump(&mut self) -> Vec<LspEvent> {
        let mut all = Vec::new();
        for server in self.servers.values_mut() {
            all.extend(server.pump_events());
        }
        all
    }
}

fn which_or_cargo(name: &str) -> Option<PathBuf> {
    if let Ok(p) = which_path(name) {
        return Some(p);
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        let cargo = PathBuf::from(home).join(".cargo").join("bin").join(name);
        if cargo.is_file() {
            return Some(cargo);
        }
    }
    None
}

fn which_or_go(name: &str) -> Option<PathBuf> {
    if let Ok(p) = which_path(name) {
        return Some(p);
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        let gopath = PathBuf::from(home).join("go").join("bin").join(name);
        if gopath.is_file() {
            return Some(gopath);
        }
    }
    None
}

fn which_path(name: &str) -> Result<PathBuf, ()> {
    let path = std::env::var("PATH").map_err(|_| ())?;
    for p in std::env::split_paths(&path) {
        let target = p.join(name);
        if target.is_file() {
            return Ok(target);
        }
    }
    Err(())
}

pub fn path_to_uri(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    if let Ok(url) = url::Url::parse(uri) {
        if let Ok(path) = url.to_file_path() {
            return Some(path);
        }
    }
    let stripped = uri.strip_prefix("file:///")?;
    Some(PathBuf::from(stripped.replace('/', "\\")))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn alive(pid: u32) -> bool {
        let out = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .expect("tasklist");
        String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
    }

    #[test]
    fn dropping_a_server_kills_its_process() {
        let child = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Start-Sleep 120"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .expect("spawn powershell");
        let pid = child.id();
        assert!(alive(pid), "the stand-in server should be running");
        let (cmd_tx, _cmd_rx) = mpsc::channel();
        let (_event_tx, event_rx) = mpsc::channel();
        let server = LspServer {
            child,
            cmd_tx,
            event_rx,
            opened: HashSet::new(),
        };
        drop(server);
        assert!(!alive(pid), "the process must not outlive its LspServer");
    }

    #[test]
    fn a_missing_server_binary_is_not_retried_on_every_call() {
        let mut m = LspManager::new();
        m.unavailable.insert(Language::Rust);
        assert!(m.ensure_server(Language::Rust, Path::new(".")).is_none());
        m.shutdown_all();
        assert!(m.unavailable.is_empty());
    }
}
