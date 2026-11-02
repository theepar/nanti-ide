use std::fmt;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const STATUS_TIMEOUT: Duration = Duration::from_secs(20);
pub const DIFF_TIMEOUT: Duration = Duration::from_secs(10);
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(30);
pub const COMMIT_TIMEOUT: Duration = Duration::from_secs(120);
pub const NETWORK_TIMEOUT: Duration = Duration::from_secs(180);
pub const MAX_STATUS_ENTRIES: usize = 5_000;
pub const MAX_DIFF_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_DIFF_LINES: usize = 20_000;
const MAX_STDERR: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitError {
    NotInstalled,
    NotARepo,
    Timeout(String),
    Failed {
        command: String,
        code: Option<i32>,
        stderr: String,
    },
    Io(String),
    Rejected(String),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::NotInstalled => {
                write!(f, "git was not found on PATH; install Git for Windows")
            }
            GitError::NotARepo => write!(f, "this folder is not inside a Git repository"),
            GitError::Timeout(c) => write!(f, "git {c} timed out"),
            GitError::Failed {
                command, stderr, ..
            } => {
                let msg = stderr.trim();
                if msg.is_empty() {
                    write!(f, "git {command} failed")
                } else {
                    write!(f, "git {command} failed: {msg}")
                }
            }
            GitError::Io(e) => write!(f, "{e}"),
            GitError::Rejected(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for GitError {}

struct Output {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: String,
    truncated: bool,
}

fn base_command(program: &str, cwd: &Path, read_only: bool) -> Command {
    let mut c = Command::new(program);
    c.current_dir(cwd)
        .args([
            "--no-pager",
            "--literal-pathspecs",
            "-c",
            "core.quotepath=off",
            // never run a repository-configured helper just because we looked at the repo
            "-c",
            "core.fsmonitor=false",
        ])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if read_only {
        c.env("GIT_OPTIONAL_LOCKS", "0");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c
}

fn run(
    program: &str,
    cwd: &Path,
    args: &[&str],
    stdin: Option<&[u8]>,
    timeout: Duration,
    max_stdout: usize,
    read_only: bool,
) -> Result<Output, GitError> {
    let label = args.first().copied().unwrap_or("git").to_owned();
    let mut cmd = base_command(program, cwd, read_only);
    cmd.args(args);
    cmd.stdin(if stdin.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    let mut child = cmd.spawn().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => GitError::NotInstalled,
        _ => GitError::Io(format!("cannot start git: {e}")),
    })?;
    if let (Some(data), Some(mut pipe)) = (stdin, child.stdin.take()) {
        let data = data.to_vec();
        std::thread::spawn(move || {
            let _ = pipe.write_all(&data);
        });
    }
    let mut out = child.stdout.take().expect("piped");
    let mut err = child.stderr.take().expect("piped");
    // Drain both pipes on threads so a chatty child can never block on a full pipe.
    let out_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 16 * 1024];
        let mut truncated = false;
        while let Ok(n) = out.read(&mut chunk) {
            if n == 0 {
                break;
            }
            if buf.len() < max_stdout {
                let take = n.min(max_stdout - buf.len());
                buf.extend_from_slice(&chunk[..take]);
                truncated |= take < n;
            } else {
                truncated = true;
            }
        }
        (buf, truncated)
    });
    let err_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = err.by_ref().take(MAX_STDERR as u64).read_to_end(&mut buf);
        let _ = std::io::copy(&mut err, &mut std::io::sink());
        String::from_utf8_lossy(&buf).into_owned()
    });

    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {
                if start.elapsed() > timeout {
                    kill_tree(&mut child);
                    // Do not join the readers: a grandchild that survived could still hold the pipes.
                    drop((out_thread, err_thread));
                    return Err(GitError::Timeout(label));
                }
                std::thread::sleep(Duration::from_millis(4));
            }
            Err(e) => return Err(GitError::Io(e.to_string())),
        }
    };
    let (stdout, truncated) = out_thread.join().unwrap_or_default();
    let stderr = err_thread.join().unwrap_or_default();
    Ok(Output {
        code: status.code(),
        stdout,
        stderr,
        truncated,
    })
}

fn kill_tree(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &child.id().to_string()])
            .creation_flags(0x0800_0000)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Change {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Untracked,
    Conflicted,
}

impl Change {
    pub fn letter(self) -> char {
        match self {
            Change::Modified => 'M',
            Change::Added => 'A',
            Change::Deleted => 'D',
            Change::Renamed => 'R',
            Change::Copied => 'C',
            Change::TypeChanged => 'T',
            Change::Untracked => 'U',
            Change::Conflicted => '!',
        }
    }

