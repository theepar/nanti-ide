
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const PROTOCOL_VERSION: u32 = 1;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("host closed the pipe")]
    Closed,
    #[error("host did not answer within {0:?}")]
    Timeout(Duration),
    #[error("protocol version mismatch: host={host}, ide={ide}")]
    VersionMismatch { host: u32, ide: u32 },
    #[error("host error {code}: {message}")]
    Remote { code: String, message: String },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    protocol_version: u32,
    request_id: u64,
    method: &'a str,
    params: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    request_id: u64,
    result: Option<Value>,
    error: Option<RemoteError>,
}

#[derive(Deserialize)]
struct RemoteError {
    code: String,
    message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub method: String,
    pub params: Value,
}

type Pending = Arc<Mutex<HashMap<u64, Sender<Response>>>>;

pub struct DeferredResponse {
    id: u64,
    receiver: Receiver<Response>,
    pending: Pending,
    started: Instant,
    timeout: Duration,
}

impl DeferredResponse {
    fn decode(response: Response) -> Result<Value, HostError> {
        match response.error {
            Some(e) => Err(HostError::Remote {
                code: e.code,
                message: e.message,
            }),
            None => Ok(response.result.unwrap_or(Value::Null)),
        }
    }

    pub fn try_complete(&self) -> Option<Result<Value, HostError>> {
        match self.receiver.try_recv() {
            Ok(response) => Some(Self::decode(response)),
            Err(TryRecvError::Disconnected) => Some(Err(HostError::Closed)),
            Err(TryRecvError::Empty) if self.started.elapsed() < self.timeout => None,
            Err(TryRecvError::Empty) => {
                self.pending.lock().unwrap().remove(&self.id);
                Some(Err(HostError::Timeout(self.timeout)))
            }
        }
    }

    fn wait(self) -> Result<Value, HostError> {
        match self.receiver.recv_timeout(self.timeout) {
            Ok(response) => Self::decode(response),
            Err(RecvTimeoutError::Timeout) => Err(HostError::Timeout(self.timeout)),
            Err(RecvTimeoutError::Disconnected) => Err(HostError::Closed),
        }
    }
}

impl Drop for DeferredResponse {
    fn drop(&mut self) {
        self.pending.lock().unwrap().remove(&self.id);
    }
}

pub struct ExtensionHost {
    child: Child,
    stdin: ChildStdin,
    next_id: u64,
    pending: Pending,
    closed: Arc<AtomicBool>,
    notifications: Receiver<Notification>,
    reader: Option<JoinHandle<()>>,
    timeout: Duration,
}

impl ExtensionHost {
    pub fn spawn(node: &str, entry: &Path) -> Result<Self, HostError> {
        Self::spawn_with_workspace(node, entry, &[])
    }

