use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const OP_PING: u32 = 3;
const OP_PONG: u32 = 4;
const MAX_FRAME: usize = 64 * 1024;
// Discord looks the application up before it answers and goes silent when asked too often, so tries are spaced out instead of hammered.
const READY_TIMEOUT: Duration = Duration::from_secs(8);
const SETTLE: Duration = Duration::from_millis(600);
const QUICK_RETRIES: u32 = 1;
const RETRY_AFTER: Duration = Duration::from_secs(15);
const TICK: Duration = Duration::from_millis(100);
const MIN_SEND_GAP: Duration = Duration::from_secs(15);
// Debug builds always have it so it can be worked on; a release build needs `--features discord`.
pub const AVAILABLE: bool = cfg!(feature = "discord") || cfg!(debug_assertions);

const SETTINGS_FILE: &str = "discord.json";
// The public id of nanti's own Discord application; it names the activity and carries no secret.
pub const APPLICATION_ID: &str = "1055749894926774333";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscordSettings {
    pub enabled: bool,
    pub client_id: String,
    pub show_file: bool,
    pub show_workspace: bool,
    pub show_time: bool,
}

impl Default for DiscordSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            client_id: std::env::var("NANTI_DISCORD_CLIENT_ID")
                .ok()
                .and_then(|v| valid_client_id(&v))
                .unwrap_or_else(|| APPLICATION_ID.to_owned()),
            show_file: true,
            show_workspace: true,
            show_time: true,
        }
    }
}

pub fn valid_client_id(text: &str) -> Option<String> {
    let t = text.trim();
    (t.len() >= 17 && t.len() <= 20 && t.bytes().all(|b| b.is_ascii_digit())).then(|| t.to_owned())
}

impl DiscordSettings {
    pub fn load(dir: &Path) -> Self {
        let mut s = Self::default();
        let Some(v) = std::fs::read_to_string(dir.join(SETTINGS_FILE))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')).ok())
        else {
            return s;
        };
        let flag = |k: &str, d: bool| v.get(k).and_then(Value::as_bool).unwrap_or(d);
        s.enabled = flag("enabled", s.enabled);
        s.show_file = flag("showFile", s.show_file);
        s.show_workspace = flag("showWorkspace", s.show_workspace);
        s.show_time = flag("showTime", s.show_time);
        if let Some(id) = v
            .get("clientId")
            .and_then(Value::as_str)
            .and_then(valid_client_id)
        {
            s.client_id = id;
        }
        s
    }

    pub fn save(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&json!({
            "enabled": self.enabled,
            "clientId": self.client_id,
            "showFile": self.show_file,
            "showWorkspace": self.show_workspace,
            "showTime": self.show_time,
        }))
        .map_err(|e| e.to_string())?;
        let tmp = dir.join(format!("{SETTINGS_FILE}.tmp"));
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, dir.join(SETTINGS_FILE)).map_err(|e| e.to_string())
    }
}

/// Who the Discord app says is signed in: the name, and what is needed to find the profile picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    pub id: String,
    pub avatar: Option<String>,
}

impl Profile {
    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            id: String::new(),
            avatar: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscordStatus {
    Off,
    NeedsId,
    Connecting,
    Connected(Profile),
    Unavailable(String),
}

impl DiscordStatus {
    pub fn describe(&self) -> String {
        match self {
            Self::Off => "Rich Presence is off.".to_owned(),
            Self::NeedsId => "Set an Application ID to connect.".to_owned(),
            Self::Connecting => "Connecting to the Discord app...".to_owned(),
            Self::Connected(user) => format!("Connected as {}.", user.name),
            Self::Unavailable(why) => why.clone(),
        }
    }