    fn from_code(c: char) -> Option<Change> {
        Some(match c {
            'M' => Change::Modified,
            'A' => Change::Added,
            'D' => Change::Deleted,
            'R' => Change::Renamed,
            'C' => Change::Copied,
            'T' => Change::TypeChanged,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub orig_path: Option<String>,
    pub staged: Option<Change>,
    pub unstaged: Option<Change>,
    pub conflicted: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Status {
    pub branch: Option<String>,
    pub detached: bool,
    pub oid: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub entries: Vec<Entry>,
    pub truncated: bool,
}

impl Status {
    pub fn staged(&self) -> impl Iterator<Item = &Entry> {
        self.entries
            .iter()
            .filter(|e| !e.conflicted && e.staged.is_some())
    }

    pub fn changes(&self) -> impl Iterator<Item = &Entry> {
        self.entries
            .iter()
            .filter(|e| !e.conflicted && e.unstaged.is_some())
    }

    pub fn conflicts(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter().filter(|e| e.conflicted)
    }

    pub fn is_clean(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entry_for(&self, rel: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.path == rel)
    }

    pub fn dir_has_changes(&self, rel: &str) -> bool {
        let prefix = if rel.is_empty() || rel.ends_with('/') {
            rel.to_owned()
        } else {
            format!("{rel}/")
        };
        self.entries.iter().any(|e| e.path.starts_with(&prefix))
    }

    pub fn covers<'a>(&'a self, rel: &str) -> Option<&'a Entry> {
        self.entries
            .iter()
            .find(|e| e.path == rel || (e.path.ends_with('/') && rel.starts_with(&e.path)))
    }
}

pub fn parse_status(bytes: &[u8]) -> Status {
    let text = String::from_utf8_lossy(bytes);
    let mut st = Status::default();
    let mut records = text.split('\0').filter(|r| !r.is_empty());
    while let Some(rec) = records.next() {
        if let Some(h) = rec.strip_prefix("# ") {
            if let Some(v) = h.strip_prefix("branch.oid ") {
                st.oid = (v != "(initial)").then(|| v.to_owned());
            } else if let Some(v) = h.strip_prefix("branch.head ") {
                if v == "(detached)" {
                    st.detached = true;
                } else {
                    st.branch = Some(v.to_owned());
                }
            } else if let Some(v) = h.strip_prefix("branch.upstream ") {
                st.upstream = Some(v.to_owned());
            } else if let Some(v) = h.strip_prefix("branch.ab ") {
                let mut it = v.split_whitespace();
                st.ahead = it
                    .next()
                    .and_then(|a| a.trim_start_matches('+').parse().ok())
                    .unwrap_or(0);
                st.behind = it
                    .next()
                    .and_then(|b| b.trim_start_matches('-').parse().ok())
                    .unwrap_or(0);
            }
            continue;
        }
        let mut chars = rec.chars();
        let kind = chars.next().unwrap_or(' ');
        let entry = match kind {
            '1' => {
                let f: Vec<&str> = rec.splitn(9, ' ').collect();
                (f.len() == 9).then(|| make_entry(f[1], f[8], None, false))
            }
            '2' => {
                let f: Vec<&str> = rec.splitn(10, ' ').collect();
                let orig = records.next().map(str::to_owned);
                (f.len() == 10).then(|| make_entry(f[1], f[9], orig, false))
            }
            'u' => {
                let f: Vec<&str> = rec.splitn(11, ' ').collect();
                (f.len() == 11).then(|| make_entry(f[1], f[10], None, true))
            }
            '?' => Some(Entry {
                path: rec[2..].to_owned(),
                orig_path: None,
                staged: None,
                unstaged: Some(Change::Untracked),
                conflicted: false,
            }),
            _ => None,
        };
        if let Some(e) = entry {
            if st.entries.len() >= MAX_STATUS_ENTRIES {
                st.truncated = true;
                break;
            }
            st.entries.push(e);
        }
    }
    st.entries.sort_by_key(|e| e.path.to_lowercase());
    st
}

fn make_entry(xy: &str, path: &str, orig: Option<String>, conflicted: bool) -> Entry {
    let mut c = xy.chars();
    let (x, y) = (c.next().unwrap_or('.'), c.next().unwrap_or('.'));
    Entry {
        path: path.to_owned(),
        orig_path: orig,
        staged: if conflicted {
            None
        } else {
            Change::from_code(x)
        },
        unstaged: if conflicted {
            None
        } else {
            Change::from_code(y)
        },
        conflicted,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Add,
    Del,
    NoNewline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old: Option<u32>,
    pub new: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub hash: String,
    pub author: String,
    pub time: i64,
    pub refs: Vec<String>,
    pub parents: usize,
    pub subject: String,
}

impl Commit {
    pub fn short(&self) -> &str {
        &self.hash[..self.hash.len().min(7)]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitFile {
    pub letter: char,
    pub path: String,
    pub old: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitDetail {
    pub body: String,
    pub offset_min: i32,
    pub files: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebRemote {
    pub host_label: &'static str,
    pub base: String,
    commit_path: &'static str,
}

impl WebRemote {
    pub fn commit_url(&self, hash: &str) -> Option<String> {
        check_hash(hash).ok()?;
        Some(format!("{}{}{}", self.base, self.commit_path, hash))
    }
}

/// Turns a clone address into the project page of the three well known hosts; anything else is not linked.
pub fn parse_web_remote(remote: &str) -> Option<WebRemote> {
    let remote = remote.trim();
    let (host, path) = if let Some(rest) = remote.strip_prefix("git@") {
        rest.split_once(':')?
    } else if let Some(rest) = remote
        .strip_prefix("https://")
        .or_else(|| remote.strip_prefix("ssh://git@"))
    {
        let rest = rest.split_once('@').map_or(rest, |(_, r)| r);
        rest.split_once('/')?
    } else {
        return None;
    };
    let path = path.trim_matches('/').trim_end_matches(".git");
    let segments: Vec<&str> = path.split('/').collect();
    let clean = |s: &&str| {
        !s.is_empty()
            && *s != "."
            && *s != ".."
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    };
    if segments.len() < 2 || !segments.iter().all(clean) {
        return None;
    }
    let (host_label, commit_path, host) = match host.to_ascii_lowercase().as_str() {
        "github.com" => ("GitHub", "/commit/", "github.com"),
        "gitlab.com" => ("GitLab", "/-/commit/", "gitlab.com"),
        "bitbucket.org" => ("Bitbucket", "/commits/", "bitbucket.org"),
        _ => return None,
    };
    Some(WebRemote {
        host_label,
        base: format!("https://{host}/{}", segments.join("/")),
        commit_path,
    })
}

pub fn parse_commit_detail(text: &str) -> CommitDetail {
    let (head, tail) = text.split_once('\x1e').unwrap_or((text, ""));
    let (body, date) = head.rsplit_once('\x1f').unwrap_or((head, ""));
    let offset_min = date
        .split_whitespace()
        .nth(2)
        .and_then(|z| {
            let sign = if z.starts_with('-') { -1 } else { 1 };
            let digits = z.trim_start_matches(['+', '-']);
            let (h, m) = (digits.get(..2)?, digits.get(2..4)?);
            Some(sign * (h.parse::<i32>().ok()? * 60 + m.parse::<i32>().ok()?))
        })
        .unwrap_or(0);
    let number_before = |word: &str| -> usize {
        tail.split(',')
            .find(|part| part.contains(word))
            .and_then(|part| part.split_whitespace().next())
            .and_then(|n| n.parse().ok())
            .unwrap_or(0)
    };
    CommitDetail {
        body: body.trim().to_owned(),
        offset_min,
        files: number_before("changed"),
        insertions: number_before("insertion"),
        deletions: number_before("deletion"),
    }
}

fn check_hash(hash: &str) -> Result<(), GitError> {
    if (7..=64).contains(&hash.len()) && hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(GitError::Rejected(format!("{hash:?} is not a commit id")))
    }
}

pub fn parse_log(text: &str) -> Vec<Commit> {
    text.split('\x1e')
        .filter_map(|rec| {
            let rec = rec.trim_matches(['\n', '\r']);
            let mut f = rec.splitn(6, '\x1f');
            let hash = f.next()?.trim().to_owned();
            if hash.is_empty() {
                return None;
            }
            let author = f.next()?.to_owned();
            let time = f.next()?.trim().parse().unwrap_or(0);
            let refs = f
                .next()?
                .split(", ")
                .map(str::trim)
                .filter(|r| !r.is_empty())
                .map(str::to_owned)
                .collect();
            let parents = f.next()?.split_whitespace().count();
            let subject = f.next().unwrap_or("").to_owned();
            Some(Commit {
                hash,
                author,
                time,
                refs,
                parents,
                subject,
            })
        })
        .collect()
}

pub fn parse_name_status(text: &str) -> Vec<CommitFile> {
    text.lines()
        .filter_map(|line| {
            let mut cols = line.split('\t');
            let status = cols.next()?.trim();
            let letter = status.chars().next()?;
            let first = cols.next()?.to_owned();
            match (letter, cols.next()) {
                ('R' | 'C', Some(new)) => Some(CommitFile {
                    letter,
                    path: new.to_owned(),
                    old: Some(first),
                }),
                _ => Some(CommitFile {
                    letter,
                    path: first,
                    old: None,
                }),
            }
        })
        .collect()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    pub hunks: Vec<Hunk>,
    pub binary: bool,
    pub truncated: bool,
    pub notes: Vec<String>,
}

impl Diff {
    pub fn added(&self) -> usize {
        self.count(LineKind::Add)
    }

    pub fn removed(&self) -> usize {
        self.count(LineKind::Del)
    }

    fn count(&self, k: LineKind) -> usize {
        self.hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == k)
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.hunks.is_empty() && !self.binary && self.notes.is_empty()
    }
}

pub fn parse_diff(text: &str) -> Diff {
    let mut diff = Diff::default();
    let (mut old, mut new) = (0u32, 0u32);
    let mut total = 0usize;
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(rest) = line.strip_prefix("@@") {
            let (a, c) = parse_hunk_header(rest);
            old = a;
            new = c;
            diff.hunks.push(Hunk {
                header: line.to_owned(),
                lines: Vec::new(),
            });
            continue;
        }
        let Some(hunk) = diff.hunks.last_mut() else {
            if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
                diff.binary = true;
            } else if line.starts_with("new file mode") {
                diff.notes.push("new file".to_owned());
            } else if line.starts_with("deleted file mode") {
                diff.notes.push("deleted file".to_owned());
            } else if let Some(f) = line.strip_prefix("rename from ") {
                diff.notes.push(format!("renamed from {f}"));
            } else if let Some(f) = line.strip_prefix("copy from ") {
                diff.notes.push(format!("copied from {f}"));
            } else if line.starts_with("old mode ") {
                diff.notes
                    .push(line.replace("old mode", "mode changed from"));
            }
            continue;
        };
        total += 1;
        if total > MAX_DIFF_LINES {
            diff.truncated = true;
            break;
        }
        let (kind, text) = match line.chars().next() {
            Some('+') => (LineKind::Add, &line[1..]),
            Some('-') => (LineKind::Del, &line[1..]),
            Some(' ') => (LineKind::Context, &line[1..]),
            Some('\\') => (LineKind::NoNewline, line),
            _ => continue,
        };
        let (o, n) = match kind {
            LineKind::Context => {
                let r = (Some(old), Some(new));
                old += 1;
                new += 1;
                r
            }
            LineKind::Add => {
                let r = (None, Some(new));
                new += 1;
                r
            }
            LineKind::Del => {
                let r = (Some(old), None);
                old += 1;
                r
            }
            LineKind::NoNewline => (None, None),
        };
        hunk.lines.push(DiffLine {
            kind,
            old: o,
            new: n,
            text: text.to_owned(),
        });
    }
    diff
}

fn parse_hunk_header(rest: &str) -> (u32, u32) {
    let mut old = 1;
    let mut new = 1;
    for tok in rest.split_whitespace() {
        if let Some(r) = tok.strip_prefix('-') {
            old = r
                .split(',')
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(1);
        } else if let Some(r) = tok.strip_prefix('+') {
            new = r
                .split(',')
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(1);
            break;
        }
    }
    (old, new)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub name: String,
    pub current: bool,
}

#[derive(Debug, Clone)]
pub struct Repo {
    pub root: PathBuf,
    program: String,
}

impl Repo {
    pub fn discover(start: &Path) -> Result<Repo, GitError> {
        Self::discover_with("git", start)
    }

    pub fn at_unchecked(root: PathBuf) -> Repo {
        Repo {
            root,
            program: "git".to_owned(),
        }
    }

    fn discover_with(program: &str, start: &Path) -> Result<Repo, GitError> {
        let out = run(
            program,
            start,
            &["rev-parse", "--show-toplevel"],
            None,
            STATUS_TIMEOUT,
            64 * 1024,
            true,
        )?;
        if out.code != Some(0) {
            let e = out.stderr.to_lowercase();
            return Err(
                if e.contains("not a git repository") || e.contains("not a git repo") {
                    GitError::NotARepo
                } else {
                    GitError::Failed {
                        command: "rev-parse".into(),
                        code: out.code,
                        stderr: out.stderr,
                    }
                },
            );
        }
        let root = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        Ok(Repo {
            root: PathBuf::from(root),
            program: program.to_owned(),
        })
    }

    fn exec(
        &self,
        args: &[&str],
        stdin: Option<&[u8]>,
        timeout: Duration,
        cap: usize,
        ro: bool,
    ) -> Result<Output, GitError> {
        run(&self.program, &self.root, args, stdin, timeout, cap, ro)
    }

    fn ok(&self, args: &[&str], timeout: Duration) -> Result<Output, GitError> {
        let out = self.exec(args, None, timeout, 1024 * 1024, false)?;
        if out.code == Some(0) {
            Ok(out)
        } else {
            Err(GitError::Failed {
                command: args.first().copied().unwrap_or("").to_owned(),
                code: out.code,
                stderr: out.stderr,
            })
        }
    }

    pub fn status(&self) -> Result<Status, GitError> {
        let out = self.exec(
            &[
                "status",
                "--porcelain=v2",
                "-z",
                "--branch",
                "--untracked-files=normal",
            ],
            None,
            STATUS_TIMEOUT,
            8 * 1024 * 1024,
            true,
        )?;
        if out.code != Some(0) {
            return Err(GitError::Failed {
                command: "status".into(),
                code: out.code,
                stderr: out.stderr,
            });
        }
        let mut st = parse_status(&out.stdout);
        st.truncated |= out.truncated;
        Ok(st)
    }

    pub fn diff(&self, paths: &[&str], staged: bool) -> Result<Diff, GitError> {
        let mut args = vec![
            "diff",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "-U3",
        ];
        if staged {
            args.push("--cached");
        }
        args.push("--");
        args.extend_from_slice(paths);
        let out = self.exec(&args, None, DIFF_TIMEOUT, MAX_DIFF_BYTES, true)?;
        if out.code != Some(0) && out.code != Some(1) {
            return Err(GitError::Failed {
                command: "diff".into(),
                code: out.code,
                stderr: out.stderr,
            });
        }
        let mut d = parse_diff(&String::from_utf8_lossy(&out.stdout));
        d.truncated |= out.truncated;
        Ok(d)
    }

    pub fn diff_untracked(&self, rel: &str) -> Result<Diff, GitError> {
        let path = self.root.join(rel);
        let meta = std::fs::metadata(&path).map_err(|e| GitError::Io(e.to_string()))?;
        if meta.is_dir() {
            return Ok(Diff {
                notes: vec!["untracked directory".to_owned()],
                ..Diff::default()
            });
        }
        let mut bytes = Vec::new();
        std::fs::File::open(&path)
            .and_then(|f| f.take(MAX_DIFF_BYTES as u64).read_to_end(&mut bytes))
            .map_err(|e| GitError::Io(e.to_string()))?;
        let mut d = Diff {
            notes: vec!["untracked file".to_owned()],
            truncated: meta.len() > MAX_DIFF_BYTES as u64,
            ..Diff::default()
        };
        if bytes.iter().take(8000).any(|&b| b == 0) {
            d.binary = true;
            return Ok(d);
        }
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<DiffLine> = text
            .lines()
            .take(MAX_DIFF_LINES)
            .enumerate()
            .map(|(i, l)| DiffLine {
                kind: LineKind::Add,
                old: None,
                new: Some(i as u32 + 1),
                text: l.to_owned(),
            })
            .collect();
        d.truncated |= text.lines().count() > MAX_DIFF_LINES;
        if !lines.is_empty() {
            d.hunks.push(Hunk {
                header: format!("@@ -0,0 +1,{} @@", lines.len()),
                lines,
            });
        }
        Ok(d)
    }

    pub fn has_head(&self) -> bool {
        self.exec(
            &["rev-parse", "--verify", "-q", "HEAD"],
            None,
            STATUS_TIMEOUT,
            1024,
            true,
        )
        .is_ok_and(|o| o.code == Some(0))
    }

    pub fn stage(&self, paths: &[String]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut args = vec!["add", "--"];
        args.extend(paths.iter().map(String::as_str));
        self.ok(&args, WRITE_TIMEOUT).map(|_| ())
    }

    pub fn stage_all(&self) -> Result<(), GitError> {
        self.ok(&["add", "-A"], WRITE_TIMEOUT).map(|_| ())
    }

    pub fn unstage(&self, paths: &[String]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut args = if self.has_head() {
            vec!["restore", "--staged", "--"]
        } else {
            vec!["rm", "--cached", "-r", "-q", "--"]
        };
        args.extend(paths.iter().map(String::as_str));
        self.ok(&args, WRITE_TIMEOUT).map(|_| ())
    }

    pub fn unstage_all(&self) -> Result<(), GitError> {
        if self.has_head() {
            self.ok(&["restore", "--staged", "--", "."], WRITE_TIMEOUT)
                .map(|_| ())
        } else {
            self.ok(&["rm", "--cached", "-r", "-q", "--", "."], WRITE_TIMEOUT)
                .map(|_| ())
        }
    }

    pub fn commit(&self, message: &str) -> Result<String, GitError> {
        if message.trim().is_empty() {
            return Err(GitError::Rejected("enter a commit message first".into()));
        }
        let status = self.status()?;
        if status.staged().next().is_none() {
            return Err(GitError::Rejected(
                "nothing is staged; stage some changes first".into(),
            ));
        }
        let mut msg = message.trim_end().to_owned();
        msg.push('\n');
        let out = self.exec(
            &["commit", "-F", "-"],
            Some(msg.as_bytes()),
            COMMIT_TIMEOUT,
            1024 * 1024,
            false,
        )?;
        if out.code == Some(0) {
            let stdout = String::from_utf8_lossy(&out.stdout);
            Ok(stdout.lines().next().unwrap_or("committed").to_owned())
        } else {
            let mut text = out.stderr;
            let so = String::from_utf8_lossy(&out.stdout);
            if !so.trim().is_empty() {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(so.trim());
            }
            Err(GitError::Failed {
                command: "commit".into(),
                code: out.code,
                stderr: text,
            })
        }
    }

    pub fn branches(&self) -> Result<Vec<Branch>, GitError> {
        let out = self.exec(
            &[
                "for-each-ref",
                "--format=%(HEAD)%(refname:short)",
                "refs/heads",
            ],
            None,
            STATUS_TIMEOUT,
            1024 * 1024,
            true,
        )?;
        if out.code != Some(0) {
            return Err(GitError::Failed {
                command: "for-each-ref".into(),
                code: out.code,
                stderr: out.stderr,
            });
        }
        let mut list: Vec<Branch> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.len() > 1)
            .map(|l| Branch {
                current: l.starts_with('*'),
                name: l[1..].to_owned(),
            })
            .collect();
        list.sort_by_key(|b| b.name.to_lowercase());
        Ok(list)
    }

    fn check_branch_name(&self, name: &str) -> Result<(), GitError> {
        if name.is_empty() || name.starts_with('-') || name.chars().any(char::is_whitespace) {
            return Err(GitError::Rejected(format!(
                "{name:?} is not a valid branch name"
            )));
        }
        let out = self.exec(
            &["check-ref-format", "--branch", name],
            None,
            STATUS_TIMEOUT,
            1024,
            true,
        )?;
        if out.code == Some(0) {
            Ok(())
        } else {
            Err(GitError::Rejected(format!(
                "{name:?} is not a valid branch name"
            )))
        }
    }

    pub fn checkout(&self, name: &str) -> Result<(), GitError> {
        self.check_branch_name(name)?;
        self.ok(&["switch", name], WRITE_TIMEOUT).map(|_| ())
    }

    pub fn create_branch(&self, name: &str) -> Result<(), GitError> {
        self.check_branch_name(name)?;
        self.ok(&["switch", "-c", name], WRITE_TIMEOUT).map(|_| ())
    }

    fn upstream(&self) -> Option<String> {
        let out = self
            .exec(
                &[
                    "rev-parse",
                    "--abbrev-ref",
                    "--symbolic-full-name",
                    "@{upstream}",
                ],
                None,
                STATUS_TIMEOUT,
                4096,
                true,
            )
            .ok()?;
        (out.code == Some(0)).then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
    }

    fn remotes(&self) -> Result<Vec<String>, GitError> {
        let out = self.ok(&["remote"], STATUS_TIMEOUT)?;
        Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| l.trim().to_owned())
            .filter(|l| !l.is_empty())
            .collect())
    }

