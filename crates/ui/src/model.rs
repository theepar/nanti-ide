use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use editor_core::Editor;

use crate::extensions::{ExtDetail, Extension, MarketplaceExtension};
use crate::find::{FindFocus, FindState};
use crate::layout::{Layout, PanelPos, Side};
use crate::menu::MenuKind;
use crate::output::OutputLog;
use crate::palette::{Command, InputKind, Palette, PaletteAction, PaletteMode, TextField};
use crate::problems::Problems;
use crate::scm::{DiffState, DiffView, Job, ScmState};
use crate::search::SearchState;
use crate::tasks::Task;

const HIDDEN_DIRS: [&str; 3] = [".git", "node_modules", "target"];
pub const INDENT_UNIT: &str = "    ";
pub const MAX_OPEN_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    Explorer,
    Search,
    SourceControl,
    Run,
    Extensions,
}

impl Activity {
    pub const ALL: [Activity; 5] = [
        Activity::Explorer,
        Activity::Search,
        Activity::SourceControl,
        Activity::Run,
        Activity::Extensions,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Activity::Explorer => "EXPLORER",
            Activity::Search => "SEARCH",
            Activity::SourceControl => "SOURCE CONTROL",
            Activity::Run => "RUN",
            Activity::Extensions => "EXTENSIONS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelTab {
    Problems,
    Output,
    Terminal,
}

impl PanelTab {
    pub const ALL: [PanelTab; 3] = [PanelTab::Problems, PanelTab::Output, PanelTab::Terminal];

    pub fn title(self) -> &'static str {
        match self {
            PanelTab::Problems => "PROBLEMS",
            PanelTab::Output => "OUTPUT",
            PanelTab::Terminal => "TERMINAL",
        }
    }
}

