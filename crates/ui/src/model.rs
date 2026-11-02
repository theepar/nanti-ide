//! Workbench state without any GUI types, so behaviour is unit-testable (CT-001, ED-01..03).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use editor_core::Buffer;

/// Directories never listed in the explorer (heavy or tool-owned).
const HIDDEN_DIRS: [&str; 3] = [".git", "node_modules", "target"];
/// Files above this size are not loaded into the editor (viewer limit for now).
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
            Activity::Run => "RUN AND DEBUG",
            Activity::Extensions => "EXTENSIONS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelTab {
    Problems,
    Output,
    DebugConsole,
    Terminal,
}

impl PanelTab {
    pub const ALL: [PanelTab; 4] = [
        PanelTab::Problems,
        PanelTab::Output,
        PanelTab::DebugConsole,
        PanelTab::Terminal,
    ];

    pub fn title(self) -> &'static str {
        match self {
            PanelTab::Problems => "PROBLEMS",
            PanelTab::Output => "OUTPUT",
            PanelTab::DebugConsole => "DEBUG CONSOLE",
            PanelTab::Terminal => "TERMINAL",
        }
    }
}

pub enum TabContent {
    Text(Buffer),
    /// File could not be shown (binary, too large, unreadable); the reason is displayed.
    Unavailable(String),
}

pub struct Tab {
    pub path: PathBuf,
    pub title: String,
    pub content: TabContent,
}

pub struct Model {
    pub root: PathBuf,
    expanded: BTreeSet<PathBuf>,
    pub tabs: Vec<Tab>,
    pub active_tab: Option<usize>,
    pub activity: Activity,
    pub sidebar_visible: bool,
    pub panel_tab: PanelTab,
}

impl Model {
    pub fn new(root: PathBuf) -> Self {
        let mut expanded = BTreeSet::new();
        expanded.insert(root.clone());
        Self {
            root,
            expanded,
            tabs: Vec::new(),
            active_tab: None,
            activity: Activity::Explorer,
            sidebar_visible: true,
            panel_tab: PanelTab::Terminal,
        }
    }

    /// Flattened, ordered list of what the explorer shows (directories first, then by name).
    pub fn visible_entries(&self) -> Vec<Entry> {
        let mut out = Vec::new();
        let root_name = self
            .root
            .file_name()
            .map_or_else(|| self.root.display().to_string(), |n| n.to_string_lossy().into_owned());
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
        items.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase())));
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

    /// Open `path` in a tab (or focus the existing tab). Never loads huge/binary files.
    pub fn open_file(&mut self, path: &Path) {
        if let Some(i) = self.tabs.iter().position(|t| t.path == path) {
            self.active_tab = Some(i);
            return;
        }
        let title = path
            .file_name()
            .map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
        self.tabs.push(Tab {
            path: path.to_path_buf(),
            title,
            content: load(path),
        });
        self.active_tab = Some(self.tabs.len() - 1);
    }

    pub fn activate_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = Some(index);
        }
    }

    /// Close a tab. (No dirty check yet: tabs are read-only, so there is nothing to lose.)
    pub fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        self.active_tab = match self.active_tab {
            _ if self.tabs.is_empty() => None,
            Some(a) if a > index => Some(a - 1),
            Some(a) if a == index => Some(index.min(self.tabs.len() - 1)),
            other => other,
        };
    }

    /// Clicking the active activity icon hides the sidebar; clicking another shows it.
    pub fn select_activity(&mut self, activity: Activity) {
        if self.activity == activity {
            self.sidebar_visible = !self.sidebar_visible;
        } else {
            self.activity = activity;
            self.sidebar_visible = true;
        }
    }

    pub fn active(&self) -> Option<&Tab> {
        self.active_tab.and_then(|i| self.tabs.get(i))
    }

    /// Current git branch read from `.git/HEAD` (no git subprocess); `None` outside a repo.
    pub fn git_branch(&self) -> Option<String> {
        let head = std::fs::read_to_string(self.root.join(".git").join("HEAD")).ok()?;
        let head = head.trim();
        Some(match head.strip_prefix("ref: refs/heads/") {
            Some(branch) => branch.to_owned(),
            None => format!("detached {}", head.chars().take(7).collect::<String>()),
        })
    }
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
            Ok(text) => TabContent::Text(Buffer::from_text(&text)),
            Err(_) => TabContent::Unavailable("Binary or non-UTF-8 file: not shown.".into()),
        },
        Err(e) => TabContent::Unavailable(format!("Cannot read file: {e}")),
    }
}

pub fn language_of(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
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
            .map(|e| format!("{}{}{}", "  ".repeat(e.depth), e.name, if e.is_dir { "/" } else { "" }))
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
    fn toggle_dir_expands_and_collapses() {
        let ws = workspace("toggle");
        let mut m = Model::new(ws.0.clone());
        m.toggle_dir(&ws.0.join("src"));
        assert!(names(&m).contains(&"    main.rs".to_string()));
        m.toggle_dir(&ws.0.join("src"));
        assert!(!names(&m).contains(&"    main.rs".to_string()));
        m.toggle_dir(&ws.0); // collapse root
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
            TabContent::Text(b) => assert_eq!(b.to_string(), "bee\n"),
            TabContent::Unavailable(r) => panic!("{r}"),
        }
    }

    #[test]
    fn binary_and_missing_files_are_reported_not_loaded() {
        let ws = workspace("bin");
        let mut m = Model::new(ws.0.clone());
        m.open_file(&ws.0.join("blob.bin"));
        assert!(matches!(m.active().unwrap().content, TabContent::Unavailable(_)));
        m.open_file(&ws.0.join("missing.txt"));
        assert!(matches!(m.active().unwrap().content, TabContent::Unavailable(_)));
    }

    #[test]
    fn close_tab_keeps_a_sensible_active_tab() {
        let ws = workspace("close");
        let mut m = Model::new(ws.0.clone());
        for f in ["A.md", "b.txt", "blob.bin"] {
            m.open_file(&ws.0.join(f));
        }
        m.activate_tab(1);
        m.close_tab(0); // active moves left with the list
        assert_eq!(m.active().unwrap().title, "b.txt");
        m.close_tab(0); // closing the active one selects its neighbour
        assert_eq!(m.active().unwrap().title, "blob.bin");
        m.close_tab(0);
        assert!(m.active().is_none());
        m.close_tab(5); // out of range is a no-op
    }

    #[test]
    fn activity_click_toggles_sidebar_only_for_active_icon() {
        let ws = workspace("act");
        let mut m = Model::new(ws.0.clone());
        m.select_activity(Activity::Explorer);
        assert!(!m.sidebar_visible);
        m.select_activity(Activity::Search);
        assert!(m.sidebar_visible);
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
}