    fn network_summary(out: &Output, fallback: &str) -> String {
        let text = format!(
            "{}\n{}",
            out.stderr.trim(),
            String::from_utf8_lossy(&out.stdout).trim()
        );
        text.lines()
            .map(str::trim)
            .rev()
            .find(|l| !l.is_empty() && !l.starts_with("remote:"))
            .unwrap_or(fallback)
            .to_owned()
    }

    /// Pushes the current branch; a branch without an upstream is published to the only remote, or to origin.
    pub fn push(&self) -> Result<String, GitError> {
        let args: Vec<String> = if self.upstream().is_some() {
            vec!["push".into()]
        } else {
            let remotes = self.remotes()?;
            let remote = match remotes.as_slice() {
                [] => {
                    return Err(GitError::Rejected(
                        "this repository has no remote to push to".into(),
                    ));
                }
                [only] => only.clone(),
                many if many.iter().any(|r| r == "origin") => "origin".to_owned(),
                _ => {
                    return Err(GitError::Rejected(
                        "several remotes and none is called origin".into(),
                    ));
                }
            };
            vec![
                "push".into(),
                "--set-upstream".into(),
                remote,
                "HEAD".into(),
            ]
        };
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = self.exec(&refs, None, NETWORK_TIMEOUT, 1024 * 1024, false)?;
        if out.code == Some(0) {
            Ok(Self::network_summary(&out, "Pushed"))
        } else {
            Err(GitError::Failed {
                command: "push".into(),
                code: out.code,
                stderr: out.stderr,
            })
        }
    }