    pub fn connected(&self) -> bool {
        matches!(self, Self::Connected(_))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Presence {
    pub details: String,
    pub state: Option<String>,
    pub start_ms: Option<u64>,
    // Asset key uploaded in the Discord developer portal, and the text shown when hovering it.
    pub small_image: Option<(String, String)>,
}

/// The asset key of the icon that stands for a file's language; the PNGs are made by scripts/gen-discord-assets.ps1.
pub fn language_asset(ext: &str) -> Option<&'static str> {
    Some(match ext.to_ascii_lowercase().as_str() {
        "rs" => "rust",
        "go" => "go",
        "dart" => "dart",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "jsx" | "tsx" => "react",
        "py" => "python",
        "html" | "htm" | "xhtml" => "html",
        "css" | "scss" | "sass" | "less" => "css",
        "json" | "jsonc" => "json",
        "md" | "markdown" => "markdown",
        "ps1" | "psm1" | "psd1" => "powershell",
        "sh" | "bash" | "zsh" => "shell",
        "php" => "php",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "cs" => "csharp",
        "rb" => "ruby",
        "swift" => "swift",
        "lua" => "lua",
        "sql" => "sql",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "vue" => "vue",
        "svelte" => "svelte",
        _ => return None,
    })
}

enum Cmd {
    Config { enabled: bool, client_id: String },
    Presence(Option<Presence>),
    Shutdown,
}

pub fn encode_frame(op: u32, payload: &Value) -> Vec<u8> {
    let body = payload.to_string().into_bytes();
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&op.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

pub fn activity_payload(pid: u32, nonce: u64, presence: Option<&Presence>) -> Value {
    let activity = presence.map(|p| {
        let mut a = json!({ "details": clip(&p.details), "instance": false });
        if let Some(s) = p.state.as_deref().map(clip).filter(|s| !s.is_empty()) {
            a["state"] = json!(s);
        }
        if let Some(t) = p.start_ms {
            a["timestamps"] = json!({ "start": t });
        }
        let mut assets = json!({ "large_image": "nanti", "large_text": "Nanti IDE" });
        if let Some((key, text)) = &p.small_image {
            assets["small_image"] = json!(key);
            assets["small_text"] = json!(clip(text));
        }
        a["assets"] = assets;
        a
    });
    json!({
        "cmd": "SET_ACTIVITY",
        "args": { "pid": pid, "activity": activity },
        "nonce": nonce.to_string(),
    })
}

fn clip(text: &str) -> String {
    let t = text.trim();
    let mut s: String = t.chars().take(128).collect();
    // Discord rejects one-character fields.
    if s.chars().count() == 1 {
        s.push(' ');
    }
    s
}

pub(crate) trait Pipe {
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()>;
    fn peek(&mut self, head: &mut [u8; 8]) -> io::Result<usize>;
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()>;
}

fn read_frame(pipe: &mut dyn Pipe) -> io::Result<Option<(u32, Value)>> {
    let mut head = [0u8; 8];
    let total = pipe.peek(&mut head)?;
    if total < 8 {
        return Ok(None);
    }
    let op = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
    let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
    if len > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    if total < 8 + len {
        return Ok(None);
    }
    let mut buf = vec![0u8; 8 + len];
    pipe.read_exact(&mut buf)?;
    let value = serde_json::from_slice(&buf[8..]).unwrap_or(Value::Null);
    Ok(Some((op, value)))
}

fn user_profile(ready: &Value) -> Profile {
    let user = &ready["data"]["user"];
    let text = |k: &str| {
        user.get(k)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    Profile {
        name: text("global_name")
            .or_else(|| text("username"))
            .unwrap_or_else(|| "Discord user".to_owned()),
        id: text("id").unwrap_or_default(),
        avatar: text("avatar"),
    }
}

/// Fetches the profile picture once from Discord's image server and keeps it in the config folder.
pub fn fetch_avatar(dir: &Path, id: &str, hash: &str) -> Option<PathBuf> {
    let id_ok = (15..=20).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit());
    let hash_ok = !hash.is_empty()
        && hash.len() <= 40
        && hash.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
    if !id_ok || !hash_ok {
        return None;
    }
    let file = dir.join(format!("discord-avatar-{hash}.png"));
    if file.is_file() {
        return Some(file);
    }
    let url = format!("https://cdn.discordapp.com/avatars/{id}/{hash}.png?size=128");
    let mut cmd = std::process::Command::new("curl.exe");
    cmd.args([
        "-sSL",
        "--fail",
        "--connect-timeout",
        "8",
        "--max-time",
        "12",
    ])
    .args(["--max-filesize", "500000"])
    .arg(&url);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().ok().filter(|o| o.status.success())?;
    if !out.stdout.starts_with(b"\x89PNG") {
        return None;
    }
    std::fs::create_dir_all(dir).ok()?;
    std::fs::write(&file, &out.stdout).ok()?;
    Some(file)
}

enum Ended {
    Reconfigured(bool, String),
    Shutdown,
    Lost(String),
}

/// Clears the status and says goodbye, so Discord frees the connection at once instead of waiting for the pipe to break.
fn goodbye(pipe: &mut dyn Pipe, pid: u32) {
    let _ = pipe.write_all(&encode_frame(OP_FRAME, &activity_payload(pid, 0, None)));
    let _ = pipe.write_all(&encode_frame(OP_CLOSE, &json!({})));
}

fn session(
    pipe: &mut dyn Pipe,
    client_id: &str,
    rx: &Receiver<Cmd>,
    status: &Mutex<DiscordStatus>,
    pending: &mut Option<Presence>,
    pid: u32,
) -> Ended {
    let lost = |e: io::Error| Ended::Lost(format!("Lost the Discord connection: {e}"));
    let hello = json!({ "v": 1, "client_id": client_id });
    if let Err(e) = pipe.write_all(&encode_frame(OP_HANDSHAKE, &hello)) {
        return lost(e);
    }
    let began = Instant::now();
    let user = loop {
        match read_frame(pipe) {
            Err(e) => return lost(e),
            Ok(Some((OP_FRAME, v))) if v["evt"] == "READY" => break user_profile(&v),
            Ok(Some((OP_FRAME, v))) if v["evt"] == "ERROR" => {
                let msg = v["data"]["message"]
                    .as_str()
                    .unwrap_or("Discord refused the Application ID");
                return Ended::Lost(msg.to_owned());
            }
            Ok(Some((OP_CLOSE, v))) => {
                let msg = v["message"]
                    .as_str()
                    .unwrap_or("Discord closed the connection");
                return Ended::Lost(msg.to_owned());
            }
            Ok(Some(_)) => {}
            Ok(None) => {
                if began.elapsed() > READY_TIMEOUT {
                    return Ended::Lost("Discord did not answer".to_owned());
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };
    if let Ok(mut s) = status.lock() {
        *s = DiscordStatus::Connected(user);
    }
    let mut nonce = 0u64;
    let mut send = |pipe: &mut dyn Pipe, p: Option<&Presence>| {
        nonce += 1;
        pipe.write_all(&encode_frame(OP_FRAME, &activity_payload(pid, nonce, p)))
    };
    if let Some(p) = pending.as_ref()
        && let Err(e) = send(pipe, Some(p))
    {
        return lost(e);
    }
    loop {
        loop {
            match read_frame(pipe) {
                Err(e) => return lost(e),
                Ok(None) => break,
                Ok(Some((OP_PING, v))) => {
                    if let Err(e) = pipe.write_all(&encode_frame(OP_PONG, &v)) {
                        return lost(e);
                    }
                }
                Ok(Some((OP_CLOSE, _))) => {
                    return Ended::Lost("Discord closed the connection".to_owned());
                }
                Ok(Some(_)) => {}
            }
        }
        match rx.recv_timeout(TICK) {
            Ok(Cmd::Presence(p)) => {
                if let Err(e) = send(pipe, p.as_ref()) {
                    return lost(e);
                }
                *pending = p;
            }
            Ok(Cmd::Config { enabled, client_id }) => {
                goodbye(pipe, pid);
                return Ended::Reconfigured(enabled, client_id);
            }
            Ok(Cmd::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                goodbye(pipe, pid);
                return Ended::Shutdown;
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

#[cfg(windows)]
mod os {
    use std::fs::{File, OpenOptions};
    use std::io::{self, Read, Write};
    use std::os::windows::io::AsRawHandle;

    use super::Pipe;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn PeekNamedPipe(
            pipe: *mut core::ffi::c_void,
            buffer: *mut core::ffi::c_void,
            size: u32,
            read: *mut u32,
            total: *mut u32,
            left: *mut u32,
        ) -> i32;
    }

    pub struct NamedPipe(File);

    pub fn open() -> Option<NamedPipe> {
        (0..10).find_map(|i| {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(format!(r"\\.\pipe\discord-ipc-{i}"))
                .ok()
                .map(NamedPipe)
        })
    }

    impl Pipe for NamedPipe {
        fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.0.write_all(bytes)
        }

        fn peek(&mut self, head: &mut [u8; 8]) -> io::Result<usize> {
            let (mut read, mut total, mut left) = (0u32, 0u32, 0u32);
            // SAFETY: the handle is a live pipe owned by `self`; the buffer is 8 writable bytes and the
            // three out-pointers refer to locals that outlive the call.
            let ok = unsafe {
                PeekNamedPipe(
                    self.0.as_raw_handle(),
                    head.as_mut_ptr().cast(),
                    8,
                    &raw mut read,
                    &raw mut total,
                    &raw mut left,
                )
            };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(total as usize)
        }

        fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
            self.0.read_exact(buf)
        }
    }
}

#[cfg(windows)]
fn open_pipe() -> Option<Box<dyn Pipe>> {
    os::open().map(|p| Box::new(p) as Box<dyn Pipe>)
}

#[cfg(not(windows))]
fn open_pipe() -> Option<Box<dyn Pipe>> {
    None
}

struct Wanted {
    enabled: bool,
    client_id: String,
    pending: Option<Presence>,
    failures: u32,
}

impl Wanted {
    /// Returns false when the owner asked the worker to stop.
    fn apply(&mut self, cmd: Cmd) -> bool {
        match cmd {
            Cmd::Config { enabled, client_id } => {
                self.enabled = enabled;
                self.client_id = client_id;
            }
            Cmd::Presence(p) => self.pending = p,
            Cmd::Shutdown => return false,
        }
        true
    }
}

/// Sleeps until the retry delay passes or a command arrives. False means stop the worker.
/// How long to wait before the next try, growing so a Discord that is slow to answer is not flooded.
fn retry_delay(failures: u32) -> Duration {
    match failures {
        0 | 1 => Duration::from_secs(5),
        2 => RETRY_AFTER,
        _ => Duration::from_secs(30),
    }
}

fn wait_for_change(rx: &Receiver<Cmd>, w: &mut Wanted, after: Duration) -> bool {
    match rx.recv_timeout(after) {
        Ok(cmd) => w.apply(cmd),
        Err(RecvTimeoutError::Timeout) => true,
        Err(RecvTimeoutError::Disconnected) => false,
    }
}

fn worker(rx: Receiver<Cmd>, status: Arc<Mutex<DiscordStatus>>, settle: Duration) {
    if !settle.is_zero() {
        std::thread::sleep(settle);
    }
    let set = |s: DiscordStatus| {
        if let Ok(mut g) = status.lock() {
            *g = s;
        }
    };
    let mut w = Wanted {
        enabled: false,
        client_id: String::new(),
        pending: None,
        failures: 0,
    };
    loop {
        if !w.enabled || w.client_id.is_empty() {
            set(if w.enabled {
                DiscordStatus::NeedsId
            } else {
                DiscordStatus::Off
            });
            match rx.recv() {
                Ok(cmd) => {
                    if !w.apply(cmd) {
                        return;
                    }
                    continue;
                }
                Err(_) => return,
            }
        }
        set(DiscordStatus::Connecting);
        let Some(mut pipe) = open_pipe() else {
            set(DiscordStatus::Unavailable(
                "The Discord app is not running. Start it and it connects by itself.".to_owned(),
            ));
            if !wait_for_change(&rx, &mut w, RETRY_AFTER) {
                return;
            }
            continue;
        };
        let id = w.client_id.clone();
        match session(
            pipe.as_mut(),
            &id,
            &rx,
            &status,
            &mut w.pending,
            std::process::id(),
        ) {
            Ended::Shutdown => return,
            Ended::Reconfigured(enabled, client_id) => {
                w.enabled = enabled;
                w.client_id = client_id;
            }
            Ended::Lost(why) => {
                let was_connected = status.lock().is_ok_and(|s| s.connected());
                w.failures = if was_connected { 1 } else { w.failures + 1 };
                // The first few failures retry quietly under "Connecting"; only a lasting failure is shown.
                set(if w.failures > QUICK_RETRIES {
                    DiscordStatus::Unavailable(why)
                } else {
                    DiscordStatus::Connecting
                });
                let delay = retry_delay(w.failures);
                if !wait_for_change(&rx, &mut w, delay) {
                    return;
                }
            }
        }
    }
}

pub struct DiscordLink {
    tx: Sender<Cmd>,
    status: Arc<Mutex<DiscordStatus>>,
    thread: Option<JoinHandle<()>>,
}

impl DiscordLink {
    pub fn start() -> Self {
        Self::start_after(Duration::ZERO)
    }

    pub fn start_after(settle: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(DiscordStatus::Off));
        let shared = status.clone();
        let thread = std::thread::Builder::new()
            .name("discord-rpc".into())
            .spawn(move || worker(rx, shared, settle))
            .ok();
        Self { tx, status, thread }
    }

    pub fn configure(&self, settings: &DiscordSettings) {
        let _ = self.tx.send(Cmd::Config {
            enabled: settings.enabled,
            client_id: settings.client_id.clone(),
        });
    }

    pub fn set_presence(&self, presence: Option<Presence>) {
        let _ = self.tx.send(Cmd::Presence(presence));
    }

    pub fn status(&self) -> DiscordStatus {
        self.status.lock().map_or(DiscordStatus::Off, |s| s.clone())
    }
}

impl Drop for DiscordLink {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Shutdown);
        // The worker only waits on the channel, so it exits at once; never block the UI on it.
        drop(self.thread.take());
    }
}

pub struct DiscordState {
    pub settings: DiscordSettings,
    link: Option<DiscordLink>,
    sent: Option<Presence>,
    sent_at: Option<Instant>,
    pub started_ms: u64,
    avatar_hash: Option<String>,
    pub avatar_path: Option<PathBuf>,
    // The last account seen, kept on screen while a refresh reconnects.
    pub last_profile: Option<Profile>,
    avatar_rx: Option<Receiver<Option<PathBuf>>>,
}

impl Default for DiscordState {
    fn default() -> Self {
        Self {
            settings: DiscordSettings::default(),
            link: None,
            sent: None,
            sent_at: None,
            avatar_hash: None,
            avatar_path: None,
            last_profile: None,
            avatar_rx: None,
            started_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis() as u64),
        }
    }
}

impl DiscordState {
    pub fn status(&self) -> DiscordStatus {
        match &self.link {
            Some(l) => l.status(),
            None if !self.settings.enabled => DiscordStatus::Off,
            None if self.settings.client_id.is_empty() => DiscordStatus::NeedsId,
            None => DiscordStatus::Connecting,
        }
    }

    /// Applies new settings; starts the worker only once the feature is wanted.
    pub fn reconfigure(&mut self) {
        if self.settings.enabled {
            if self.link.is_none() {
                self.link = Some(DiscordLink::start());
            }
        } else {
            // Off means off: dropping the link ends its worker thread and closes the pipe to Discord.
            self.link = None;
            self.avatar_hash = None;
            self.avatar_path = None;
            self.last_profile = None;
            self.avatar_rx = None;
        }
        if let Some(link) = &self.link {
            link.configure(&self.settings);
        }
        self.sent = None;
        self.sent_at = None;
    }

    pub fn reconnect(&mut self) {
        self.link = None;
        if self.settings.enabled {
            self.link = Some(DiscordLink::start_after(SETTLE));
            if let Some(link) = &self.link {
                link.configure(&self.settings);
            }
            self.sent = None;
            self.sent_at = None;
        }
    }

    /// Sends the wanted presence when it changed, at most once per 15 seconds. Returns whether the
    /// settings page should refresh (the status may have changed).
    /// Starts the one-time download of the profile picture once Discord says who is signed in.
    pub fn pump_avatar(&mut self, dir: Option<&Path>) -> bool {
        let mut changed = false;
        if let Some(rx) = &self.avatar_rx {
            match rx.try_recv() {
                Ok(path) => {
                    self.avatar_path = path;
                    self.avatar_rx = None;
                    changed = true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.avatar_rx = None,
            }
        }
        let status = self.status();
        if let DiscordStatus::Connected(p) = &status
            && self.last_profile.as_ref() != Some(p)
        {
            self.last_profile = Some(p.clone());
            changed = true;
        }
        let (Some(dir), DiscordStatus::Connected(profile)) = (dir, status) else {
            return changed;
        };
        let Some(hash) = profile.avatar else {
            return changed;
        };
        if self.avatar_hash.as_deref() == Some(hash.as_str()) || self.avatar_rx.is_some() {
            return changed;
        }
        self.avatar_hash = Some(hash.clone());
        let (tx, rx) = mpsc::channel();
        let (dir, id) = (dir.to_path_buf(), profile.id);
        std::thread::spawn(move || {
            let _ = tx.send(fetch_avatar(&dir, &id, &hash));
        });
        self.avatar_rx = Some(rx);
        changed
    }

    pub fn pump(&mut self, wanted: Option<Presence>) -> bool {
        let Some(link) = &self.link else {
            return false;
        };
        if !self.settings.enabled || wanted == self.sent {
            return false;
        }
        let due = self.sent_at.is_none_or(|t| t.elapsed() >= MIN_SEND_GAP);
        if !due {
            return false;
        }
        link.set_presence(wanted.clone());
        self.sent = wanted;
        self.sent_at = Some(Instant::now());
        true
    }
}

#[cfg(test)]
mod asset_tests {
    use super::*;

    #[test]
    fn retries_back_off_instead_of_flooding_discord() {
        let waits: Vec<_> = (1..=6).map(|n| retry_delay(n).as_millis()).collect();
        assert_eq!(waits, [5000, 15000, 30000, 30000, 30000, 30000]);
    }

    #[test]
    fn the_ready_event_gives_the_name_id_and_picture_hash() {
        let ready = json!({ "data": { "user": { "id": "123456789012345678", "username": "thepar", "global_name": "", "avatar": "abc123" } } });
        let p = user_profile(&ready);
        assert_eq!(p.name, "thepar");
        assert_eq!(p.id, "123456789012345678");
        assert_eq!(p.avatar.as_deref(), Some("abc123"));
        let bare = user_profile(&json!({ "data": { "user": { "username": "x" } } }));
        assert_eq!((bare.id.as_str(), bare.avatar), ("", None));
    }

    #[test]
    fn avatar_requests_only_go_out_for_well_formed_ids_and_hashes() {
        let dir = std::env::temp_dir().join("nanti-avatar-none");
        assert!(fetch_avatar(&dir, "12x", "abc").is_none());
        assert!(fetch_avatar(&dir, "123456789012345678", "../x").is_none());
        assert!(fetch_avatar(&dir, "123456789012345678", "").is_none());
    }

    #[test]
    fn a_cached_picture_is_used_without_any_download() {
        let dir = std::env::temp_dir().join(format!("nanti-avatar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("discord-avatar-abc123.png");
        std::fs::write(&file, b"cached").unwrap();
        assert_eq!(
            fetch_avatar(&dir, "123456789012345678", "abc123"),
            Some(file)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn turning_it_off_drops_the_worker_and_turning_it_on_starts_one() {
        let mut st = DiscordState::default();
        st.settings.client_id = String::new();
        st.settings.enabled = false;
        st.reconfigure();
        assert!(st.link.is_none(), "nothing runs while it is off");
        st.settings.enabled = true;
        st.reconfigure();
        assert!(st.link.is_some());
        st.settings.enabled = false;
        st.reconfigure();
        assert!(st.link.is_none(), "switching off ends the worker");
        assert_eq!(st.status(), DiscordStatus::Off);
    }

    #[test]
    fn the_activity_carries_the_app_logo_and_the_language_icon() {
        let p = Presence {
            details: "Editing a.rs".into(),
            small_image: Some(("rust".into(), "Rust".into())),
            ..Presence::default()
        };
        let v = activity_payload(1, 2, Some(&p));
        let assets = &v["args"]["activity"]["assets"];
        assert_eq!(assets["large_image"], "nanti");
        assert_eq!(assets["small_image"], "rust");
        assert_eq!(assets["small_text"], "Rust");
        let none = activity_payload(1, 3, Some(&Presence::default()));
        assert!(none["args"]["activity"]["assets"]["small_image"].is_null());
    }

    #[test]
    fn languages_map_to_asset_keys_and_unknown_files_have_none() {
        assert_eq!(language_asset("RS"), Some("rust"));
        assert_eq!(language_asset("tsx"), Some("react"));
        assert_eq!(language_asset("ps1"), Some("powershell"));
        assert_eq!(language_asset("exe"), None);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    #[derive(Default)]
    struct Fake {
        inbound: VecDeque<u8>,
        written: Vec<u8>,
    }

    impl Fake {
        fn push(&mut self, op: u32, v: &Value) {
            self.inbound.extend(encode_frame(op, v));
        }

        fn frames(&self) -> Vec<(u32, Value)> {
            let mut out = Vec::new();
            let mut i = 0;
            while i + 8 <= self.written.len() {
                let op = u32::from_le_bytes(self.written[i..i + 4].try_into().unwrap());
                let len =
                    u32::from_le_bytes(self.written[i + 4..i + 8].try_into().unwrap()) as usize;
                out.push((
                    op,
                    serde_json::from_slice(&self.written[i + 8..i + 8 + len]).unwrap(),
                ));
                i += 8 + len;
            }
            out
        }
    }

    impl Pipe for Fake {
        fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.written.extend_from_slice(bytes);
            Ok(())
        }

        fn peek(&mut self, head: &mut [u8; 8]) -> io::Result<usize> {
            for (i, b) in self.inbound.iter().take(8).enumerate() {
                head[i] = *b;
            }
            Ok(self.inbound.len())
        }

        fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
            for b in buf.iter_mut() {
                *b = self
                    .inbound
                    .pop_front()
                    .ok_or(io::ErrorKind::UnexpectedEof)?;
            }
            Ok(())
        }
    }

    fn ready(name: &str) -> Value {
        json!({ "cmd": "DISPATCH", "evt": "READY", "data": { "v": 1, "user": { "username": "login", "global_name": name } } })
    }

    #[test]
    fn frames_round_trip_with_little_endian_headers() {
        let bytes = encode_frame(OP_FRAME, &json!({ "a": 1 }));
        assert_eq!(&bytes[..4], &[1, 0, 0, 0]);
        assert_eq!(&bytes[4..8], &[7, 0, 0, 0]);
        let mut f = Fake::default();
        f.inbound.extend(bytes);
        assert_eq!(
            read_frame(&mut f).unwrap(),
            Some((OP_FRAME, json!({ "a": 1 })))
        );
        assert_eq!(read_frame(&mut f).unwrap(), None);
    }

    #[test]
    fn a_half_received_frame_is_left_alone_until_it_is_complete() {
        let mut f = Fake::default();
        let bytes = encode_frame(OP_FRAME, &json!({ "evt": "X" }));
        f.inbound.extend(&bytes[..bytes.len() - 2]);
        assert_eq!(read_frame(&mut f).unwrap(), None);
        f.inbound.extend(&bytes[bytes.len() - 2..]);
        assert!(read_frame(&mut f).unwrap().is_some());
    }

    #[test]
    fn an_oversized_frame_is_an_error_not_an_allocation() {
        let mut f = Fake::default();
        f.inbound.extend(OP_FRAME.to_le_bytes());
        f.inbound.extend(u32::MAX.to_le_bytes());
        assert!(read_frame(&mut f).is_err());
    }

    #[test]
    fn handshake_ready_then_presence_then_clear_then_shutdown() {
        let mut fake = Fake::default();
        fake.push(OP_FRAME, &ready("Deva"));
        let (tx, rx) = mpsc::channel();
        let status = Mutex::new(DiscordStatus::Connecting);
        tx.send(Cmd::Presence(Some(Presence {
            details: "Editing main.rs".into(),
            state: Some("Workspace: nanti-ide".into()),
            start_ms: Some(1_700_000_000_000),
            ..Presence::default()
        })))
        .unwrap();
        tx.send(Cmd::Presence(None)).unwrap();
        tx.send(Cmd::Shutdown).unwrap();
        let mut pending = None;
        let ended = session(
            &mut fake,
            "123456789012345678",
            &rx,
            &status,
            &mut pending,
            77,
        );
        assert!(matches!(ended, Ended::Shutdown));
        assert_eq!(
            *status.lock().unwrap(),
            DiscordStatus::Connected(Profile::named("Deva"))
        );
        let frames = fake.frames();
        assert_eq!(
            frames[0],
            (
                OP_HANDSHAKE,
                json!({ "v": 1, "client_id": "123456789012345678" })
            )
        );
        let set = &frames[1].1;
        assert_eq!(set["cmd"], "SET_ACTIVITY");
        assert_eq!(set["args"]["pid"], 77);
        assert_eq!(set["args"]["activity"]["details"], "Editing main.rs");
        assert_eq!(set["args"]["activity"]["state"], "Workspace: nanti-ide");
        assert_eq!(
            set["args"]["activity"]["timestamps"]["start"],
            1_700_000_000_000u64
        );
        assert!(
            frames[2].1["args"]["activity"].is_null(),
            "None clears the presence"
        );
    }

    #[test]
    fn ping_is_answered_with_the_same_payload() {
        let mut fake = Fake::default();
        fake.push(OP_FRAME, &ready("A"));
        fake.push(OP_PING, &json!({ "n": 5 }));
        let (tx, rx) = mpsc::channel();
        tx.send(Cmd::Shutdown).unwrap();
        let status = Mutex::new(DiscordStatus::Connecting);
        let _ = session(&mut fake, "123456789012345678", &rx, &status, &mut None, 1);
        assert!(fake.frames().contains(&(OP_PONG, json!({ "n": 5 }))));
    }

    #[test]
    fn a_refused_application_id_reports_discords_message() {
        let mut fake = Fake::default();
        fake.push(
            OP_CLOSE,
            &json!({ "code": 4000, "message": "Invalid Client ID" }),
        );
        let (_tx, rx) = mpsc::channel::<Cmd>();
        let status = Mutex::new(DiscordStatus::Connecting);
        match session(&mut fake, "123456789012345678", &rx, &status, &mut None, 1) {
            Ended::Lost(why) => assert_eq!(why, "Invalid Client ID"),
            _ => panic!("expected a lost connection"),
        }
    }

    #[test]
    fn the_last_presence_is_replayed_after_a_reconnect() {
        let mut fake = Fake::default();
        fake.push(OP_FRAME, &ready("A"));
        let (tx, rx) = mpsc::channel();
        tx.send(Cmd::Shutdown).unwrap();
        let mut pending = Some(Presence {
            details: "Idle".into(),
            ..Presence::default()
        });
        let status = Mutex::new(DiscordStatus::Connecting);
        let _ = session(
            &mut fake,
            "123456789012345678",
            &rx,
            &status,
            &mut pending,
            1,
        );
        assert_eq!(fake.frames()[1].1["args"]["activity"]["details"], "Idle");
    }

    #[test]
    fn text_is_trimmed_and_never_a_single_character() {
        assert_eq!(clip("  x "), "x ");
        assert_eq!(clip(&"é".repeat(200)).chars().count(), 128);
    }

    #[test]
    fn application_ids_must_be_snowflakes() {
        assert!(valid_client_id("123456789012345678").is_some());
        assert!(valid_client_id(" 123456789012345678 ").is_some());
        for bad in ["", "123", "12345678901234567x", "123456789012345678901"] {
            assert!(valid_client_id(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn settings_save_and_load_and_reject_a_bad_id() {
        let dir = std::env::temp_dir().join(format!("nanti-discord-{}", std::process::id()));
        let s = DiscordSettings {
            enabled: true,
            client_id: "123456789012345678".into(),
            show_file: false,
            show_workspace: true,
            show_time: false,
        };
        s.save(&dir).unwrap();
        assert_eq!(DiscordSettings::load(&dir), s);
        std::fs::write(
            dir.join(SETTINGS_FILE),
            r#"{"enabled": true, "clientId": "nope"}"#,
        )
        .unwrap();
        let loaded = DiscordSettings::load(&dir);
        assert!(loaded.enabled && !loaded.client_id.contains("nope"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nothing_starts_until_the_feature_is_switched_on() {
        let mut st = DiscordState::default();
        st.settings.enabled = false;
        st.reconfigure();
        assert!(st.link.is_none());
        assert_eq!(st.status(), DiscordStatus::Off);
        assert!(!st.pump(Some(Presence {
            details: "x".into(),
            ..Presence::default()
        })));
    }
}