pub enum TabContent {
    Text(Editor),
    Unavailable(String),
    Diff(DiffView),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskStamp {
    pub modified: Option<SystemTime>,
    pub len: u64,
}

impl DiskStamp {
    pub fn of(path: &Path) -> Option<Self> {
        let m = std::fs::metadata(path).ok()?;
        Some(Self {
            modified: m.modified().ok(),
            len: m.len(),
        })
    }
}

pub struct Tab {
    pub path: PathBuf,
    pub title: String,
    pub content: TabContent,
    pub stamp: Option<DiskStamp>,
}

impl Tab {
    pub fn is_dirty(&self) -> bool {
        matches!(&self.content, TabContent::Text(e) if e.is_dirty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prompt {
    CloseDirty(usize),
    SaveConflict {
        index: usize,
        reason: String,
        then_close: bool,
    },
    QuitDirty,
    DeleteItem(PathBuf),
    RemoveExtension(PathBuf),
    DiscardChanges {
        rel: String,
        untracked: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenu {
    pub path: PathBuf,
    pub is_dir: bool,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved,
    NothingToSave,
    Conflict(String),
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickRequest {
    File,
    Folder,
    Vsix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocSyncEvent {
    Open {
        uri: String,
        language_id: String,
        version: u64,
        text: String,
    },
    Change {
        uri: String,
        version: u64,
        text: String,
    },
    Save {
        uri: String,
    },
    Close {
        uri: String,
    },
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalRequest {
    Ensure,
    New,
    KillActive,
}

pub struct Model {
    pub root: PathBuf,
    pub(crate) expanded: BTreeSet<PathBuf>,
    pub tabs: Vec<Tab>,
    pub active_tab: Option<usize>,
    pub activity: Activity,
    pub panel_tab: PanelTab,
    pub prompt: Option<Prompt>,
    pub palette: Option<Palette>,
    pub find: Option<FindState>,
    pub layout: Layout,
    pub terminal_focused: bool,
    pub terminal_request: Option<TerminalRequest>,
    pub search: SearchState,
    pub scm: ScmState,
    pub folder_open: bool,
    pub explorer_sel: Option<PathBuf>,
    pub explorer_gen: u64,
    pub menu_open: Option<MenuKind>,
    pub context_menu: Option<ContextMenu>,
    pub output: OutputLog,
    pub problems: Problems,
    pub tasks: Vec<Task>,
    pub extensions: Vec<Extension>,
    pub config_dir: Option<PathBuf>,
    pub panel_extension: Option<PathBuf>,
    pub clipboard_request: Option<String>,
    pub quit_request: bool,
    pub pick_request: Option<PickRequest>,
    pub native_dialogs: bool,
    pub terminal_run: Option<String>,
    pub doc_sync_events: Vec<DocSyncEvent>,
    pub ext_query: TextField,
    pub ext_query_focused: bool,
    pub marketplace_results: Vec<MarketplaceExtension>,
    pub marketplace_searching: bool,
    pub marketplace_message: Option<String>,
    pub marketplace_query: Option<String>,
    pub marketplace_recommended: bool,
    pub(crate) recommend_tried: bool,
    pub marketplace_enabled: bool,
    pub installing: Option<String>,
    pub install_progress: std::sync::Arc<crate::extensions::Progress>,
    pub(crate) progress_seen: u64,
    pub ext_icons: std::collections::HashMap<String, PathBuf>,
    pub(crate) icons_requested: std::collections::HashSet<String>,
    pub(crate) icon_tx: std::sync::mpsc::Sender<(String, PathBuf)>,
    pub(crate) icon_rx: std::sync::mpsc::Receiver<(String, PathBuf)>,
    pub ext_detail: Option<ExtDetail>,
    pub settings_open: bool,
    pub(crate) readme_rx:
        Option<std::sync::mpsc::Receiver<(String, Result<crate::extensions::ExtExtra, String>)>>,
    pub(crate) marketplace_rx:
        Option<std::sync::mpsc::Receiver<Result<Vec<MarketplaceExtension>, String>>>,
    pub(crate) install_rx: Option<std::sync::mpsc::Receiver<Result<Extension, String>>>,
    pub(crate) outline_cache: std::cell::RefCell<Option<(PathBuf, u64, Vec<syntax::Symbol>)>>,
    pub message: Option<String>,
}

impl Model {
    pub fn new(root: PathBuf) -> Self {
        let mut expanded = BTreeSet::new();
        expanded.insert(root.clone());
        let (icon_tx, icon_rx) = std::sync::mpsc::channel();
        Self {
            scm: ScmState::new(root.clone()),
            folder_open: true,
            explorer_sel: None,
            explorer_gen: 0,
            menu_open: None,
            context_menu: None,
            output: OutputLog::new(),
            problems: Problems::default(),
            tasks: Vec::new(),
            extensions: Vec::new(),
            config_dir: None,
            panel_extension: None,
            clipboard_request: None,
            quit_request: false,
            pick_request: None,
            native_dialogs: false,
            terminal_run: None,
            doc_sync_events: Vec::new(),
            ext_query: TextField::default(),
            ext_query_focused: false,
            marketplace_results: Vec::new(),
            marketplace_searching: false,
            marketplace_message: None,
            marketplace_query: None,
            marketplace_recommended: false,
            recommend_tried: false,
            marketplace_enabled: false,
            installing: None,
            install_progress: std::sync::Arc::new(crate::extensions::Progress::default()),
            progress_seen: 0,
            ext_icons: std::collections::HashMap::new(),
            icons_requested: std::collections::HashSet::new(),
            icon_tx,
            icon_rx,
            ext_detail: None,
            settings_open: false,
            readme_rx: None,
            marketplace_rx: None,
            install_rx: None,
            outline_cache: std::cell::RefCell::new(None),
            root,
            expanded,
            tabs: Vec::new(),
            active_tab: None,
            activity: Activity::Explorer,
            panel_tab: PanelTab::Terminal,
            prompt: None,
            palette: None,
            find: None,
            layout: Layout::default(),
            terminal_focused: false,
            terminal_request: None,
            search: SearchState::new(),
            message: None,
        }
    }

    pub fn invalidate_explorer(&mut self) {
        self.explorer_gen = self.explorer_gen.wrapping_add(1);
    }

    pub fn explorer_signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.root.hash(&mut h);
        self.explorer_gen.hash(&mut h);
        for p in &self.expanded {
            p.hash(&mut h);
        }
        h.finish()
    }

    pub fn visible_entries(&self) -> Vec<Entry> {
        let mut out = Vec::new();
        if !self.folder_open {
            return out;
        }
        let root_name = self.root.file_name().map_or_else(
            || self.root.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        out.push(Entry {
            path: self.root.clone(),
            name: root_name,
            depth: 0,
            is_dir: true,
            expanded: self.expanded.contains(&self.root),
        });
        if self.expanded.contains(&self.root) {
            self.collect(&self.root, 1, &mut out);
        }
        out
    }

    fn collect(&self, dir: &Path, depth: usize, out: &mut Vec<Entry>) {
        let Ok(read) = std::fs::read_dir(dir) else {
            return;
        };
        let mut items: Vec<(bool, String, PathBuf)> = read
            .filter_map(Result::ok)
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let is_dir = e.file_type().ok()?.is_dir();
                if is_dir && HIDDEN_DIRS.contains(&name.as_str()) {
                    return None;
                }
                Some((is_dir, name, e.path()))
            })
            .collect();
        items.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
        });
        for (is_dir, name, path) in items {
            let expanded = is_dir && self.expanded.contains(&path);
            out.push(Entry {
                path: path.clone(),
                name,
                depth,
                is_dir,
                expanded,
            });
            if expanded {
                self.collect(&path, depth + 1, out);
            }
        }
    }

    pub fn toggle_dir(&mut self, path: &Path) {
        if !self.expanded.remove(path) {
            self.expanded.insert(path.to_path_buf());
        }
    }

    pub fn without_folder() -> Self {
        let mut m = Self::new(PathBuf::new());
        m.folder_open = false;
        m
    }

    pub fn open_folder(&mut self, folder: PathBuf) {
        if !folder.is_dir() {
            return;
        }
        self.root = folder.clone();
        self.folder_open = true;
        self.expanded.clear();
        self.expanded.insert(folder.clone());
        self.scm = ScmState::new(folder.clone());
        self.scm.enable();
        self.outline_cache = std::cell::RefCell::new(None);
        // Otherwise they keep the old folder's files (and memory) alive and diagnose against the wrong project.
        for t in self.tabs.iter().filter(|t| !t.is_dirty()) {
            let uri = format!("file:///{}", t.path.to_string_lossy().replace('\\', "/"));
            self.doc_sync_events.push(DocSyncEvent::Close { uri });
        }
        self.doc_sync_events.push(DocSyncEvent::Reset);
        self.tabs.retain(|t| t.is_dirty());
        self.active_tab = if self.tabs.is_empty() { None } else { Some(0) };
        self.message = Some(format!("Opened folder {}", self.root.display()));
    }

    pub fn open_file(&mut self, path: &Path) {
        self.ext_detail = None;
        self.settings_open = false;
        if let Some(i) = self
            .tabs
            .iter()
            .position(|t| t.path == path && !matches!(t.content, TabContent::Diff(_)))
        {
            self.active_tab = Some(i);
            return;
        }
        let title = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let content = load(path);
        if let TabContent::Text(e) = &content {
            let uri = format!("file:///{}", path.to_string_lossy().replace('\\', "/"));
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            let lang = match ext {
                "rs" => "rust",
                "go" => "go",
                "js" | "mjs" | "cjs" => "javascript",
                "ts" => "typescript",
                "json" => "json",
                "md" => "markdown",
                "dart" => "dart",
                _ => "plaintext",
            };
            self.doc_sync_events.push(DocSyncEvent::Open {
                uri,
                language_id: lang.to_owned(),
                version: e.version(),
                text: e.text(),
            });
        }
        self.tabs.push(Tab {
            path: path.to_path_buf(),
            title,
            content,
            stamp: DiskStamp::of(path),
        });
        self.active_tab = Some(self.tabs.len() - 1);
    }

    pub fn activate_tab(&mut self, index: usize) {
        self.ext_detail = None;
        self.settings_open = false;
        if index < self.tabs.len() {
            self.active_tab = Some(index);
        }
    }

    pub fn close_tab(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() {
            return false;
        }
        if self.tabs[index].is_dirty() {
            self.prompt = Some(Prompt::CloseDirty(index));
            return false;
        }
        self.remove_tab(index);
        true
    }

    fn remove_tab(&mut self, index: usize) {
        let removed = self.tabs.remove(index);
        let uri = format!(
            "file:///{}",
            removed.path.to_string_lossy().replace('\\', "/")
        );
        self.doc_sync_events.push(DocSyncEvent::Close { uri });
        self.active_tab = match self.active_tab {
            _ if self.tabs.is_empty() => None,
            Some(a) if a > index => Some(a - 1),
            Some(a) if a == index => Some(index.min(self.tabs.len() - 1)),
            other => other,
        };
    }

    pub fn active_editor_mut(&mut self) -> Option<&mut Editor> {
        let i = self.active_tab?;
        match &mut self.tabs.get_mut(i)?.content {
            TabContent::Text(e) => Some(e),
            TabContent::Unavailable(_) | TabContent::Diff(_) => None,
        }
    }

    pub fn enable_source_control(&mut self) {
        if self.folder_open {
            self.scm.enable();
        }
    }

    pub fn open_diff(&mut self, rel: &str, staged: bool) {
        self.ext_detail = None;
        self.settings_open = false;
        let conflicted = self
            .scm
            .status
            .as_ref()
            .and_then(|s| s.entry_for(rel))
            .is_some_and(|e| e.conflicted);
        let abs = self
            .scm
            .repo_root()
            .map_or_else(|| self.root.join(rel), |r| r.join(rel));
        if conflicted {
            return self.open_file(&abs);
        }
        let req = self.scm.diff_req(rel, staged);
        let existing = self
            .tabs
            .iter()
            .position(|t| matches!(&t.content, TabContent::Diff(v) if v.req.key == req.key));
        match existing {
            Some(i) => {
                self.active_tab = Some(i);
                if let TabContent::Diff(v) = &mut self.tabs[i].content {
                    v.req = req.clone();
                }
            }
            None => {
                let name = rel.trim_end_matches('/').rsplit('/').next().unwrap_or(rel);
                let title = format!("{name} ({})", if staged { "Index" } else { "Working Tree" });
                self.tabs.push(Tab {
                    path: abs,
                    title,
                    content: TabContent::Diff(DiffView {
                        req: req.clone(),
                        state: DiffState::Loading,
                    }),
                    stamp: None,
                });
                self.active_tab = Some(self.tabs.len() - 1);
            }
        }
        if !self.scm.is_enabled() {
            if let Some(TabContent::Diff(v)) = self.active_mut_content() {
                v.state = DiffState::Failed("source control is not active".into());
            }
            return;
        }
        self.scm.request(Job::Diff(req));
    }

    fn active_mut_content(&mut self) -> Option<&mut TabContent> {
        let i = self.active_tab?;
        self.tabs.get_mut(i).map(|t| &mut t.content)
    }

    pub fn reload_diff_tabs(&mut self) {
        let keys: Vec<(String, bool)> = self
            .tabs
            .iter()
            .filter_map(|t| match &t.content {
                TabContent::Diff(v) => Some((v.req.key.rel.clone(), v.req.key.staged)),
                _ => None,
            })
            .collect();
        for (rel, staged) in keys {
            let req = self.scm.diff_req(&rel, staged);
            for t in &mut self.tabs {
                if let TabContent::Diff(v) = &mut t.content
                    && v.req.key == req.key
                {
                    v.req = req.clone();
                }
            }
            self.scm.request(Job::Diff(req));
        }
    }

    pub fn pump_scm(&mut self) -> bool {
        let polled = self.scm.poll();
        for (key, result) in polled.diffs {
            for t in &mut self.tabs {
                if let TabContent::Diff(v) = &mut t.content
                    && v.req.key == key
                {
                    v.state = match &result {
                        Ok(d) => DiffState::Ready(d.clone()),
                        Err(e) => DiffState::Failed(e.clone()),
                    };
                }
            }
        }
        for note in &polled.notices {
            self.output.push("Git", note);
        }
        if let Some(note) = polled.notices.into_iter().last() {
            self.message = Some(note);
        }
        if polled.worktree_changed {
            self.reload_clean_tabs();
        }
        if polled.status_changed {
            self.reload_diff_tabs();
        }
        polled.changed
    }

    pub fn branch_label(&self) -> String {
        if let Some(l) = self.scm.branch_label() {
            return format!("⎇ {l}");
        }
        if let Some(e) = &self.scm.error
            && self.scm.is_enabled()
            && !e.contains("not inside a Git repository")
        {
            return format!("git: {e}");
        }
        self.git_branch()
            .map_or_else(|| "no repository".to_owned(), |b| format!("⎇ {b}"))
    }

    fn edit_active(&mut self, f: impl FnOnce(&mut Editor)) {
        if let Some(e) = self.active_editor_mut() {
            f(e);
        }
    }

    pub fn toggle_comment(&mut self) {
        let Some(path) = self.active().map(|t| t.path.clone()) else {
            return;
        };
        match comment_prefix(&path) {
            Some(prefix) => self.edit_active(|e| {
                e.toggle_line_comment(prefix);
            }),
            None => self.say("Editor", "This file type has no line comment syntax"),
        }
    }

    pub fn dirty_count(&self) -> usize {
        self.tabs.iter().filter(|t| t.is_dirty()).count()
    }

    pub fn save_tab(&mut self, index: usize, force: bool) -> SaveOutcome {
        let Some(tab) = self.tabs.get_mut(index) else {
            return SaveOutcome::NothingToSave;
        };
        let TabContent::Text(editor) = &mut tab.content else {
            return SaveOutcome::NothingToSave;
        };
        if !editor.is_dirty() && !force {
            return SaveOutcome::NothingToSave;
        }
        if !force {
            let now = DiskStamp::of(&tab.path);
            if now != tab.stamp {
                let reason = if now.is_none() {
                    "The file was deleted or moved on disk."
                } else {
                    "The file was changed on disk after it was opened."
                };
                return SaveOutcome::Conflict(reason.to_owned());
            }
        }
        match write_atomic(&tab.path, &editor.text()) {
            Ok(()) => {
                tab.stamp = DiskStamp::of(&tab.path);
                editor.mark_saved();
                let uri = format!("file:///{}", tab.path.to_string_lossy().replace('\\', "/"));
                self.doc_sync_events.push(DocSyncEvent::Save { uri });
                let saved = format!("Saved {}", tab.title);
                self.output.push("Files", &saved);
                self.message = Some(saved);
                self.scm.request(Job::Refresh);
                self.reload_diff_tabs();
                SaveOutcome::Saved
            }
            Err(e) => {
                let msg = format!("Save failed for {}: {e}", tab.title);
                self.message = Some(msg.clone());
                SaveOutcome::Failed(msg)
            }
        }
    }

    pub fn save_active(&mut self) -> SaveOutcome {
        match self.active_tab {
            Some(i) => {
                let out = self.save_tab(i, false);
                if let SaveOutcome::Conflict(reason) = &out {
                    self.prompt = Some(Prompt::SaveConflict {
                        index: i,
                        reason: reason.clone(),
                        then_close: false,
                    });
                }
                out
            }
            None => SaveOutcome::NothingToSave,
        }
    }

    pub fn reload_tab(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.content = load(&tab.path);
            tab.stamp = DiskStamp::of(&tab.path);
        }
    }

    pub fn request_quit(&mut self) -> bool {
        if self.dirty_count() == 0 {
            return true;
        }
        self.prompt = Some(Prompt::QuitDirty);
        false
    }

    pub fn resolve_save(&mut self) -> bool {
        match self.prompt.take() {
            Some(Prompt::CloseDirty(i)) => {
                match self.save_tab(i, false) {
                    SaveOutcome::Saved | SaveOutcome::NothingToSave => self.remove_tab(i),
                    SaveOutcome::Conflict(reason) => {
                        self.prompt = Some(Prompt::SaveConflict {
                            index: i,
                            reason,
                            then_close: true,
                        });
                    }
                    SaveOutcome::Failed(_) => {}
                }
                false
            }
            Some(Prompt::SaveConflict {
                index, then_close, ..
            }) => {
                if self.save_tab(index, true) == SaveOutcome::Saved && then_close {
                    self.remove_tab(index);
                }
                false
            }
            Some(Prompt::DeleteItem(path)) => {
                self.delete_item(&path);
                false
            }
            Some(Prompt::DiscardChanges { rel, untracked }) => {
                self.discard_changes(&rel, untracked);
                false
            }
            Some(Prompt::RemoveExtension(dir)) => {
                self.remove_extension(&dir);
                false
            }
            Some(Prompt::QuitDirty) => {
                for i in 0..self.tabs.len() {
                    match self.save_tab(i, false) {
                        SaveOutcome::Saved | SaveOutcome::NothingToSave => {}
                        SaveOutcome::Conflict(reason) => {
                            self.prompt = Some(Prompt::SaveConflict {
                                index: i,
                                reason,
                                then_close: false,
                            });
                            return false;
                        }
                        SaveOutcome::Failed(_) => return false,
                    }
                }
                true
            }
            None => false,
        }
    }

    pub fn resolve_discard(&mut self) -> bool {
        match self.prompt.take() {
            Some(Prompt::CloseDirty(i)) => {
                if i < self.tabs.len() {
                    self.remove_tab(i);
                }
                false
            }
            Some(Prompt::SaveConflict {
                index, then_close, ..
            }) => {
                if then_close {
                    if index < self.tabs.len() {
                        self.remove_tab(index);
                    }
                } else {
                    self.reload_tab(index);
                }
                false
            }
            Some(Prompt::QuitDirty) => true,
            Some(
                Prompt::DeleteItem(_) | Prompt::RemoveExtension(_) | Prompt::DiscardChanges { .. },
            )
            | None => false,
        }
    }

    pub fn open_palette(&mut self, mode: PaletteMode) {
        self.palette = Some(match mode {
            PaletteMode::Commands => Palette::commands(),
            PaletteMode::Files => Palette::files(&self.root),
            PaletteMode::Branches => Palette::branches(&self.scm.branches),
            PaletteMode::Input(kind) => Palette::input(kind, ""),
        });
    }

    pub fn close_palette(&mut self) {
        self.palette = None;
    }

    pub fn accept_palette(&mut self) -> Option<Command> {
        let action = self.palette.as_ref()?.selected_action();
        if let Some(PaletteAction::Submit(kind, text)) = &action {
            match self.submit_input(kind.clone(), text) {
                Ok(()) => self.palette = None,
                Err(e) => {
                    if let Some(p) = self.palette.as_mut() {
                        p.error = Some(e);
                    }
                }
            }
            return None;
        }
        self.palette = None;
        match action? {
            PaletteAction::Submit(..) => None,
            PaletteAction::Command(c) => {
                self.run_command(c);
                Some(c)
            }
            PaletteAction::OpenFile(path) => {
                self.open_file(&path);
                None
            }
            PaletteAction::Checkout(name) => {
                self.scm.checkout(&name);
                None
            }
            PaletteAction::NewBranchPrompt => {
                self.open_input(InputKind::NewBranch, "");
                None
            }
        }
    }

    pub fn run_command(&mut self, command: Command) {
        match command {
            Command::CommandPalette => self.open_palette(PaletteMode::Commands),
            Command::QuickOpen => self.open_palette(PaletteMode::Files),
            Command::Save => {
                let _ = self.save_active();
            }
            Command::CloseTab => {
                if let Some(i) = self.active_tab {
                    self.close_tab(i);
                }
            }
            Command::NextTab | Command::PrevTab => {
                let n = self.tabs.len();
                if let (Some(a), true) = (self.active_tab, n > 1) {
                    let next = if command == Command::NextTab {
                        (a + 1) % n
                    } else {
                        (a + n - 1) % n
                    };
                    self.active_tab = Some(next);
                }
            }
            Command::ToggleSidebar => self.layout.sidebar = !self.layout.sidebar,
            Command::ShowExplorer => self.show_activity(Activity::Explorer),
            Command::ShowSearch => self.show_activity(Activity::Search),
            Command::ShowSourceControl => self.show_activity(Activity::SourceControl),
            Command::ShowRun => self.show_activity(Activity::Run),
            Command::ShowExtensions => self.show_activity(Activity::Extensions),
            Command::GitRefresh => self.scm.refresh(),
            Command::ToggleComment => self.toggle_comment(),
            Command::MoveLineUp => self.edit_active(|e| {
                e.move_lines(false);
            }),
            Command::MoveLineDown => self.edit_active(|e| {
                e.move_lines(true);
            }),
            Command::DuplicateLineUp => self.edit_active(|e| e.duplicate_lines(false)),
            Command::DuplicateLineDown => self.edit_active(|e| e.duplicate_lines(true)),
            Command::DeleteLine => self.edit_active(Editor::delete_lines),
            Command::IndentLines => self.edit_active(|e| e.indent_lines(INDENT_UNIT)),
            Command::OutdentLines => self.edit_active(|e| e.outdent_lines(INDENT_UNIT)),
            Command::NewFile => self.cmd_new(false),
            Command::NewFolder => self.cmd_new(true),
            Command::SaveAll => self.save_all(),
            Command::CloseAllTabs => self.close_all_tabs(),
            Command::RenameItem => self.cmd_rename(),
            Command::DeleteItem => self.cmd_delete(),
            Command::CopyPath => self.cmd_copy_path(),
            Command::RevealInOs => self.cmd_reveal(),
            Command::RefreshExplorer => {
                self.invalidate_explorer();
                self.scm.refresh();
                self.say("Explorer", "Refreshed");
            }
            Command::CollapseAll => self.collapse_all(),
            Command::GoToLine => self.cmd_go_to_line(),
            Command::ShowProblems => self.show_panel_tab(PanelTab::Problems),
            Command::ShowOutput => self.show_panel_tab(PanelTab::Output),
            Command::ClearOutput => self.output.clear(),
            Command::InstallVsix => self.cmd_install_vsix(),
            Command::RunTask => self.show_activity(Activity::Run),
            Command::About => self.open_about(),
            Command::Exit => self.quit_request = true,
            Command::OpenFile => self.cmd_open_file(),
            Command::OpenFolder => self.cmd_open_folder(),
            Command::GitStageAll => self.scm.stage_all(),
            Command::GitUnstageAll => self.scm.unstage_all(),
            Command::SwitchBranch => self.open_branch_picker(),
            Command::GitCommit => {
                self.show_activity(Activity::SourceControl);
                if self.scm.commit().is_err() && self.scm.message.text().trim().is_empty() {
                    self.focus_editor_inputs();
                    self.scm.focused = true;
                }
            }
            Command::Undo => {
                if let Some(e) = self.active_editor_mut() {
                    e.undo();
                }
            }
            Command::Redo => {
                if let Some(e) = self.active_editor_mut() {
                    e.redo();
                }
            }
            Command::SelectAll => {
                if let Some(e) = self.active_editor_mut() {
                    e.select_all();
                }
            }
            Command::NewTerminal => {
                self.show_terminal_panel();
                self.terminal_request = Some(TerminalRequest::New);
            }
            Command::ToggleTerminal => {
                if self.layout.panel
                    && self.panel_tab == PanelTab::Terminal
                    && self.terminal_focused
                {
                    self.layout.panel = false;
                    self.terminal_focused = false;
                } else {
                    self.show_terminal_panel();
                    self.terminal_request = Some(TerminalRequest::Ensure);
                }
            }
            Command::KillTerminal => self.terminal_request = Some(TerminalRequest::KillActive),
            Command::TogglePanel => {
                self.layout.panel = !self.layout.panel;
                if !self.layout.panel {
                    self.layout.panel_maximized = false;
                    self.terminal_focused = false;
                }
            }
            Command::MaximizePanel => {
                if !self.layout.panel {
                    self.layout.panel = true;
                    self.layout.panel_maximized = true;
                } else {
                    self.layout.panel_maximized = !self.layout.panel_maximized;
                }
            }
            Command::ToggleRightPanel => self.layout.right = !self.layout.right,
            Command::OpenSettings => {
                self.settings_open = true;
                self.ext_detail = None;
                self.readme_rx = None;
            }
            Command::SidebarLeft | Command::SidebarRight => {
                self.layout.sidebar_side = if command == Command::SidebarLeft {
                    Side::Left
                } else {
                    Side::Right
                };
                self.layout.sidebar = true;
            }
            Command::PanelBottom | Command::PanelTop | Command::PanelLeft | Command::PanelRight => {
                self.layout.panel_pos = match command {
                    Command::PanelTop => PanelPos::Top,
                    Command::PanelLeft => PanelPos::Left,
                    Command::PanelRight => PanelPos::Right,
                    _ => PanelPos::Bottom,
                };
                self.layout.panel = true;
            }
            Command::ToggleActivityBar => self.layout.activity_bar = !self.layout.activity_bar,
            Command::ToggleStatusBar => self.layout.status_bar = !self.layout.status_bar,
            Command::ResetLayout => self.layout.reset(),
            Command::Find => self.open_find(false),
            Command::Replace => self.open_find(true),
            Command::FindNext => self.find_next(),
            Command::FindPrev => self.find_prev(),
            Command::FindInFiles => {
                self.show_activity(Activity::Search);
                self.search.focused = true;
                if let Some(TabContent::Text(e)) = self.active().map(|t| &t.content)
                    && e.has_selection()
                {
                    let sel = e.selected_text();
                    if !sel.contains(['\n', '\r']) && sel.chars().count() <= 200 {
                        self.search.query.clear();
                        self.search.query.insert(&sel);
                    }
                }
            }
        }
    }

    pub fn open_find(&mut self, show_replace: bool) {
        let selected = match self.active().map(|t| &t.content) {
            Some(TabContent::Text(e)) if e.has_selection() => Some(e.selected_text()),
            _ => None,
        };
        let find = self
            .find
            .get_or_insert_with(|| FindState::new(show_replace));
        find.show_replace |= show_replace;
        find.focus = FindFocus::Query;
        if let Some(sel) = selected
            && !sel.contains(['\n', '\r'])
            && !sel.is_empty()
            && sel.chars().count() <= 200
        {
            find.query.clear();
            find.query.insert(&sel);
        }
        self.refresh_find();
    }

    pub fn close_find(&mut self) {
        self.find = None;
    }

    pub fn find_focus(&self) -> FindFocus {
        self.find.as_ref().map_or(FindFocus::Editor, |f| f.focus)
    }

    pub fn refresh_find(&mut self) {
        let (Some(find), Some(i)) = (self.find.as_mut(), self.active_tab) else {
            return;
        };
        if let Some(Tab {
            content: TabContent::Text(e),
            path,
            ..
        }) = self.tabs.get(i)
        {
            find.refresh(e, Some(path));
        }
    }

    pub fn focus_editor_inputs(&mut self) {
        if let Some(f) = self.find.as_mut() {
            f.focus = FindFocus::Editor;
        }
        self.search.focused = false;
        self.scm.focused = false;
        self.terminal_focused = false;
        self.ext_query_focused = false;
    }

    pub fn find_select_nearest(&mut self) {
        self.refresh_find();
        let (Some(find), Some(i)) = (self.find.as_mut(), self.active_tab) else {
            return;
        };
        if let Some(Tab {
            content: TabContent::Text(e),
            ..
        }) = self.tabs.get_mut(i)
        {
            find.select_nearest(e);
        }
    }

    pub fn find_next(&mut self) {
        self.step_find(true);
    }

    pub fn find_prev(&mut self) {
        self.step_find(false);
    }

    fn step_find(&mut self, forward: bool) {
        if self.find.is_none() {
            return self.open_find(false);
        }
        self.refresh_find();
        let (Some(find), Some(i)) = (self.find.as_mut(), self.active_tab) else {
            return;
        };
        if let Some(Tab {
            content: TabContent::Text(e),
            ..
        }) = self.tabs.get_mut(i)
        {
            if forward {
                find.next(e);
            } else {
                find.prev(e);
            }
        }
    }

    pub fn find_replace_current(&mut self) {
        self.refresh_find();
        let (Some(find), Some(i)) = (self.find.as_mut(), self.active_tab) else {
            return;
        };
        if let Some(Tab {
            content: TabContent::Text(e),
            ..
        }) = self.tabs.get_mut(i)
        {
            find.replace_current(e);
        }
    }

    pub fn find_replace_all(&mut self) {
        self.refresh_find();
        let (Some(find), Some(i)) = (self.find.as_mut(), self.active_tab) else {
            return;
        };
        let mut replaced = 0;
        if let Some(Tab {
            content: TabContent::Text(e),
            ..
        }) = self.tabs.get_mut(i)
        {
            replaced = find.replace_all(e);
        }
        self.message = Some(format!(
            "Replaced {replaced} occurrence{}",
            if replaced == 1 { "" } else { "s" }
        ));
    }

    pub fn run_search(&mut self) {
        if !self.folder_open {
            return self.say("Search", "Open a folder first");
        }
        let root = self.root.clone();
        self.search.run(&root);
    }

    pub fn open_at(&mut self, path: &Path, line: usize, col: usize, len: usize) {
        self.open_file(path);
        if let Some(e) = self.active_editor_mut() {
            let last = e.buffer().len_lines().saturating_sub(1);
            let line = line.min(last);
            let start = e.buffer().line_start(line) + col;
            e.select_range(start..(start + len).min(e.buffer().len_chars()));
        }
    }

    pub(crate) fn show_terminal_panel(&mut self) {
        self.layout.panel = true;
        self.panel_tab = PanelTab::Terminal;
        self.terminal_focused = true;
        if let Some(f) = self.find.as_mut() {
            f.focus = FindFocus::Editor;
        }
        self.search.focused = false;
    }

    pub(crate) fn show_activity(&mut self, activity: Activity) {
        self.activity = activity;
        self.layout.sidebar = true;
        match activity {
            Activity::SourceControl => self.scm.refresh(),
            Activity::Run => self.refresh_tasks(),
            Activity::Extensions => self.refresh_extensions(),
            _ => {}
        }
    }

    pub fn resolve_cancel(&mut self) {
        self.prompt = None;
    }

    pub fn select_activity(&mut self, activity: Activity) {
        if self.activity == activity {
            self.layout.sidebar = !self.layout.sidebar;
        } else {
            self.show_activity(activity);
        }
    }

    pub fn active(&self) -> Option<&Tab> {
        self.active_tab.and_then(|i| self.tabs.get(i))
    }

    pub fn git_branch(&self) -> Option<String> {
        if !self.folder_open {
            return None;
        }
        let head = std::fs::read_to_string(self.root.join(".git").join("HEAD")).ok()?;
        let head = head.trim();
        Some(match head.strip_prefix("ref: refs/heads/") {
            Some(branch) => branch.to_owned(),
            None => format!("detached {}", head.chars().take(7).collect::<String>()),
        })
    }
}

fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map_or_else(|| "file".into(), |n| n.to_string_lossy().into_owned());
    let tmp = path.with_file_name(format!(".{name}.nanti-tmp"));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

fn load(path: &Path) -> TabContent {
    match std::fs::metadata(path) {
        Ok(m) if m.len() > MAX_OPEN_BYTES => {
            return TabContent::Unavailable(format!(
                "File is {} MB; the viewer limit is {} MB.",
                m.len() / (1024 * 1024),
                MAX_OPEN_BYTES / (1024 * 1024)
            ));
        }
        Err(e) => return TabContent::Unavailable(format!("Cannot read file: {e}")),
        Ok(_) => {}
    }
    match std::fs::read(path) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => TabContent::Text(Editor::new(&text)),
            Err(_) => TabContent::Unavailable("Binary or non-UTF-8 file: not shown.".into()),
        },
        Err(e) => TabContent::Unavailable(format!("Cannot read file: {e}")),
    }
}

pub fn comment_prefix(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "rs" | "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" | "mts" | "cts" | "go" | "dart"
        | "java" | "c" | "h" | "cpp" | "hpp" | "cs" | "kt" | "swift" | "scala" | "php" => "//",
        "py" | "rb" | "sh" | "bash" | "ps1" | "toml" | "yaml" | "yml" | "conf" | "gitignore"
        | "r" | "pl" => "#",
        "sql" | "lua" | "hs" => "--",
        _ => return None,
    })
}