    /// Fast-forward only, so a pull never creates a merge commit or rewrites history behind the user's back.
    pub fn pull(&self) -> Result<String, GitError> {
        if self.upstream().is_none() {
            return Err(GitError::Rejected(
                "this branch has no upstream to pull from".into(),
            ));
        }
        let out = self.exec(
            &["pull", "--ff-only"],
            None,
            NETWORK_TIMEOUT,
            1024 * 1024,
            false,
        )?;
        if out.code == Some(0) {
            Ok(Self::network_summary(&out, "Up to date"))
        } else {
            Err(GitError::Failed {
                command: "pull".into(),
                code: out.code,
                stderr: out.stderr,
            })
        }
    }

    pub fn fetch(&self) -> Result<String, GitError> {
        let out = self.exec(
            &["fetch", "--prune"],
            None,
            NETWORK_TIMEOUT,
            1024 * 1024,
            false,
        )?;
        if out.code == Some(0) {
            Ok("Fetched".to_owned())
        } else {
            Err(GitError::Failed {
                command: "fetch".into(),
                code: out.code,
                stderr: out.stderr,
            })
        }
    }

    pub fn log(&self, skip: usize, limit: usize) -> Result<Vec<Commit>, GitError> {
        if !self.has_head() {
            return Ok(Vec::new());
        }
        let skip = skip.to_string();
        let max = limit.to_string();
        let out = self.exec(
            &[
                "log",
                "--decorate=short",
                "--skip",
                &skip,
                "--max-count",
                &max,
                "--date-order",
                "--format=%H%x1f%an%x1f%at%x1f%D%x1f%P%x1f%s%x1e",
            ],
            None,
            STATUS_TIMEOUT,
            4 * 1024 * 1024,
            true,
        )?;
        if out.code != Some(0) {
            return Err(GitError::Failed {
                command: "log".into(),
                code: out.code,
                stderr: out.stderr,
            });
        }
        Ok(parse_log(&String::from_utf8_lossy(&out.stdout)))
    }

    pub fn commit_detail(&self, hash: &str) -> Result<CommitDetail, GitError> {
        check_hash(hash)?;
        let out = self.exec(
            &[
                "show",
                "--no-patch",
                "--shortstat",
                "--first-parent",
                "-m",
                "--format=%B%x1f%ai%x1e",
                hash,
            ],
            None,
            DIFF_TIMEOUT,
            1024 * 1024,
            true,
        )?;
        if out.code != Some(0) {
            return Err(GitError::Failed {
                command: "show".into(),
                code: out.code,
                stderr: out.stderr,
            });
        }
        Ok(parse_commit_detail(&String::from_utf8_lossy(&out.stdout)))
    }

    /// The web page of the remote this branch tracks (or origin), when it is a known host.
    pub fn web_remote(&self) -> Option<WebRemote> {
        let name = self
            .upstream()
            .and_then(|u| u.split_once('/').map(|(r, _)| r.to_owned()))
            .unwrap_or_else(|| "origin".to_owned());
        let out = self
            .exec(
                &["remote", "get-url", &name],
                None,
                STATUS_TIMEOUT,
                4096,
                true,
            )
            .ok()?;
        if out.code != Some(0) {
            return None;
        }
        parse_web_remote(&String::from_utf8_lossy(&out.stdout))
    }

    pub fn commit_files(&self, hash: &str) -> Result<Vec<CommitFile>, GitError> {
        check_hash(hash)?;
        let out = self.exec(
            &[
                "show",
                "--format=",
                "--name-status",
                "-M",
                "--first-parent",
                "-m",
                hash,
            ],
            None,
            DIFF_TIMEOUT,
            1024 * 1024,
            true,
        )?;
        if out.code != Some(0) {
            return Err(GitError::Failed {
                command: "show".into(),
                code: out.code,
                stderr: out.stderr,
            });
        }
        Ok(parse_name_status(&String::from_utf8_lossy(&out.stdout)))
    }