    pub fn spawn_with_workspace(
        node: &str,
        entry: &Path,
        workspace_folders: &[&Path],
    ) -> Result<Self, HostError> {
        let mut child = Command::new(node)
            .arg(entry)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child.stdin.take().ok_or(HostError::Closed)?;
        let stdout = child.stdout.take().ok_or(HostError::Closed)?;

        let pending: Pending = Arc::default();
        let closed = Arc::new(AtomicBool::new(false));
        let (note_tx, note_rx) = mpsc::channel();
        let reader = {
            let pending = pending.clone();
            let closed = closed.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines() {
                    let Ok(line) = line else { break };
                    let Ok(raw) = serde_json::from_str::<Value>(&line) else {
                        continue;
                    };
                    if raw.get("requestId").is_none() {
                        let _ = note_tx.send(Notification {
                            method: raw["method"].as_str().unwrap_or_default().to_owned(),
                            params: raw["params"].clone(),
                        });
                        continue;
                    }
                    if let Ok(resp) = serde_json::from_value::<Response>(raw) {
                        if let Some(tx) = pending.lock().unwrap().remove(&resp.request_id) {
                            let _ = tx.send(resp);
                        }
                    }
                }
                closed.store(true, Ordering::SeqCst);
                pending.lock().unwrap().clear();
            })
        };

        let mut host = Self {
            child,
            stdin,
            next_id: 1,
            pending,
            closed,
            notifications: note_rx,
            reader: Some(reader),
            timeout: DEFAULT_TIMEOUT,
        };
        let folders: Vec<String> = workspace_folders
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        let hello = host.request(
            "initialize",
            json!({ "ideVersion": env!("CARGO_PKG_VERSION"), "workspaceFolders": folders }),
        )?;
        let remote = hello["protocolVersion"].as_u64().unwrap_or(0) as u32;
        if remote != PROTOCOL_VERSION {
            let _ = host.kill();
            return Err(HostError::VersionMismatch {
                host: remote,
                ide: PROTOCOL_VERSION,
            });
        }
        Ok(host)
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    fn dispatch(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<DeferredResponse, HostError> {
        let request_id = self.next_id;
        self.next_id += 1;
        let raw = serde_json::to_string(&Request {
            protocol_version: PROTOCOL_VERSION,
            request_id,
            method,
            params,
        })?;
        if self.closed.load(Ordering::SeqCst) {
            return Err(HostError::Closed);
        }
        let (tx, receiver) = mpsc::channel();
        self.pending.lock().unwrap().insert(request_id, tx);
        let deferred = DeferredResponse {
            id: request_id,
            receiver,
            pending: self.pending.clone(),
            started: Instant::now(),
            timeout,
        };
        if let Err(e) = writeln!(self.stdin, "{raw}").and_then(|()| self.stdin.flush()) {
            return Err(if self.closed.load(Ordering::SeqCst) {
                HostError::Closed
            } else {
                e.into()
            });
        }
        Ok(deferred)
    }

    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, HostError> {
        self.dispatch(method, params, self.timeout)?.wait()
    }

    pub fn send_webview_message_deferred(
        &mut self,
        handle: &str,
        message: Value,
    ) -> Result<DeferredResponse, HostError> {
        self.dispatch(
            "webview.message",
            json!({ "handle": handle, "message": message }),
            Duration::from_secs(130),
        )
    }

    pub fn load_extension(&mut self, dir: &Path) -> Result<String, HostError> {
        let v = self.request("extension.load", json!({ "path": dir.to_string_lossy() }))?;
        Ok(v["id"].as_str().unwrap_or_default().to_owned())
    }

    pub fn activate_extension(&mut self, id: &str) -> Result<String, HostError> {
        let v = self.request("extension.activate", json!({ "id": id }))?;
        Ok(v["state"].as_str().unwrap_or_default().to_owned())
    }

    pub fn deactivate_extension(&mut self, id: &str) -> Result<(), HostError> {
        self.request("extension.deactivate", json!({ "id": id }))
            .map(|_| ())
    }

    pub fn execute_command(&mut self, command: &str, args: Vec<Value>) -> Result<Value, HostError> {
        self.request(
            "commands.execute",
            json!({ "command": command, "args": args }),
        )
    }

    pub fn resolve_view(&mut self, view_type: &str) -> Result<String, HostError> {
        let v = self.request("webview.resolveView", json!({ "viewType": view_type }))?;
        Ok(v["handle"].as_str().unwrap_or_default().to_owned())
    }

    pub fn send_webview_message(&mut self, handle: &str, message: Value) -> Result<(), HostError> {
        self.request(
            "webview.message",
            json!({ "handle": handle, "message": message }),
        )
        .map(|_| ())
    }

    pub fn dispose_webview(&mut self, handle: &str) -> Result<(), HostError> {
        self.request("webview.dispose", json!({ "handle": handle }))
            .map(|_| ())
    }

    pub fn handle_uri(&mut self, uri: &str) -> Result<(), HostError> {
        self.request("uri.handle", json!({ "uri": uri }))
            .map(|_| ())
    }

    pub fn take_notifications(&mut self) -> Vec<Notification> {
        self.notifications.try_iter().collect()
    }

    pub fn recv_notification(&mut self, timeout: Duration) -> Option<Notification> {
        self.notifications.recv_timeout(timeout).ok()
    }

    pub fn shutdown(mut self) -> Result<(), HostError> {
        let _ = self.request("shutdown", Value::Null);
        self.child.wait()?;
        if let Some(r) = self.reader.take() {
            let _ = r.join();
        }
        Ok(())
    }

    pub fn kill(&mut self) -> Result<(), HostError> {
        let _ = self.child.kill();
        self.child.wait()?;
        if let Some(r) = self.reader.take() {
            let _ = r.join();
        }
        Ok(())
    }
}

impl Drop for ExtensionHost {
    fn drop(&mut self) {
        // Never leave an orphaned Node process behind.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
