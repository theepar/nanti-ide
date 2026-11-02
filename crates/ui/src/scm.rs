use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

use git::{Change, Diff, GitError, Repo, Status};

use crate::palette::TextField;

pub const REFRESH_INTERVAL: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiffKey {
    pub rel: String,
    pub staged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffReq {
    pub key: DiffKey,
    pub orig: Option<String>,
    pub untracked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    Refresh,
    Stage(Vec<String>),
    StageAll,
    Unstage(Vec<String>),
    UnstageAll,
    Commit(String),
    CommitAll(String),
    Checkout(String),
    NewBranch(String),
    Discard(Vec<String>),
    Diff(DiffReq),
}

#[derive(Debug, Clone)]
pub enum DiffState {
    Loading,
    Ready(Diff),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct DiffView {
    pub req: DiffReq,
    pub state: DiffState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GroupKind {
    Conflicts,
    Staged,
    Changes,
}

impl GroupKind {
    pub fn title(self) -> &'static str {
        match self {
            GroupKind::Conflicts => "MERGE CONFLICTS",
            GroupKind::Staged => "STAGED CHANGES",
            GroupKind::Changes => "CHANGES",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Modified,
    Added,
    Removed,
    Conflict,
}

impl Tone {
    fn of(c: Change) -> Tone {
        match c {
            Change::Modified | Change::TypeChanged => Tone::Modified,
            Change::Added | Change::Untracked | Change::Renamed | Change::Copied => Tone::Added,
            Change::Deleted => Tone::Removed,
            Change::Conflicted => Tone::Conflict,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub rel: String,
    pub orig: Option<String>,
    pub letter: char,
    pub tone: Tone,
    pub untracked: bool,
    pub group: GroupKind,
}

impl Row {
    pub fn name_and_dir(&self) -> (&str, &str) {
        let trimmed = self.rel.trim_end_matches('/');
        match trimmed.rfind('/') {
            Some(i) => (&trimmed[i + 1..], &trimmed[..i]),
            None => (trimmed, ""),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decor {
    pub letter: char,
    pub tone: Tone,
}

#[derive(Default)]
struct Done {
    repo: Option<Repo>,
    status: Option<Result<Status, GitError>>,
    notice: Option<(bool, String)>,
    diff: Option<(DiffKey, Result<Diff, String>)>,
    committed: bool,
    branches: Option<Vec<git::Branch>>,
    worktree_changed: bool,
}

#[derive(Default)]
pub struct Polled {
    pub changed: bool,
    pub worktree_changed: bool,
    pub status_changed: bool,
    pub diffs: Vec<(DiffKey, Result<Diff, String>)>,
    pub notices: Vec<String>,
}

impl Polled {
    fn merge(&mut self, other: Polled) {
        self.changed |= other.changed;
        self.status_changed |= other.status_changed;
        self.diffs.extend(other.diffs);
        self.notices.extend(other.notices);
    }
}

pub struct ScmState {
    root: PathBuf,
    enabled: bool,
    repo: Option<Repo>,
    pub status: Option<Status>,
    pub branches: Vec<git::Branch>,
    pub error: Option<String>,
    pub message: TextField,
    pub focused: bool,
    pub notice: Option<(bool, String)>,
    pub collapsed: HashSet<GroupKind>,
    queue: VecDeque<Job>,
    running: Option<Receiver<Done>>,
    last_refresh: Instant,
    files: HashMap<String, Decor>,
    dirs: HashSet<String>,
    untracked_dirs: Vec<String>,
}

impl ScmState {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            enabled: false,
            repo: None,
            status: None,
            branches: Vec::new(),
            error: None,
            message: TextField::default(),
            focused: false,
            notice: None,
            collapsed: HashSet::new(),
            queue: VecDeque::new(),
            running: None,
            last_refresh: Instant::now(),
            files: HashMap::new(),
            dirs: HashSet::new(),
            untracked_dirs: Vec::new(),
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
        self.request(Job::Refresh);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_busy(&self) -> bool {
        self.running.is_some() || !self.queue.is_empty()
    }

    pub fn repo_root(&self) -> Option<&Path> {
        self.repo.as_ref().map(|r| r.root.as_path())
    }

    pub fn request(&mut self, job: Job) {
        if !self.enabled {
            return;
        }
        let duplicate = match &job {
            Job::Refresh => self.queue.contains(&Job::Refresh),
            Job::Diff(r) => self
                .queue
                .iter()
                .any(|j| matches!(j, Job::Diff(q) if q.key == r.key)),
            _ => false,
        };
        if !duplicate {
            if !matches!(job, Job::Refresh | Job::Diff(_)) {
                self.notice = None;
            }
            self.queue.push_back(job);
        }
        self.start_next();
    }

    fn start_next(&mut self) {
        if self.running.is_some() {
            return;
        }
        let Some(job) = self.queue.pop_front() else {
            return;
        };
        let (tx, rx) = channel();
        let root = self.root.clone();
        let repo = self.repo.clone();
        std::thread::spawn(move || {
            let _ = tx.send(execute(&root, repo, job));
        });
        self.running = Some(rx);
    }

    pub fn poll(&mut self) -> Polled {
        let mut out = Polled::default();
        if let Some(rx) = &self.running {
            match rx.try_recv() {
                Ok(done) => {
                    self.running = None;
                    out = self.apply(done);
                    self.start_next();
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.running = None;
                    self.notice = Some((true, "git worker stopped unexpectedly".into()));
                    out.changed = true;
                    self.start_next();
                }
            }
        }
        if self.enabled
            && self.running.is_none()
            && self.queue.is_empty()
            && self.last_refresh.elapsed() >= REFRESH_INTERVAL
        {
            self.last_refresh = Instant::now();
            self.request(Job::Refresh);
        }
        out
    }

    pub fn wait_idle(&mut self) -> Polled {
        let mut all = Polled::default();
        let start = Instant::now();
        while self.is_busy() {
            all.merge(self.poll());
            assert!(
                start.elapsed() < Duration::from_secs(60),
                "git jobs never finished"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        all
    }

    fn apply(&mut self, done: Done) -> Polled {
        let mut p = Polled {
            changed: true,
            ..Polled::default()
        };
        if let Some(r) = done.repo {
            self.repo = Some(r);
        }
        if let Some(b) = done.branches {
            self.branches = b;
        }
        p.worktree_changed = done.worktree_changed;
        if let Some(st) = done.status {
            self.last_refresh = Instant::now();
            match st {
                Ok(s) => {
                    self.error = None;
                    if self.status.as_ref() != Some(&s) {
                        p.status_changed = true;
                        self.status = Some(s);
                        self.rebuild_decor();
                    }
                }
                Err(e) => {
                    self.error = Some(e.to_string());
                    if self.status.take().is_some() {
                        p.status_changed = true;
                    }
                    if matches!(e, GitError::NotARepo) {
                        self.repo = None;
                    }
                    self.rebuild_decor();
                }
            }
        }
        if let Some((is_err, text)) = done.notice {
            p.notices.push(if is_err {
                format!("Git: {text}")
            } else {
                text.clone()
            });
            self.notice = Some((is_err, text));
        }
        if let Some(d) = done.diff {
            p.diffs.push(d);
        }
        if done.committed {
            self.message.clear();
        }
        p
    }

    pub fn refresh(&mut self) {
        self.request(Job::Refresh);
    }

    pub fn stage_all(&mut self) {
        self.request(Job::StageAll);
    }

    pub fn unstage_all(&mut self) {
        self.request(Job::UnstageAll);
    }

    pub fn checkout(&mut self, name: &str) {
        self.request(Job::Checkout(name.to_owned()));
    }

    pub fn new_branch(&mut self, name: &str) {
        self.request(Job::NewBranch(name.to_owned()));
    }

    pub fn discard(&mut self, rel: &str) {
        self.request(Job::Discard(vec![rel.to_owned()]));
    }

    pub fn stage(&mut self, rel: &str) {
        self.request(Job::Stage(vec![rel.to_owned()]));
    }

    pub fn unstage(&mut self, rel: &str) {
        let mut paths = vec![rel.to_owned()];
        if let Some(orig) = self
            .status
            .as_ref()
            .and_then(|s| s.entry_for(rel))
            .and_then(|e| e.orig_path.clone())
        {
            paths.push(orig);
        }
        self.request(Job::Unstage(paths));
    }

    pub fn commit(&mut self) -> Result<(), String> {
        let msg = self.message.text().trim().to_owned();
        let refusal = if self.status.is_none() {
            Some("no Git repository".to_owned())
        } else if msg.is_empty() {
            Some("enter a commit message first".to_owned())
        } else if self.staged_count() == 0 && self.committable_changes() == 0 {
            Some("nothing to commit".to_owned())
        } else {
            None
        };
        if let Some(r) = refusal {
            self.notice = Some((true, r.clone()));
            return Err(r);
        }
        if self.staged_count() == 0 {
            self.request(Job::CommitAll(msg));
        } else {
            self.request(Job::Commit(msg));
        }
        Ok(())
    }

    pub fn committable_changes(&self) -> usize {
        self.status.as_ref().map_or(0, |s| s.changes().count())
    }

    pub fn toggle_group(&mut self, g: GroupKind) {
        if !self.collapsed.remove(&g) {
            self.collapsed.insert(g);
        }
    }

    pub fn staged_count(&self) -> usize {
        self.status.as_ref().map_or(0, |s| s.staged().count())
    }

    pub fn change_count(&self) -> usize {
        self.status.as_ref().map_or(0, |s| s.entries.len())
    }

    pub fn group(&self, kind: GroupKind) -> Vec<Row> {
        let Some(st) = &self.status else {
            return Vec::new();
        };
        st.entries
            .iter()
            .filter_map(|e| {
                let (change, in_group) = match kind {
                    GroupKind::Conflicts => (Some(Change::Conflicted), e.conflicted),
                    GroupKind::Staged => (e.staged, !e.conflicted && e.staged.is_some()),
                    GroupKind::Changes => (e.unstaged, !e.conflicted && e.unstaged.is_some()),
                };
                if !in_group {
                    return None;
                }
                let change = change?;
                Some(Row {
                    rel: e.path.clone(),
                    orig: if kind == GroupKind::Staged {
                        e.orig_path.clone()
                    } else {
                        None
                    },
                    letter: change.letter(),
                    tone: Tone::of(change),
                    untracked: change == Change::Untracked,
                    group: kind,
                })
            })
            .collect()
    }

    pub fn branch_label(&self) -> Option<String> {
        let st = self.status.as_ref()?;
        let mut s = match (&st.branch, &st.oid) {
            (Some(b), _) => b.clone(),
            (None, Some(oid)) => format!("detached {}", oid.chars().take(7).collect::<String>()),
            (None, None) => "no commits".to_owned(),
        };
        if !st.is_clean() {
            s.push('*');
        }
        if st.ahead > 0 {
            s.push_str(&format!(" ↑{}", st.ahead));
        }
        if st.behind > 0 {
            s.push_str(&format!(" ↓{}", st.behind));
        }
        Some(s)
    }

    pub fn diff_req(&self, rel: &str, staged: bool) -> DiffReq {
        let entry = self.status.as_ref().and_then(|s| s.entry_for(rel));
        DiffReq {
            key: DiffKey {
                rel: rel.to_owned(),
                staged,
            },
            orig: if staged {
                entry.and_then(|e| e.orig_path.clone())
            } else {
                None
            },
            untracked: !staged && entry.is_some_and(|e| e.unstaged == Some(Change::Untracked)),
        }
    }

    fn rebuild_decor(&mut self) {
        self.files.clear();
        self.dirs.clear();
        self.untracked_dirs.clear();
        let Some(st) = &self.status else {
            return;
        };
        for e in &st.entries {
            let (letter, tone) = if e.conflicted {
                ('!', Tone::Conflict)
            } else {
                let c = e.unstaged.or(e.staged).unwrap_or(Change::Modified);
                (c.letter(), Tone::of(c))
            };
            let path = e.path.trim_end_matches('/');
            if e.path.ends_with('/') {
                self.untracked_dirs.push(e.path.clone());
            }
            self.files.insert(path.to_owned(), Decor { letter, tone });
            let mut dir = path;
            while let Some(i) = dir.rfind('/') {
                dir = &dir[..i];
                if !self.dirs.insert(dir.to_owned()) {
                    break;
                }
            }
        }
    }

    pub fn rel_of(&self, abs: &Path) -> Option<String> {
        let root = self.repo_root()?;
        let norm = |p: &Path| {
            let s = p.to_string_lossy().replace('\\', "/");
            s.strip_prefix("//?/").map_or(s.clone(), str::to_owned)
        };
        let (r, a) = (norm(root), norm(abs));
        let r = r.trim_end_matches('/');
        let head = a.get(..r.len())?;
        if !head.eq_ignore_ascii_case(r) {
            return None;
        }
        let rest = a[r.len()..].strip_prefix('/')?;
        Some(rest.to_owned())
    }

    pub fn decor_for(&self, abs: &Path, is_dir: bool) -> Option<Decor> {
        let rel = self.rel_of(abs)?;
        if is_dir {
            if self
                .untracked_dirs
                .iter()
                .any(|d| d.trim_end_matches('/') == rel || rel.starts_with(&d[..]))
            {
                return Some(Decor {
                    letter: '•',
                    tone: Tone::Added,
                });
            }
            return self.dirs.contains(&rel).then_some(Decor {
                letter: '•',
                tone: Tone::Modified,
            });
        }
        if let Some(d) = self.files.get(&rel) {
            return Some(*d);
        }
        self.untracked_dirs
            .iter()
            .any(|d| rel.starts_with(d.as_str()))
            .then_some(Decor {
                letter: 'U',
                tone: Tone::Added,
            })
    }
}

fn execute(root: &Path, repo: Option<Repo>, job: Job) -> Done {
    let mut done = Done::default();
    let repo = match repo {
        Some(r) => r,
        None => match Repo::discover(root) {
            Ok(r) => r,
            Err(e) => {
                done.status = Some(Err(e));
                return done;
            }
        },
    };
    done.repo = Some(repo.clone());
    let report = |done: &mut Done, r: Result<(), GitError>| {
        if let Err(e) = r {
            done.notice = Some((true, e.to_string()));
        }
    };
    match job {
        Job::Refresh => {}
        Job::Stage(p) => report(&mut done, repo.stage(&p)),
        Job::StageAll => report(&mut done, repo.stage_all()),
        Job::Unstage(p) => report(&mut done, repo.unstage(&p)),
        Job::UnstageAll => report(&mut done, repo.unstage_all()),
        Job::Checkout(name) => match repo.checkout(&name) {
            Ok(()) => {
                done.worktree_changed = true;
                done.notice = Some((false, format!("Switched to {name}")));
            }
            Err(e) => done.notice = Some((true, e.to_string())),
        },
        Job::NewBranch(name) => match repo.create_branch(&name) {
            Ok(()) => {
                done.worktree_changed = true;
                done.notice = Some((false, format!("Created and switched to {name}")));
            }
            Err(e) => done.notice = Some((true, e.to_string())),
        },
        Job::Discard(paths) => match repo.discard(&paths) {
            Ok(()) => {
                done.worktree_changed = true;
                done.notice = Some((false, format!("Discarded changes to {}", paths.join(", "))));
            }
            Err(e) => done.notice = Some((true, e.to_string())),
        },
        Job::CommitAll(msg) => match repo.stage_all().and_then(|()| repo.commit(&msg)) {
            Ok(summary) => {
                done.committed = true;
                done.notice = Some((false, format!("Committed: {summary}")));
            }
            Err(e) => done.notice = Some((true, e.to_string())),
        },
        Job::Commit(msg) => match repo.commit(&msg) {
            Ok(summary) => {
                done.committed = true;
                done.notice = Some((false, format!("Committed: {summary}")));
            }
            Err(e) => done.notice = Some((true, e.to_string())),
        },
        Job::Diff(req) => {
            let result = if req.untracked {
                repo.diff_untracked(&req.key.rel)
            } else {
                let mut paths = Vec::new();
                if let Some(o) = &req.orig {
                    paths.push(o.as_str());
                }
                paths.push(req.key.rel.as_str());
                repo.diff(&paths, req.key.staged)
            };
            done.diff = Some((req.key, result.map_err(|e| e.to_string())));
            return done;
        }
    }
    done.branches = repo.branches().ok();
    done.status = Some(repo.status());
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, staged: Option<Change>, unstaged: Option<Change>) -> git::Entry {
        git::Entry {
            path: path.into(),
            orig_path: None,
            staged,
            unstaged,
            conflicted: false,
        }
    }

    #[test]
    fn disabled_state_never_spawns_anything() {
        let mut s = ScmState::new(std::env::temp_dir());
        s.request(Job::Refresh);
        s.stage("a.txt");
        assert!(!s.is_busy());
        assert_eq!(s.commit(), Err("no Git repository".to_owned()));
    }

    #[test]
    fn row_name_and_dir_split() {
        let r = |rel: &str| Row {
            rel: rel.into(),
            orig: None,
            letter: 'M',
            tone: Tone::Modified,
            untracked: false,
            group: GroupKind::Changes,
        };
        assert_eq!(r("src/ui/lib.rs").name_and_dir(), ("lib.rs", "src/ui"));
        assert_eq!(r("top.txt").name_and_dir(), ("top.txt", ""));
        assert_eq!(r("dir/").name_and_dir(), ("dir", ""));
    }

    fn bare(entries: Vec<git::Entry>) -> ScmState {
        let mut s = ScmState::new(PathBuf::from("D:/ws"));
        s.status = Some(Status {
            branch: Some("main".into()),
            entries,
            ..Status::default()
        });
        s.rebuild_decor();
        s
    }

    #[test]
    fn groups_split_staged_changes_and_conflicts() {
        let mut conflict = entry("c.txt", None, None);
        conflict.conflicted = true;
        let mut renamed = entry("new.txt", Some(Change::Renamed), None);
        renamed.orig_path = Some("old.txt".into());
        let s = bare(vec![
            entry("both.txt", Some(Change::Modified), Some(Change::Modified)),
            conflict,
            renamed,
            entry("u.txt", None, Some(Change::Untracked)),
        ]);
        let staged = s.group(GroupKind::Staged);
        assert_eq!(
            staged.iter().map(|r| r.rel.as_str()).collect::<Vec<_>>(),
            ["both.txt", "new.txt"]
        );
        assert_eq!(staged[1].orig.as_deref(), Some("old.txt"));
        let changes = s.group(GroupKind::Changes);
        assert_eq!(
            changes
                .iter()
                .map(|r| (r.rel.as_str(), r.letter))
                .collect::<Vec<_>>(),
            [("both.txt", 'M'), ("u.txt", 'U')]
        );
        assert!(changes[1].untracked);
        let conflicts = s.group(GroupKind::Conflicts);
        assert_eq!(
            (conflicts.len(), conflicts[0].letter, conflicts[0].tone),
            (1, '!', Tone::Conflict)
        );
        assert_eq!((s.staged_count(), s.change_count()), (2, 4));
    }

    #[test]
    fn branch_label_shows_dirty_and_sync_state() {
        let mut s = bare(vec![]);
        assert_eq!(s.branch_label().as_deref(), Some("main"));
        s.status = Some(Status {
            branch: Some("dev".into()),
            ahead: 2,
            behind: 1,
            entries: vec![entry("a", None, Some(Change::Modified))],
            ..Status::default()
        });
        assert_eq!(s.branch_label().as_deref(), Some("dev* ↑2 ↓1"));
        s.status = Some(Status {
            detached: true,
            oid: Some("0123456789".into()),
            ..Status::default()
        });
        assert_eq!(s.branch_label().as_deref(), Some("detached 0123456"));
        s.status = Some(Status::default());
        assert_eq!(s.branch_label().as_deref(), Some("no commits"));
        s.status = None;
        assert_eq!(s.branch_label(), None);
    }

    #[test]
    fn diff_req_follows_the_status() {
        let mut renamed = entry("new.txt", Some(Change::Renamed), None);
        renamed.orig_path = Some("old.txt".into());
        let s = bare(vec![
            renamed,
            entry("u.txt", None, Some(Change::Untracked)),
            entry("m.txt", None, Some(Change::Modified)),
        ]);
        let r = s.diff_req("new.txt", true);
        assert_eq!(
            (r.orig.as_deref(), r.untracked, r.key.staged),
            (Some("old.txt"), false, true)
        );
        assert!(s.diff_req("u.txt", false).untracked);
        assert!(!s.diff_req("m.txt", false).untracked);
        assert!(!s.diff_req("gone.txt", false).untracked);
    }

    #[test]
    fn commit_refuses_without_message_or_staged_files() {
        let mut s = bare(vec![entry("a.txt", None, Some(Change::Modified))]);
        s.enabled = true;
        assert_eq!(s.commit(), Err("enter a commit message first".to_owned()));
        let mut clean = bare(Vec::new());
        clean.enabled = true;
        clean.message.insert("msg");
        assert!(clean.commit().unwrap_err().contains("nothing to commit"));
        assert!(matches!(clean.notice, Some((true, _))));
        assert!(
            !clean.is_busy(),
            "refused before any git process was started"
        );
    }

    #[test]
    fn decor_marks_files_folders_and_untracked_directories() {
        let mut s = state_with_root(
            "D:/ws",
            vec![
                entry("src/deep/a.rs", None, Some(Change::Modified)),
                entry("new_dir/", None, Some(Change::Untracked)),
                entry("gone.txt", Some(Change::Deleted), None),
            ],
        );
        let d = |p: &str, dir: bool| s.decor_for(Path::new(p), dir).map(|d| (d.letter, d.tone));
        assert_eq!(
            d("D:\\ws\\src\\deep\\a.rs", false),
            Some(('M', Tone::Modified))
        );
        assert_eq!(d("D:\\ws\\src\\deep", true), Some(('•', Tone::Modified)));
        assert_eq!(d("D:\\ws\\src", true), Some(('•', Tone::Modified)));
        assert_eq!(d("D:\\ws\\other", true), None);
        assert_eq!(d("D:\\ws\\new_dir", true), Some(('•', Tone::Added)));
        assert_eq!(d("D:\\ws\\new_dir\\x.rs", false), Some(('U', Tone::Added)));
        assert_eq!(d("D:\\ws\\gone.txt", false), Some(('D', Tone::Removed)));
        assert_eq!(d("D:\\ws\\clean.txt", false), None);
        assert_eq!(
            d("d:\\WS\\src\\deep\\a.rs", false),
            Some(('M', Tone::Modified)),
            "drive and root are case-insensitive"
        );
        assert_eq!(
            d("E:\\elsewhere\\a.rs", false),
            None,
            "outside the repository"
        );
        s.status = None;
        s.rebuild_decor();
        assert_eq!(
            s.decor_for(Path::new("D:\\ws\\src\\deep\\a.rs"), false),
            None
        );
    }

    fn state_with_root(root: &str, entries: Vec<git::Entry>) -> ScmState {
        let mut s = bare(entries);
        s.repo = Some(Repo::at_unchecked(PathBuf::from(root)));
        s
    }
}