    pub fn commit_diff(&self, hash: &str, path: &str, old: Option<&str>) -> Result<Diff, GitError> {
        check_hash(hash)?;
        let mut args = vec![
            "show",
            "--format=",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "-U3",
            "-M",
            "--first-parent",
            "-m",
            hash,
            "--",
            path,
        ];
        if let Some(o) = old {
            args.push(o);
        }
        let out = self.exec(&args, None, DIFF_TIMEOUT, MAX_DIFF_BYTES, true)?;
        if out.code != Some(0) {
            return Err(GitError::Failed {
                command: "show".into(),
                code: out.code,
                stderr: out.stderr,
            });
        }
        let mut d = parse_diff(&String::from_utf8_lossy(&out.stdout));
        d.truncated |= out.truncated;
        Ok(d)
    }

    pub fn discard(&self, paths: &[String]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut args = vec!["restore", "--"];
        args.extend(paths.iter().map(String::as_str));
        self.ok(&args, WRITE_TIMEOUT).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tmp(PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = Command::new("cmd")
                .args(["/C", "rmdir", "/S", "/Q"])
                .arg(&self.0)
                .output();
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn sh(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .expect("git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn new_repo(tag: &str) -> (Tmp, Repo) {
        let dir = std::env::temp_dir().join(format!("nanti-git-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        sh(&dir, &["init", "-q", "-b", "main"]);
        for (k, v) in [
            ("user.name", "Nanti Test"),
            ("user.email", "test@example.com"),
            ("commit.gpgsign", "false"),
            ("core.autocrlf", "false"),
            ("core.safecrlf", "false"),
        ] {
            sh(&dir, &["config", k, v]);
        }
        let repo = Repo::discover(&dir).unwrap();
        (Tmp(dir), repo)
    }

    fn write(repo: &Repo, rel: &str, text: &str) {
        let p = repo.root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    fn commit_all(repo: &Repo, msg: &str) {
        sh(&repo.root, &["add", "-A"]);
        sh(&repo.root, &["commit", "-q", "-m", msg]);
    }

    #[test]
    fn parse_status_handles_every_record_kind() {
        let raw = b"# branch.oid 0123456789abcdef\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0\
1 M. N... 100644 100644 100644 aaaa bbbb staged.txt\0\
1 .M N... 100644 100644 100644 aaaa bbbb work tree.txt\0\
1 MM N... 100644 100644 100644 aaaa bbbb both.txt\0\
1 D. N... 100644 000000 000000 aaaa 0000 gone.txt\0\
2 R. N... 100644 100644 100644 aaaa bbbb R100 new name.txt\0old name.txt\0\
u UU N... 100644 100644 100644 100644 aaaa bbbb cccc conflict.txt\0\
? untracked.txt\0\
? dir/\0\
! ignored.log\0";
        let st = parse_status(raw);
        assert_eq!((st.branch.as_deref(), st.detached), (Some("main"), false));
        assert_eq!(st.upstream.as_deref(), Some("origin/main"));
        assert_eq!((st.ahead, st.behind), (2, 1));
        assert_eq!(st.oid.as_deref(), Some("0123456789abcdef"));
        let by = |p: &str| {
            st.entries
                .iter()
                .find(|e| e.path == p)
                .unwrap_or_else(|| panic!("no {p}"))
        };
        assert_eq!(
            (by("staged.txt").staged, by("staged.txt").unstaged),
            (Some(Change::Modified), None)
        );
        assert_eq!(
            (by("work tree.txt").staged, by("work tree.txt").unstaged),
            (None, Some(Change::Modified))
        );
        assert_eq!(
            (by("both.txt").staged, by("both.txt").unstaged),
            (Some(Change::Modified), Some(Change::Modified))
        );
        assert_eq!(by("gone.txt").staged, Some(Change::Deleted));
        assert_eq!(by("new name.txt").staged, Some(Change::Renamed));
        assert_eq!(
            by("new name.txt").orig_path.as_deref(),
            Some("old name.txt")
        );
        assert!(by("conflict.txt").conflicted);
        assert_eq!(by("untracked.txt").unstaged, Some(Change::Untracked));
        assert_eq!(by("dir/").unstaged, Some(Change::Untracked));
        assert!(
            st.entries.iter().all(|e| e.path != "ignored.log"),
            "ignored files are not listed"
        );
        assert_eq!(st.staged().count(), 4);
        assert_eq!(st.changes().count(), 4);
        assert_eq!(st.conflicts().count(), 1);
        assert!(
            st.covers("dir/inner/file.rs").is_some(),
            "an untracked directory covers its files"
        );
        assert!(st.covers("other/file.rs").is_none());
        assert!(st.dir_has_changes("dir") && !st.dir_has_changes("nope"));
    }

    #[test]
    fn parse_status_detached_initial_and_empty() {
        let st = parse_status(b"# branch.oid (initial)\0# branch.head main\0");
        assert_eq!((st.oid, st.branch.as_deref()), (None, Some("main")));
        let d = parse_status(b"# branch.oid abc\0# branch.head (detached)\0");
        assert!(d.detached && d.branch.is_none());
        assert!(parse_status(b"").is_clean());
        assert!(
            parse_status(b"garbage\0\0").is_clean(),
            "unknown records are ignored, not fatal"
        );
    }

    #[test]
    fn parse_status_caps_entries() {
        let mut raw = Vec::new();
        for i in 0..MAX_STATUS_ENTRIES + 10 {
            raw.extend(format!("? f{i}.txt\0").into_bytes());
        }
        let st = parse_status(&raw);
        assert_eq!(st.entries.len(), MAX_STATUS_ENTRIES);
        assert!(st.truncated);
    }

    #[test]
    fn parse_log_reads_records_refs_and_parent_counts() {
        let text = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\u{1f}Ann\u{1f}1700000000\u{1f}HEAD -> main, origin/main\u{1f}bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\u{1f}fix: a | b\u{1e}\ncccccccccccccccccccccccccccccccccccccccc\u{1f}Bob\u{1f}1690000000\u{1f}\u{1f}\u{1f}root\u{1e}\n";
        let log = parse_log(text);
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].short(), "aaaaaaa");
        assert_eq!(log[0].refs, ["HEAD -> main", "origin/main"]);
        assert_eq!((log[0].parents, log[0].subject.as_str()), (1, "fix: a | b"));
        assert_eq!(
            (log[1].parents, log[1].refs.len(), log[1].time),
            (0, 0, 1_690_000_000)
        );
    }

    #[test]
    fn parse_name_status_handles_renames_and_plain_changes() {
        let files = parse_name_status(
            "M\tsrc/a.rs\nA\tnew.txt\nR100\told.txt\tnew name.txt\nD\tgone.txt\n",
        );
        assert_eq!(files.len(), 4);
        assert_eq!((files[0].letter, files[0].path.as_str()), ('M', "src/a.rs"));
        assert_eq!(
            (
                files[2].letter,
                files[2].path.as_str(),
                files[2].old.as_deref()
            ),
            ('R', "new name.txt", Some("old.txt"))
        );
    }

    #[test]
    fn web_remotes_cover_the_usual_address_shapes_and_nothing_else() {
        let gh = |s: &str| parse_web_remote(s).map(|w| (w.host_label, w.base));
        let want = Some(("GitHub", "https://github.com/theepar/nanti-ide".to_owned()));
        for ok in [
            "https://github.com/theepar/nanti-ide.git",
            "https://github.com/theepar/nanti-ide",
            "git@github.com:theepar/nanti-ide.git",
            "ssh://git@github.com/theepar/nanti-ide.git",
            "https://user:token@github.com/theepar/nanti-ide.git\n",
        ] {
            assert_eq!(gh(ok), want, "{ok}");
        }
        assert_eq!(gh("https://gitlab.com/g/s/p.git").unwrap().0, "GitLab");
        for bad in [
            "",
            "https://example.com/a/b.git",
            "https://github.com/onlyone",
            "https://github.com/a/../b",
            "https://github.com/a/b c",
            "file:///C:/repo",
            "C:\\repo",
        ] {
            assert_eq!(gh(bad), None, "{bad}");
        }
        let w = parse_web_remote("git@github.com:o/r.git").unwrap();
        assert_eq!(
            w.commit_url("abcdef1234").as_deref(),
            Some("https://github.com/o/r/commit/abcdef1234")
        );
        assert_eq!(w.commit_url("--x"), None);
        assert!(
            parse_web_remote("https://gitlab.com/g/p.git")
                .unwrap()
                .commit_url("abcdef1")
                .unwrap()
                .contains("/-/commit/")
        );
    }

    #[test]
    fn commit_detail_reads_body_offset_and_stats() {
        let text = "feat: x\n\nlong body line\n\u{1f}2026-11-03 03:00:45 +0700\u{1e}\n\n 3 files changed, 12 insertions(+), 7 deletions(-)\n";
        let d = parse_commit_detail(text);
        assert_eq!(d.body, "feat: x\n\nlong body line");
        assert_eq!(
            (d.offset_min, d.files, d.insertions, d.deletions),
            (420, 3, 12, 7)
        );
        let only_add = parse_commit_detail(
            "m\u{1f}2026-01-01 00:00:00 -0330\u{1e}\n\n 1 file changed, 58 insertions(+)\n",
        );
        assert_eq!(
            (
                only_add.offset_min,
                only_add.files,
                only_add.insertions,
                only_add.deletions
            ),
            (-210, 1, 58, 0)
        );
        let empty = parse_commit_detail("m\u{1f}2026-01-01 00:00:00 +0000\u{1e}\n");
        assert_eq!((empty.files, empty.insertions, empty.deletions), (0, 0, 0));
    }

    #[test]
    fn only_hex_ids_reach_git() {
        assert!(check_hash("abcdef1").is_ok());
        for bad in ["", "abc", "--output=x", "HEAD", "abcdefg"] {
            assert!(check_hash(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn parse_diff_tracks_line_numbers_and_markers() {
        let text = "diff --git a/x b/x\nindex 1..2 100644\n--- a/x\n+++ b/x\n@@ -3,4 +3,5 @@ fn ctx()\n keep\n-old\n+new1\n+new2\n keep2\n\\ No newline at end of file\n";
        let d = parse_diff(text);
        assert_eq!(d.hunks.len(), 1);
        let l = &d.hunks[0].lines;
        assert_eq!(
            (l[0].kind, l[0].old, l[0].new),
            (LineKind::Context, Some(3), Some(3))
        );
        assert_eq!(
            (l[1].kind, l[1].old, l[1].new, l[1].text.as_str()),
            (LineKind::Del, Some(4), None, "old")
        );
        assert_eq!(
            (l[2].kind, l[2].old, l[2].new),
            (LineKind::Add, None, Some(4))
        );
        assert_eq!((l[3].old, l[3].new), (None, Some(5)));
        assert_eq!((l[4].old, l[4].new), (Some(5), Some(6)));
        assert_eq!(l[5].kind, LineKind::NoNewline);
        assert_eq!((d.added(), d.removed()), (2, 1));
    }

    #[test]
    fn parse_diff_headers_binary_crlf_and_limits() {
        let d = parse_diff(
            "diff --git a/x b/x\nnew file mode 100644\nBinary files /dev/null and b/x differ\n",
        );
        assert!(d.binary);
        assert_eq!(d.notes, ["new file"]);
        let r = parse_diff(
            "diff --git a/a b/b\nsimilarity index 90%\nrename from a\nrename to b\n@@ -1 +1 @@\n-a\r\n+b\r\n",
        );
        assert_eq!(r.notes, ["renamed from a"]);
        assert_eq!(
            r.hunks[0].lines[0].text, "a",
            "CR of CRLF content is dropped"
        );
        let mut big = String::from("@@ -1,0 +1,0 @@\n");
        for _ in 0..MAX_DIFF_LINES + 5 {
            big.push_str("+x\n");
        }
        let t = parse_diff(&big);
        assert!(t.truncated);
        assert_eq!(t.hunks[0].lines.len(), MAX_DIFF_LINES);
        assert!(parse_diff("").is_empty());
    }

    #[test]
    fn discover_finds_the_root_from_a_subdirectory_and_rejects_non_repos() {
        let (tmp, repo) = new_repo("disc");
        std::fs::create_dir_all(tmp.0.join("a/b")).unwrap();
        let sub = Repo::discover(&tmp.0.join("a/b")).unwrap();
        assert_eq!(
            std::fs::canonicalize(&sub.root).unwrap(),
            std::fs::canonicalize(&repo.root).unwrap()
        );
        let outside =
            std::env::temp_dir().join(format!("nanti-git-nonrepo-{}", std::process::id()));
        std::fs::create_dir_all(&outside).unwrap();
        assert_eq!(Repo::discover(&outside).unwrap_err(), GitError::NotARepo);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn missing_git_binary_is_reported_clearly() {
        let err = Repo::discover_with("definitely-not-a-git-binary-xyz", &std::env::temp_dir())
            .unwrap_err();
        assert_eq!(err, GitError::NotInstalled);
        assert!(err.to_string().contains("install"));
    }

    #[test]
    fn status_on_a_fresh_repo_then_after_changes() {
        let (_t, repo) = new_repo("stat");
        let st = repo.status().unwrap();
        assert_eq!(st.branch.as_deref(), Some("main"));
        assert!(st.oid.is_none() && st.is_clean());
        write(&repo, "a.txt", "one\n");
        write(&repo, "dir with space/ü ni.txt", "x\n");
        let st = repo.status().unwrap();
        let paths: Vec<&str> = st.entries.iter().map(|e| e.path.as_str()).collect();
        assert!(
            paths.contains(&"a.txt") && paths.contains(&"dir with space/"),
            "{paths:?}"
        );
        commit_all(&repo, "first");
        let st = repo.status().unwrap();
        assert!(st.is_clean() && st.oid.is_some());
        write(&repo, "a.txt", "one\ntwo\n");
        write(&repo, "new.txt", "n\n");
        sh(&repo.root, &["add", "new.txt"]);
        write(&repo, "new.txt", "n\nm\n");
        let st = repo.status().unwrap();
        let a = st.entry_for("a.txt").unwrap();
        assert_eq!((a.staged, a.unstaged), (None, Some(Change::Modified)));
        let n = st.entry_for("new.txt").unwrap();
        assert_eq!(
            (n.staged, n.unstaged),
            (Some(Change::Added), Some(Change::Modified))
        );
    }

    #[test]
    fn rename_delete_and_unicode_paths() {
        let (_t, repo) = new_repo("ren");
        write(
            &repo,
            "old name.txt",
            "content that is long enough to be detected as a rename\n",
        );
        write(&repo, "日本語.txt", "jp\n");
        write(&repo, "gone.txt", "bye\n");
        commit_all(&repo, "base");
        sh(&repo.root, &["mv", "old name.txt", "new name.txt"]);
        std::fs::remove_file(repo.root.join("gone.txt")).unwrap();
        write(&repo, "日本語.txt", "jp2\n");
        let st = repo.status().unwrap();
        let r = st.entry_for("new name.txt").unwrap();
        assert_eq!(
            (r.staged, r.orig_path.as_deref()),
            (Some(Change::Renamed), Some("old name.txt"))
        );
        assert_eq!(
            st.entry_for("gone.txt").unwrap().unstaged,
            Some(Change::Deleted)
        );
        assert_eq!(
            st.entry_for("日本語.txt").unwrap().unstaged,
            Some(Change::Modified)
        );
    }

    #[test]
    fn diff_unstaged_staged_new_deleted_binary_and_no_newline() {
        let (_t, repo) = new_repo("diff");
        write(&repo, "a.txt", "l1\nl2\nl3\n");
        write(&repo, "gone.txt", "bye\n");
        commit_all(&repo, "base");
        write(&repo, "a.txt", "l1\nL2\nl3\nl4");
        let d = repo.diff(&["a.txt"], false).unwrap();
        assert_eq!((d.added(), d.removed()), (2, 1));
        assert!(
            d.hunks[0]
                .lines
                .iter()
                .any(|l| l.kind == LineKind::NoNewline)
        );
        let none = repo.diff(&["a.txt"], true).unwrap();
        assert!(none.is_empty(), "nothing staged yet");
        repo.stage(&["a.txt".to_owned()]).unwrap();
        assert_eq!(repo.diff(&["a.txt"], true).unwrap().added(), 2);
        assert!(
            repo.diff(&["a.txt"], false).unwrap().is_empty(),
            "now only in the index"
        );
        std::fs::remove_file(repo.root.join("gone.txt")).unwrap();
        let del = repo.diff(&["gone.txt"], false).unwrap();
        assert_eq!(del.notes, ["deleted file"]);
        assert_eq!(del.removed(), 1);
        std::fs::write(repo.root.join("blob.bin"), [0u8, 1, 2, 3, 0, 255]).unwrap();
        sh(&repo.root, &["add", "blob.bin"]);
        assert!(repo.diff(&["blob.bin"], true).unwrap().binary);
    }

    #[test]
    fn diff_of_untracked_files_is_all_additions() {
        let (_t, repo) = new_repo("untr");
        write(&repo, "n.txt", "héllo\r\nwörld\n");
        let d = repo.diff_untracked("n.txt").unwrap();
        assert_eq!(d.notes, ["untracked file"]);
        assert_eq!(d.added(), 2);
        assert_eq!(d.hunks[0].lines[0].text, "héllo");
        std::fs::write(repo.root.join("b.bin"), [0u8, 0, 1]).unwrap();
        assert!(repo.diff_untracked("b.bin").unwrap().binary);
        std::fs::create_dir_all(repo.root.join("sub")).unwrap();
        assert_eq!(
            repo.diff_untracked("sub").unwrap().notes,
            ["untracked directory"]
        );
        assert!(repo.diff_untracked("missing.txt").is_err());
    }

    #[test]
    fn crlf_files_diff_cleanly() {
        let (_t, repo) = new_repo("crlf");
        write(&repo, "w.txt", "a\r\nb\r\n");
        commit_all(&repo, "base");
        write(&repo, "w.txt", "a\r\nB\r\n");
        let d = repo.diff(&["w.txt"], false).unwrap();
        assert_eq!((d.added(), d.removed()), (1, 1));
        assert!(d.hunks[0].lines.iter().all(|l| !l.text.contains('\r')));
    }

    #[test]
    fn stage_and_unstage_including_before_the_first_commit_and_awkward_names() {
        let (_t, repo) = new_repo("stage");
        let names = ["a b.txt", "[br].txt", "日本.txt", "#h.txt"];
        for n in names {
            write(&repo, n, "x\n");
        }
        write(&repo, "b.txt", "x\n");
        write(&repo, "r.txt", "x\n");
        repo.stage(&names.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .unwrap();
        let st = repo.status().unwrap();
        assert_eq!(
            st.staged().count(),
            4,
            "literal paths, no glob expansion: {:?}",
            st.entries
        );
        assert_eq!(st.entry_for("b.txt").unwrap().staged, None);
        repo.unstage(&["[br].txt".to_owned()]).unwrap();
        let st = repo.status().unwrap();
        assert_eq!(st.staged().count(), 3);
        assert_eq!(
            st.entry_for("[br].txt").unwrap().unstaged,
            Some(Change::Untracked)
        );
        repo.unstage_all().unwrap();
        assert_eq!(repo.status().unwrap().staged().count(), 0);
        repo.stage_all().unwrap();
        assert_eq!(repo.status().unwrap().staged().count(), 6);
        commit_all(&repo, "base");
        write(&repo, "a b.txt", "y\n");
        repo.stage(&["a b.txt".to_owned()]).unwrap();
        repo.unstage(&["a b.txt".to_owned()]).unwrap();
        let e = repo.status().unwrap();
        assert_eq!(e.entry_for("a b.txt").unwrap().staged, None);
        repo.stage(&[]).unwrap();
        repo.unstage(&[]).unwrap();
    }

    #[test]
    fn commit_creates_a_commit_with_the_exact_message() {
        let (_t, repo) = new_repo("commit");
        write(&repo, "a.txt", "1\n");
        repo.stage(&["a.txt".to_owned()]).unwrap();
        let summary = repo
            .commit("Add a.txt\n\nBody with ünïcode and 日本語.")
            .unwrap();
        assert!(summary.contains("Add a.txt"), "{summary}");
        let log = sh(&repo.root, &["log", "-1", "--format=%B"]);
        assert_eq!(log.trim_end(), "Add a.txt\n\nBody with ünïcode and 日本語.");
        assert!(repo.status().unwrap().is_clean());
        assert_eq!(sh(&repo.root, &["rev-list", "--count", "HEAD"]).trim(), "1");
    }

    #[test]
    fn commit_refuses_empty_message_and_empty_index_without_touching_history() {
        let (_t, repo) = new_repo("refuse");
        write(&repo, "a.txt", "1\n");
        repo.stage(&["a.txt".to_owned()]).unwrap();
        assert!(matches!(repo.commit("   \n"), Err(GitError::Rejected(_))));
        commit_all(&repo, "base");
        write(&repo, "a.txt", "2\n");
        assert!(matches!(repo.commit("msg"), Err(GitError::Rejected(m)) if m.contains("staged")));
        assert_eq!(sh(&repo.root, &["rev-list", "--count", "HEAD"]).trim(), "1");
    }

    #[test]
    fn a_failing_hook_blocks_the_commit_and_its_output_is_returned() {
        let (_t, repo) = new_repo("hook");
        write(&repo, "a.txt", "1\n");
        let hook = repo.root.join(".git/hooks/pre-commit");
        std::fs::write(
            &hook,
            "#!/bin/sh\necho 'lint failed: fix your code' >&2\nexit 1\n",
        )
        .unwrap();
        repo.stage(&["a.txt".to_owned()]).unwrap();
        let err = repo.commit("will be blocked").unwrap_err();
        match &err {
            GitError::Failed { stderr, .. } => assert!(stderr.contains("lint failed"), "{stderr}"),
            other => panic!("{other:?}"),
        }
        assert!(err.to_string().contains("lint failed"));
        assert!(!repo.has_head(), "no commit was made");
        assert_eq!(
            repo.status().unwrap().staged().count(),
            1,
            "the work is still staged"
        );
    }

    #[test]
    fn commit_never_pushes_and_status_reports_ahead_behind() {
        let (tmp, repo) = new_repo("remote");
        write(&repo, "a.txt", "1\n");
        commit_all(&repo, "base");
        let remote = tmp.0.join("remote.git");
        sh(
            &tmp.0,
            &[
                "init",
                "-q",
                "--bare",
                "-b",
                "main",
                remote.to_str().unwrap(),
            ],
        );
        sh(
            &repo.root,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        sh(&repo.root, &["push", "-q", "-u", "origin", "main"]);
        let st = repo.status().unwrap();
        assert_eq!(
            (st.upstream.as_deref(), st.ahead, st.behind),
            (Some("origin/main"), 0, 0)
        );
        write(&repo, "b.txt", "2\n");
        repo.stage(&["b.txt".to_owned()]).unwrap();
        repo.commit("local only").unwrap();
        let st = repo.status().unwrap();
        assert_eq!((st.ahead, st.behind), (1, 0));
        assert_eq!(
            sh(&remote, &["rev-list", "--count", "main"]).trim(),
            "1",
            "the remote did not receive the commit"
        );
    }

    #[test]
    fn merge_conflicts_show_up_as_conflicted_entries() {
        let (_t, repo) = new_repo("conf");
        write(&repo, "c.txt", "base\n");
        commit_all(&repo, "base");
        sh(&repo.root, &["checkout", "-q", "-b", "other"]);
        write(&repo, "c.txt", "other\n");
        commit_all(&repo, "other");
        sh(&repo.root, &["checkout", "-q", "main"]);
        write(&repo, "c.txt", "main\n");
        commit_all(&repo, "main");
        let merge = Command::new("git")
            .current_dir(&repo.root)
            .args(["merge", "other"])
            .output()
            .unwrap();
        assert!(!merge.status.success());
        let st = repo.status().unwrap();
        assert_eq!(st.conflicts().count(), 1);
        assert!(st.entry_for("c.txt").unwrap().conflicted);
        assert!(st.staged().next().is_none());
    }

    #[test]
    fn a_hung_git_is_killed_at_the_timeout() {
        let (_t, repo) = new_repo("timeout");
        let start = Instant::now();
        let err = run(
            "git",
            &repo.root,
            &["-c", "alias.slow=!sleep 20", "slow"],
            None,
            Duration::from_millis(400),
            1024,
            true,
        )
        .err()
        .expect("timed out");
        assert!(matches!(err, GitError::Timeout(_)), "{err:?}");
        assert!(
            start.elapsed() < Duration::from_secs(6),
            "killed promptly: {:?}",
            start.elapsed()
        );
    }

    #[test]
    fn output_beyond_the_cap_is_truncated_not_buffered() {
        let (_t, repo) = new_repo("cap");
        write(&repo, "big.txt", &"line of text\n".repeat(200_000));
        commit_all(&repo, "base");
        write(&repo, "big.txt", &"changed line of text\n".repeat(200_000));
        let d = repo.diff(&["big.txt"], false).unwrap();
        assert!(d.truncated);
        assert!(d.hunks.iter().map(|h| h.lines.len()).sum::<usize>() <= MAX_DIFF_LINES);
    }

    #[test]
    fn opening_a_repo_with_a_configured_fsmonitor_does_not_run_it() {
        let (_t, repo) = new_repo("fsmon");
        write(&repo, "a.txt", "1\n");
        commit_all(&repo, "base");
        let marker = repo.root.join("PWNED.txt");
        let script = repo.root.join(".git").join("evil.sh");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\necho pwned > '{}'\nexit 0\n",
                marker.display().to_string().replace('\\', "/")
            ),
        )
        .unwrap();
        sh(
            &repo.root,
            &[
                "config",
                "core.fsmonitor",
                &script.display().to_string().replace('\\', "/"),
            ],
        );
        let _ = Command::new("git")
            .current_dir(&repo.root)
            .args(["status", "--porcelain"])
            .output();
        let control_ran = marker.exists();
        let _ = std::fs::remove_file(&marker);
        repo.status().unwrap();
        repo.diff(&["a.txt"], false).unwrap();
        assert!(
            !marker.exists(),
            "fsmonitor must not run when the IDE looks at a repo (control ran: {control_ran})"
        );
    }

    #[test]
    fn status_does_not_take_the_index_lock() {
        let (_t, repo) = new_repo("lock");
        write(&repo, "a.txt", "1\n");
        commit_all(&repo, "base");
        std::fs::write(repo.root.join(".git/index.lock"), "").unwrap();
        assert!(repo.status().is_ok());
        let _ = std::fs::remove_file(repo.root.join(".git/index.lock"));
    }

    #[test]
    fn branches_list_current_first_marker_and_sorted_names() {
        let (_t, repo) = new_repo("branches");
        assert!(
            repo.branches().unwrap().is_empty(),
            "no commit yet, no branch"
        );
        write(&repo, "a.txt", "1\n");
        commit_all(&repo, "base");
        sh(&repo.root, &["branch", "Zeta"]);
        sh(&repo.root, &["branch", "alpha"]);
        let list = repo.branches().unwrap();
        let names: Vec<&str> = list.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, ["alpha", "main", "Zeta"]);
        assert_eq!(list.iter().filter(|b| b.current).count(), 1);
        assert!(list.iter().find(|b| b.name == "main").unwrap().current);
    }

    #[test]
    fn create_and_switch_branches_and_see_the_change_in_status() {
        let (_t, repo) = new_repo("switch");
        write(&repo, "a.txt", "1\n");
        commit_all(&repo, "base");
        repo.create_branch("feature/x").unwrap();
        assert_eq!(repo.status().unwrap().branch.as_deref(), Some("feature/x"));
        write(&repo, "b.txt", "2\n");
        commit_all(&repo, "on feature");
        repo.checkout("main").unwrap();
        assert!(!repo.root.join("b.txt").exists());
        assert_eq!(repo.status().unwrap().branch.as_deref(), Some("main"));
        assert!(repo.create_branch("feature/x").is_err(), "already exists");
        repo.checkout("feature/x").unwrap();
        assert!(repo.root.join("b.txt").exists());
    }

    #[test]
    fn bad_branch_names_are_refused_before_git_changes_anything() {
        let (_t, repo) = new_repo("badnames");
        write(&repo, "a.txt", "1\n");
        commit_all(&repo, "base");
        for bad in [
            "", "-d", "--force", "a b", "a..b", "x~1", "bad:name", "end.lock", "/lead", "trail/",
            "q?",
        ] {
            assert!(
                matches!(repo.create_branch(bad), Err(GitError::Rejected(_))),
                "{bad:?}"
            );
            assert!(
                matches!(repo.checkout(bad), Err(GitError::Rejected(_))),
                "{bad:?}"
            );
        }
        assert_eq!(repo.branches().unwrap().len(), 1);
    }

    #[test]
    fn switching_with_conflicting_local_changes_is_refused_and_keeps_the_work() {
        let (_t, repo) = new_repo("dirtyswitch");
        write(&repo, "a.txt", "base\n");
        commit_all(&repo, "base");
        repo.create_branch("other").unwrap();
        write(&repo, "a.txt", "other\n");
        commit_all(&repo, "other edit");
        repo.checkout("main").unwrap();
        write(&repo, "a.txt", "my unsaved work\n");
        let err = repo.checkout("other").unwrap_err();
        assert!(matches!(err, GitError::Failed { .. }), "{err:?}");
        assert_eq!(
            std::fs::read_to_string(repo.root.join("a.txt")).unwrap(),
            "my unsaved work\n"
        );
        assert_eq!(repo.status().unwrap().branch.as_deref(), Some("main"));
    }

    #[test]
    fn discard_restores_the_index_version_and_keeps_staged_work() {
        let (_t, repo) = new_repo("discard");
        write(&repo, "a.txt", "one\n");
        write(&repo, "b.txt", "bee\n");
        write(&repo, "[x].txt", "lit\n");
        write(&repo, "x.txt", "glob decoy\n");
        commit_all(&repo, "base");
        write(&repo, "a.txt", "staged edit\n");
        sh(&repo.root, &["add", "a.txt"]);
        write(&repo, "a.txt", "staged edit\nplus unstaged\n");
        write(&repo, "b.txt", "changed\n");
        write(&repo, "[x].txt", "changed\n");
        write(&repo, "x.txt", "changed\n");
        std::fs::remove_file(repo.root.join("b.txt")).unwrap();
        repo.discard(&["a.txt".to_owned(), "b.txt".to_owned(), "[x].txt".to_owned()])
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(repo.root.join("a.txt")).unwrap(),
            "staged edit\n"
        );
        assert_eq!(
            std::fs::read_to_string(repo.root.join("b.txt")).unwrap(),
            "bee\n",
            "a deleted file comes back"
        );
        assert_eq!(
            std::fs::read_to_string(repo.root.join("[x].txt")).unwrap(),
            "lit\n"
        );
        assert_eq!(
            std::fs::read_to_string(repo.root.join("x.txt")).unwrap(),
            "changed\n",
            "the glob decoy is untouched"
        );
        let st = repo.status().unwrap();
        assert_eq!(
            st.entry_for("a.txt").unwrap().staged,
            Some(Change::Modified)
        );
        assert_eq!(st.entry_for("a.txt").unwrap().unstaged, None);
        repo.discard(&[]).unwrap();
    }

    #[test]
    fn discard_refuses_untracked_files_and_leaves_them() {
        let (_t, repo) = new_repo("discarduntracked");
        write(&repo, "a.txt", "1\n");
        commit_all(&repo, "base");
        write(&repo, "new.txt", "keep\n");
        assert!(repo.discard(&["new.txt".to_owned()]).is_err());
        assert_eq!(
            std::fs::read_to_string(repo.root.join("new.txt")).unwrap(),
            "keep\n"
        );
    }
}