pub fn language_of(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("rs") => "Rust",
        Some("js" | "mjs" | "cjs") => "JavaScript",
        Some("ts") => "TypeScript",
        Some("tsx") => "TypeScript React",
        Some("jsx") => "JavaScript React",
        Some("json") => "JSON",
        Some("md") => "Markdown",
        Some("toml") => "TOML",
        Some("yml" | "yaml") => "YAML",
        Some("go") => "Go",
        Some("dart") => "Dart",
        Some("html") => "HTML",
        Some("css") => "CSS",
        _ => "Plain Text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tmp(PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn workspace(tag: &str) -> Tmp {
        let root = std::env::temp_dir().join(format!("nanti-model-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("node_modules/x")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/feature/x\n").unwrap();
        std::fs::write(root.join("b.txt"), "bee\n").unwrap();
        std::fs::write(root.join("A.md"), "# a\n").unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(root.join("blob.bin"), [0xff, 0xfe, 0x00, 0x80]).unwrap();
        Tmp(root)
    }

    fn names(m: &Model) -> Vec<String> {
        m.visible_entries()
            .iter()
            .map(|e| {
                format!(
                    "{}{}{}",
                    "  ".repeat(e.depth),
                    e.name,
                    if e.is_dir { "/" } else { "" }
                )
            })
            .collect()
    }

    #[test]
    fn explorer_lists_dirs_first_sorted_and_hides_heavy_dirs() {
        let ws = workspace("list");
        let m = Model::new(ws.0.clone());
        let n = names(&m);
        assert_eq!(n[1..], ["  src/", "  A.md", "  b.txt", "  blob.bin"]);
    }

    #[test]
    fn recommendations_are_requested_only_by_the_app_and_only_in_the_extensions_view() {
        let ws = workspace("rec");
        let mut m = Model::new(ws.0.clone());
        m.activity = Activity::Extensions;
        assert!(!m.pump_marketplace(), "tests never touch the network");
        assert!(m.marketplace_rx.is_none() && !m.marketplace_searching);

        m.marketplace_enabled = true;
        m.activity = Activity::Explorer;
        assert!(
            !m.pump_marketplace(),
            "nothing to fetch outside the Extensions view"
        );
        assert!(m.marketplace_rx.is_none());

        m.activity = Activity::Extensions;
        m.recommend_tried = true;
        assert!(!m.pump_marketplace());
        assert!(m.marketplace_rx.is_none());
    }

    #[test]
    fn marketplace_answers_arrive_through_the_pump_without_blocking() {
        use std::sync::mpsc::channel;
        let ws = workspace("mkt");
        let mut m = Model::new(ws.0.clone());
        let (tx, rx) = channel();
        m.marketplace_rx = Some(rx);
        m.marketplace_searching = true;
        m.marketplace_query = Some("rust".into());
        assert!(!m.pump_marketplace(), "nothing has arrived yet");
        assert!(m.marketplace_searching);

        tx.send(Ok(vec![MarketplaceExtension {
            id: "a.b".into(),
            name: "b".into(),
            publisher: "a".into(),
            display_name: "B".into(),
            version: "1.0.0".into(),
            description: String::new(),
            download_url: "https://open-vsx.org/x.vsix".into(),
            icon_url: None,
            readme_url: None,
            downloads: 0,
            rating: None,
            rating_count: 0,
            updated: None,
        }]))
        .unwrap();
        assert!(m.pump_marketplace());
        assert!(!m.marketplace_searching);
        assert_eq!(m.marketplace_results.len(), 1);

        let (tx, rx) = channel();
        m.marketplace_rx = Some(rx);
        tx.send(Err("offline".to_owned())).unwrap();
        assert!(m.pump_marketplace());
        assert_eq!(m.marketplace_message.as_deref(), Some("offline"));

        // a worker that died without answering must not leave "Searching..." on screen forever
        let (tx, rx) = channel::<Result<Vec<MarketplaceExtension>, String>>();
        drop(tx);
        m.marketplace_searching = true;
        m.marketplace_rx = Some(rx);
        assert!(m.pump_marketplace());
        assert!(!m.marketplace_searching);
        assert!(
            m.marketplace_message
                .as_deref()
                .is_some_and(|t| t.contains("unexpectedly"))
        );

        m.search_marketplace("   ");
        assert!(m.marketplace_results.is_empty() && m.marketplace_query.is_none());
        assert!(m.marketplace_message.is_none());
    }

    #[test]
    fn the_settings_page_opens_and_gives_way_to_files() {
        let ws = workspace("settings");
        let mut m = Model::new(ws.0.clone());
        m.run_command(Command::OpenSettings);
        assert!(m.settings_open);
        m.open_file(&ws.0.join("b.txt"));
        assert!(!m.settings_open);
        m.run_command(Command::OpenSettings);
        m.close_settings();
        assert!(!m.settings_open);
    }

    #[test]
    fn explorer_signature_changes_with_expansion_root_and_file_operations() {
        let a = workspace("sig-a");
        let b = workspace("sig-b");
        let mut m = Model::new(a.0.clone());
        let base = m.explorer_signature();
        assert_eq!(base, m.explorer_signature(), "stable while nothing changes");
        m.toggle_dir(&a.0.join("src"));
        let expanded = m.explorer_signature();
        assert_ne!(base, expanded);
        m.invalidate_explorer();
        let invalidated = m.explorer_signature();
        assert_ne!(expanded, invalidated);
        m.open_folder(b.0.clone());
        assert_ne!(invalidated, m.explorer_signature());
    }

    #[test]
    fn creating_through_the_model_invalidates_the_listing() {
        let ws = workspace("gen");
        let mut m = Model::new(ws.0.clone());
        let g0 = m.explorer_gen;
        m.submit_input(
            crate::palette::InputKind::NewFile(ws.0.clone()),
            "fresh.txt",
        )
        .unwrap();
        assert!(
            m.explorer_gen > g0,
            "create must invalidate the cached listing"
        );
        assert!(names(&m).contains(&"  fresh.txt".to_string()));
    }

    #[test]
    fn switching_folder_closes_old_documents_and_resets_language_servers() {
        let a = workspace("switch-a");
        let b = workspace("switch-b");
        let mut m = Model::new(a.0.clone());
        m.open_file(&a.0.join("b.txt"));
        m.doc_sync_events.clear();
        m.open_folder(b.0.clone());
        let uri = format!(
            "file:///{}",
            a.0.join("b.txt").to_string_lossy().replace('\\', "/")
        );
        assert_eq!(
            m.doc_sync_events,
            [DocSyncEvent::Close { uri }, DocSyncEvent::Reset]
        );
        assert!(m.tabs.is_empty());
        assert_eq!(m.root, b.0);
    }

    #[test]
    fn toggle_dir_expands_and_collapses() {
        let ws = workspace("toggle");
        let mut m = Model::new(ws.0.clone());
        m.toggle_dir(&ws.0.join("src"));
        assert!(names(&m).contains(&"    main.rs".to_string()));
        m.toggle_dir(&ws.0.join("src"));
        assert!(!names(&m).contains(&"    main.rs".to_string()));
        m.toggle_dir(&ws.0);
        assert_eq!(names(&m).len(), 1);
    }

    #[test]
    fn open_file_dedupes_and_activates() {
        let ws = workspace("open");
        let mut m = Model::new(ws.0.clone());
        m.open_file(&ws.0.join("b.txt"));
        m.open_file(&ws.0.join("A.md"));
        m.open_file(&ws.0.join("b.txt"));
        assert_eq!(m.tabs.len(), 2);
        assert_eq!(m.active().unwrap().title, "b.txt");
        match &m.active().unwrap().content {
            TabContent::Text(b) => assert_eq!(b.text(), "bee\n"),
            TabContent::Unavailable(r) => panic!("{r}"),
            TabContent::Diff(_) => panic!("unexpected diff tab"),
        }
    }

    #[test]
    fn binary_and_missing_files_are_reported_not_loaded() {
        let ws = workspace("bin");
        let mut m = Model::new(ws.0.clone());
        m.open_file(&ws.0.join("blob.bin"));
        assert!(matches!(
            m.active().unwrap().content,
            TabContent::Unavailable(_)
        ));
        m.open_file(&ws.0.join("missing.txt"));
        assert!(matches!(
            m.active().unwrap().content,
            TabContent::Unavailable(_)
        ));
    }

    #[test]
    fn close_tab_keeps_a_sensible_active_tab() {
        let ws = workspace("close");
        let mut m = Model::new(ws.0.clone());
        for f in ["A.md", "b.txt", "blob.bin"] {
            m.open_file(&ws.0.join(f));
        }
        m.activate_tab(1);
        m.close_tab(0);
        assert_eq!(m.active().unwrap().title, "b.txt");
        m.close_tab(0);
        assert_eq!(m.active().unwrap().title, "blob.bin");
        m.close_tab(0);
        assert!(m.active().is_none());
        m.close_tab(5);
    }

    #[test]
    fn activity_click_toggles_sidebar_only_for_active_icon() {
        let ws = workspace("act");
        let mut m = Model::new(ws.0.clone());
        m.select_activity(Activity::Explorer);
        assert!(!m.layout.sidebar);
        m.select_activity(Activity::Search);
        assert!(m.layout.sidebar);
        assert_eq!(m.activity, Activity::Search);
    }

    #[test]
    fn git_branch_is_read_from_head() {
        let ws = workspace("git");
        let m = Model::new(ws.0.clone());
        assert_eq!(m.git_branch().as_deref(), Some("feature/x"));
        assert_eq!(Model::new(ws.0.join("src")).git_branch(), None);
    }

    #[test]
    fn language_detection() {
        assert_eq!(language_of(Path::new("a.rs")), "Rust");
        assert_eq!(language_of(Path::new("README")), "Plain Text");
    }

    fn type_into_active(m: &mut Model, text: &str) {
        let e = m.active_editor_mut().expect("text tab");
        e.move_doc_end(false);
        e.insert(text);
    }

    fn disk(path: &Path) -> Vec<u8> {
        std::fs::read(path).unwrap()
    }

    fn touch_future(path: &Path) {
        let f = std::fs::OpenOptions::new().write(true).open(path).unwrap();
        f.set_modified(SystemTime::now() + std::time::Duration::from_secs(120))
            .unwrap();
    }

    #[test]
    fn edits_survive_tab_switching_and_mark_dirty() {
        let ws = workspace("sw");
        let mut m = Model::new(ws.0.clone());
        m.open_file(&ws.0.join("b.txt"));
        type_into_active(&mut m, "X");
        m.open_file(&ws.0.join("A.md"));
        m.activate_tab(0);
        assert!(m.tabs[0].is_dirty());
        assert!(!m.tabs[1].is_dirty());
        match &m.tabs[0].content {
            TabContent::Text(e) => assert_eq!(e.text(), "bee\nX"),
            TabContent::Unavailable(r) => panic!("{r}"),
            TabContent::Diff(_) => panic!("unexpected diff tab"),
        }
    }

    #[test]
    fn save_writes_atomically_and_clears_dirty() {
        let ws = workspace("save");
        let mut m = Model::new(ws.0.clone());
        let f = ws.0.join("b.txt");
        m.open_file(&f);
        type_into_active(&mut m, "more");
        assert_eq!(m.save_active(), SaveOutcome::Saved);
        assert_eq!(disk(&f), b"bee\nmore");
        assert!(!m.tabs[0].is_dirty());
        assert!(!ws.0.join(".b.txt.nanti-tmp").exists());
        assert_eq!(m.save_active(), SaveOutcome::NothingToSave);
        type_into_active(&mut m, "!");
        assert!(m.tabs[0].is_dirty());
        assert_eq!(m.save_active(), SaveOutcome::Saved);
        assert_eq!(disk(&f), b"bee\nmore!");
    }

    #[test]
    fn save_preserves_crlf() {
        let ws = workspace("crlf");
        let f = ws.0.join("win.txt");
        std::fs::write(&f, "a\r\nb\r\n").unwrap();
        let mut m = Model::new(ws.0.clone());
        m.open_file(&f);
        {
            let e = m.active_editor_mut().unwrap();
            e.move_doc_end(false);
            e.newline();
            e.insert("c");
        }
        assert_eq!(m.save_active(), SaveOutcome::Saved);
        assert_eq!(disk(&f), b"a\r\nb\r\n\r\nc");
    }

    #[test]
    fn external_change_blocks_save_until_user_decides() {
        let ws = workspace("conf");
        let mut m = Model::new(ws.0.clone());
        let f = ws.0.join("b.txt");
        m.open_file(&f);
        type_into_active(&mut m, "mine");
        std::fs::write(&f, "theirs\n").unwrap();
        touch_future(&f);
        match m.save_active() {
            SaveOutcome::Conflict(_) => {}
            other => panic!("expected conflict, got {other:?}"),
        }
        assert!(matches!(
            m.prompt,
            Some(Prompt::SaveConflict {
                index: 0,
                then_close: false,
                ..
            })
        ));
        assert_eq!(disk(&f), b"theirs\n", "nothing was overwritten");
        assert!(m.tabs[0].is_dirty(), "my edits are still there");
        assert!(!m.resolve_save());
        assert_eq!(disk(&f), b"bee\nmine");
        assert!(!m.tabs[0].is_dirty());
    }

    #[test]
    fn conflict_can_be_resolved_by_reloading_from_disk() {
        let ws = workspace("reload");
        let mut m = Model::new(ws.0.clone());
        let f = ws.0.join("b.txt");
        m.open_file(&f);
        type_into_active(&mut m, "mine");
        std::fs::write(&f, "theirs\n").unwrap();
        touch_future(&f);
        m.save_active();
        m.resolve_discard();
        match &m.tabs[0].content {
            TabContent::Text(e) => assert_eq!(e.text(), "theirs\n"),
            TabContent::Unavailable(r) => panic!("{r}"),
            TabContent::Diff(_) => panic!("unexpected diff tab"),
        }
        assert!(!m.tabs[0].is_dirty());
        assert_eq!(disk(&f), b"theirs\n");
    }

    #[test]
    fn deleted_file_is_a_conflict_not_a_silent_recreate() {
        let ws = workspace("del");
        let mut m = Model::new(ws.0.clone());
        let f = ws.0.join("b.txt");
        m.open_file(&f);
        type_into_active(&mut m, "x");
        std::fs::remove_file(&f).unwrap();
        assert!(matches!(m.save_active(), SaveOutcome::Conflict(_)));
        assert!(!f.exists());
    }

    #[test]
    fn closing_a_dirty_tab_asks_first() {
        let ws = workspace("closedirty");
        let mut m = Model::new(ws.0.clone());
        let f = ws.0.join("b.txt");
        m.open_file(&f);
        type_into_active(&mut m, "x");
        assert!(!m.close_tab(0));
        assert_eq!(m.tabs.len(), 1, "dirty tab is not closed silently");
        assert_eq!(m.prompt, Some(Prompt::CloseDirty(0)));
        m.resolve_cancel();
        assert_eq!(m.tabs.len(), 1);
        m.close_tab(0);
        m.resolve_save();
        assert!(m.tabs.is_empty());
        assert_eq!(disk(&f), b"bee\nx");
    }

    #[test]
    fn discarding_on_close_leaves_disk_untouched() {
        let ws = workspace("discard");
        let mut m = Model::new(ws.0.clone());
        let f = ws.0.join("b.txt");
        m.open_file(&f);
        type_into_active(&mut m, "x");
        m.close_tab(0);
        m.resolve_discard();
        assert!(m.tabs.is_empty());
        assert_eq!(disk(&f), b"bee\n");
    }

    #[test]
    fn quitting_with_unsaved_tabs_asks_and_can_save_all() {
        let ws = workspace("quit");
        let mut m = Model::new(ws.0.clone());
        assert!(m.request_quit(), "nothing open: quit freely");
        m.open_file(&ws.0.join("b.txt"));
        m.open_file(&ws.0.join("A.md"));
        type_into_active(&mut m, "1");
        m.activate_tab(0);
        type_into_active(&mut m, "2");
        assert!(!m.request_quit());
        assert_eq!(m.prompt, Some(Prompt::QuitDirty));
        m.resolve_cancel();
        assert!(!m.request_quit());
        assert!(m.resolve_save(), "all saved, window may close");
        assert_eq!(disk(&ws.0.join("b.txt")), b"bee\n2");
        assert_eq!(disk(&ws.0.join("A.md")), b"# a\n1");
    }

    #[test]
    fn quit_with_discard_does_not_write() {
        let ws = workspace("quitdiscard");
        let mut m = Model::new(ws.0.clone());
        m.open_file(&ws.0.join("b.txt"));
        type_into_active(&mut m, "x");
        assert!(!m.request_quit());
        assert!(m.resolve_discard());
        assert_eq!(disk(&ws.0.join("b.txt")), b"bee\n");
    }

    #[test]
    fn unavailable_tabs_cannot_be_edited_or_saved() {
        let ws = workspace("unavail");
        let mut m = Model::new(ws.0.clone());
        m.open_file(&ws.0.join("blob.bin"));
        assert!(m.active_editor_mut().is_none());
        assert_eq!(m.save_active(), SaveOutcome::NothingToSave);
        assert!(m.close_tab(0), "nothing to lose");
    }

    #[test]
    fn quick_open_accept_opens_the_chosen_file() {
        let ws = workspace("qo");
        let mut m = Model::new(ws.0.clone());
        m.run_command(Command::QuickOpen);
        {
            let p = m.palette.as_mut().expect("palette open");
            assert_eq!(p.mode, PaletteMode::Files);
            p.field.insert("main");
            p.refilter();
        }
        assert_eq!(m.accept_palette(), None);
        assert!(m.palette.is_none());
        assert_eq!(m.active().unwrap().title, "main.rs");
    }

    #[test]
    fn command_palette_runs_the_selected_command() {
        let ws = workspace("cp");
        let mut m = Model::new(ws.0.clone());
        m.run_command(Command::CommandPalette);
        {
            let p = m.palette.as_mut().unwrap();
            assert_eq!(p.mode, PaletteMode::Commands);
            p.field.insert("toggle side");
            p.refilter();
        }
        assert!(m.layout.sidebar);
        assert_eq!(m.accept_palette(), Some(Command::ToggleSidebar));
        assert!(!m.layout.sidebar);
    }

    #[test]
    fn accepting_with_no_match_just_closes() {
        let ws = workspace("nomatch");
        let mut m = Model::new(ws.0.clone());
        m.open_palette(PaletteMode::Commands);
        {
            let p = m.palette.as_mut().unwrap();
            p.field.insert("zzzzzz");
            p.refilter();
        }
        assert_eq!(m.accept_palette(), None);
        assert!(m.palette.is_none());
    }

    #[test]
    fn open_at_selects_the_match_and_clamps_out_of_range_positions() {
        let ws = workspace("openat");
        std::fs::write(ws.0.join("t.txt"), "one\ntwo needle\nthree\n").unwrap();
        let mut m = Model::new(ws.0.clone());
        m.open_at(&ws.0.join("t.txt"), 1, 4, 6);
        assert_eq!(m.active_editor_mut().unwrap().selected_text(), "needle");
        m.open_at(&ws.0.join("t.txt"), 99, 0, 3);
        let e = m.active_editor_mut().unwrap();
        assert!(e.selection().end <= e.buffer().len_chars());
    }

    #[test]
    fn find_commands_open_the_bar_prefill_and_navigate() {
        let ws = workspace("findcmd");
        std::fs::write(ws.0.join("t.txt"), "foo bar foo").unwrap();
        let mut m = Model::new(ws.0.clone());
        m.open_file(&ws.0.join("t.txt"));
        m.active_editor_mut().unwrap().select_range(0..3);
        m.run_command(Command::Find);
        assert_eq!(
            m.find.as_ref().unwrap().query.text(),
            "foo",
            "prefilled from selection"
        );
        assert_eq!(m.find_focus(), FindFocus::Query);
        assert!(!m.find.as_ref().unwrap().show_replace);
        m.run_command(Command::Replace);
        assert!(m.find.as_ref().unwrap().show_replace);
        m.run_command(Command::FindNext);
        assert_eq!(m.active_editor_mut().unwrap().selection(), 8..11);
        m.run_command(Command::FindPrev);
        assert_eq!(m.active_editor_mut().unwrap().selection(), 0..3);
        m.close_find();
        assert!(m.find.is_none());
        assert_eq!(m.find_focus(), FindFocus::Editor);
        m.run_command(Command::FindNext);
        assert!(m.find.is_some());
    }

    #[test]
    fn find_in_files_command_shows_the_search_sidebar_with_the_selection() {
        let ws = workspace("fif");
        std::fs::write(ws.0.join("t.txt"), "alpha beta").unwrap();
        let mut m = Model::new(ws.0.clone());
        m.open_file(&ws.0.join("t.txt"));
        m.active_editor_mut().unwrap().select_range(6..10);
        m.layout.sidebar = false;
        m.run_command(Command::FindInFiles);
        assert_eq!((m.activity, m.layout.sidebar), (Activity::Search, true));
        assert!(m.search.focused);
        assert_eq!(m.search.query.text(), "beta");
        m.run_search();
        assert_eq!(m.search.results.as_ref().unwrap().total_hits, 1);
    }

    #[test]
    fn replace_all_reports_a_message_and_focus_editor_inputs_resets_targets() {
        let ws = workspace("repmsg");
        std::fs::write(ws.0.join("t.txt"), "a a a").unwrap();
        let mut m = Model::new(ws.0.clone());
        m.open_file(&ws.0.join("t.txt"));
        m.open_find(true);
        {
            let f = m.find.as_mut().unwrap();
            f.query.insert("a");
            f.replace.insert("b");
        }
        m.find_replace_all();
        assert_eq!(m.message.as_deref(), Some("Replaced 3 occurrences"));
        assert_eq!(m.active_editor_mut().unwrap().text(), "b b b");
        m.search.focused = true;
        m.focus_editor_inputs();
        assert_eq!(m.find_focus(), FindFocus::Editor);
        assert!(!m.search.focused);
    }

    #[test]
    fn terminal_commands_raise_requests_and_manage_panel_state() {
        let ws = workspace("termcmd");
        let mut m = Model::new(ws.0.clone());
        assert!(
            !m.layout.panel && !m.terminal_focused,
            "the terminal starts hidden"
        );
        m.run_command(Command::ToggleTerminal);
        assert_eq!(
            (m.layout.panel, m.terminal_focused, m.panel_tab),
            (true, true, PanelTab::Terminal)
        );
        assert_eq!(m.terminal_request.take(), Some(TerminalRequest::Ensure));
        m.run_command(Command::ToggleTerminal);
        assert_eq!((m.layout.panel, m.terminal_focused), (false, false));
        m.run_command(Command::NewTerminal);
        assert_eq!((m.layout.panel, m.terminal_focused), (true, true));
        assert_eq!(m.terminal_request.take(), Some(TerminalRequest::New));
        m.run_command(Command::KillTerminal);
        assert_eq!(m.terminal_request.take(), Some(TerminalRequest::KillActive));
        m.focus_editor_inputs();
        assert!(
            !m.terminal_focused,
            "clicking the editor takes the keyboard back"
        );
        m.run_command(Command::TogglePanel);
        assert!(!m.layout.panel);
        m.run_command(Command::TogglePanel);
        assert!(m.layout.panel);
    }

    #[test]
    fn layout_commands_show_hide_maximize_and_reset() {
        let ws = workspace("layoutcmd");
        let mut m = Model::new(ws.0.clone());
        assert!(
            m.layout.sidebar
                && !m.layout.panel
                && m.layout.right
                && m.layout.activity_bar
                && m.layout.status_bar
        );
        m.run_command(Command::ToggleSidebar);
        m.run_command(Command::ToggleRightPanel);
        m.run_command(Command::ToggleActivityBar);
        m.run_command(Command::ToggleStatusBar);
        assert!(
            !m.layout.sidebar && !m.layout.right && !m.layout.activity_bar && !m.layout.status_bar
        );
        m.run_command(Command::MaximizePanel);
        assert!(m.layout.panel && m.layout.panel_maximized);
        m.run_command(Command::MaximizePanel);
        assert!(!m.layout.panel_maximized, "second press restores");
        m.run_command(Command::MaximizePanel);
        m.run_command(Command::TogglePanel);
        assert!(!m.layout.panel && !m.layout.panel_maximized);
        m.run_command(Command::MaximizePanel);
        assert!(m.layout.panel && m.layout.panel_maximized);
        m.layout.sidebar_width = 480.0;
        m.run_command(Command::ResetLayout);
        assert_eq!(m.layout, Layout::default());
    }

    #[test]
    fn tab_navigation_commands_wrap_around() {
        let ws = workspace("tabs");
        let mut m = Model::new(ws.0.clone());
        for f in ["A.md", "b.txt", "blob.bin"] {
            m.open_file(&ws.0.join(f));
        }
        assert_eq!(m.active_tab, Some(2));
        m.run_command(Command::NextTab);
        assert_eq!(m.active_tab, Some(0));
        m.run_command(Command::PrevTab);
        assert_eq!(m.active_tab, Some(2));
    }

    #[test]
    fn view_and_edit_commands_change_state() {
        let ws = workspace("cmds");
        let mut m = Model::new(ws.0.clone());
        m.run_command(Command::ShowSearch);
        assert_eq!((m.activity, m.layout.sidebar), (Activity::Search, true));
        m.layout.sidebar = false;
        m.run_command(Command::ShowExplorer);
        assert_eq!((m.activity, m.layout.sidebar), (Activity::Explorer, true));
        m.open_file(&ws.0.join("b.txt"));
        type_into_active(&mut m, "x");
        m.run_command(Command::Undo);
        assert!(!m.tabs[0].is_dirty());
        m.run_command(Command::Redo);
        assert!(m.tabs[0].is_dirty());
        m.run_command(Command::Save);
        assert_eq!(disk(&ws.0.join("b.txt")), b"bee\nx");
        m.run_command(Command::SelectAll);
        assert_eq!(m.active_editor_mut().unwrap().selected_text(), "bee\nx");
        m.run_command(Command::CloseTab);
        assert!(m.tabs.is_empty());
    }
}
