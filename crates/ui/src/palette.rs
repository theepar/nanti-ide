use std::ops::Range;
use std::path::{Path, PathBuf};

const SKIP_DIRS: [&str; 3] = [".git", "node_modules", "target"];
pub const MAX_FILES: usize = 20_000;
pub const MAX_RESULTS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    CommandPalette,
    QuickOpen,
    Save,
    CloseTab,
    NextTab,
    PrevTab,
    ToggleSidebar,
    ShowExplorer,
    ShowSearch,
    ShowSourceControl,
    ShowRun,
    ShowExtensions,
    Undo,
    Redo,
    SelectAll,
    Find,
    Replace,
    FindNext,
    FindPrev,
    FindInFiles,
    NewTerminal,
    ToggleTerminal,
    KillTerminal,
    TogglePanel,
    MaximizePanel,
    ToggleRightPanel,
    SidebarLeft,
    SidebarRight,
    PanelBottom,
    PanelTop,
    PanelLeft,
    PanelRight,
    ToggleActivityBar,
    ToggleStatusBar,
    ResetLayout,
    GitRefresh,
    GitStageAll,
    GitUnstageAll,
    GitCommit,
    NewFile,
    NewFolder,
    OpenFile,
    OpenFolder,
    SaveAll,
    CloseAllTabs,
    RenameItem,
    DeleteItem,
    CopyPath,
    RevealInOs,
    RefreshExplorer,
    CollapseAll,
    GoToLine,
    ShowProblems,
    ShowOutput,
    ClearOutput,
    InstallVsix,
    RunTask,
    About,
    Exit,
    OpenSettings,
    ToggleComment,
    MoveLineUp,
    MoveLineDown,
    DuplicateLineUp,
    DuplicateLineDown,
    DeleteLine,
    IndentLines,
    OutdentLines,
    SwitchBranch,
}

impl Command {
    pub const ALL: [Command; 69] = [
        Command::CommandPalette,
        Command::QuickOpen,
        Command::Save,
        Command::CloseTab,
        Command::NextTab,
        Command::PrevTab,
        Command::ToggleSidebar,
        Command::ShowExplorer,
        Command::ShowSearch,
        Command::ShowSourceControl,
        Command::ShowRun,
        Command::ShowExtensions,
        Command::Undo,
        Command::Redo,
        Command::SelectAll,
        Command::Find,
        Command::Replace,
        Command::FindNext,
        Command::FindPrev,
        Command::FindInFiles,
        Command::NewTerminal,
        Command::ToggleTerminal,
        Command::KillTerminal,
        Command::TogglePanel,
        Command::MaximizePanel,
        Command::ToggleRightPanel,
        Command::SidebarLeft,
        Command::SidebarRight,
        Command::PanelBottom,
        Command::PanelTop,
        Command::PanelLeft,
        Command::PanelRight,
        Command::ToggleActivityBar,
        Command::ToggleStatusBar,
        Command::ResetLayout,
        Command::GitRefresh,
        Command::GitStageAll,
        Command::GitUnstageAll,
        Command::GitCommit,
        Command::NewFile,
        Command::NewFolder,
        Command::OpenFile,
        Command::OpenFolder,
        Command::SaveAll,
        Command::CloseAllTabs,
        Command::RenameItem,
        Command::DeleteItem,
        Command::CopyPath,
        Command::RevealInOs,
        Command::RefreshExplorer,
        Command::CollapseAll,
        Command::GoToLine,
        Command::ShowProblems,
        Command::ShowOutput,
        Command::ClearOutput,
        Command::InstallVsix,
        Command::RunTask,
        Command::About,
        Command::Exit,
        Command::OpenSettings,
        Command::ToggleComment,
        Command::MoveLineUp,
        Command::MoveLineDown,
        Command::DuplicateLineUp,
        Command::DuplicateLineDown,
        Command::DeleteLine,
        Command::IndentLines,
        Command::OutdentLines,
        Command::SwitchBranch,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Command::CommandPalette => "View: Command Palette",
            Command::QuickOpen => "File: Go to File...",
            Command::Save => "File: Save",
            Command::CloseTab => "File: Close Tab",
            Command::NextTab => "View: Next Tab",
            Command::PrevTab => "View: Previous Tab",
            Command::ToggleSidebar => "View: Toggle Sidebar",
            Command::ShowExplorer => "View: Show Explorer",
            Command::ShowSearch => "View: Show Search",
            Command::ShowSourceControl => "View: Show Source Control",
            Command::ShowRun => "View: Show Run",
            Command::ShowExtensions => "View: Show Extensions",
            Command::Undo => "Edit: Undo",
            Command::Redo => "Edit: Redo",
            Command::SelectAll => "Edit: Select All",
            Command::Find => "Edit: Find",
            Command::Replace => "Edit: Replace",
            Command::FindNext => "Edit: Find Next",
            Command::FindPrev => "Edit: Find Previous",
            Command::FindInFiles => "Search: Find in Files",
            Command::NewTerminal => "Terminal: New Terminal",
            Command::ToggleTerminal => "Terminal: Toggle Terminal",
            Command::KillTerminal => "Terminal: Kill Active Terminal",
            Command::TogglePanel => "View: Toggle Panel",
            Command::MaximizePanel => "View: Maximize / Restore Panel",
            Command::ToggleRightPanel => "View: Toggle Right Panel",
            Command::SidebarLeft => "View: Move Sidebar to the Left",
            Command::SidebarRight => "View: Move Sidebar to the Right",
            Command::PanelBottom => "View: Move Panel to the Bottom",
            Command::PanelTop => "View: Move Panel to the Top",
            Command::PanelLeft => "View: Move Panel to the Left",
            Command::PanelRight => "View: Move Panel to the Right",
            Command::ToggleActivityBar => "View: Toggle Activity Bar",
            Command::ToggleStatusBar => "View: Toggle Status Bar",
            Command::ResetLayout => "View: Reset Layout",
            Command::GitRefresh => "Git: Refresh",
            Command::GitStageAll => "Git: Stage All Changes",
            Command::GitUnstageAll => "Git: Unstage All Changes",
            Command::GitCommit => "Git: Commit Staged Changes",
            Command::NewFile => "File: New File...",
            Command::NewFolder => "File: New Folder...",
            Command::OpenFile => "File: Open File...",
            Command::OpenFolder => "File: Open Folder...",
            Command::SaveAll => "File: Save All",
            Command::CloseAllTabs => "File: Close All Tabs",
            Command::RenameItem => "File: Rename...",
            Command::DeleteItem => "File: Delete (Recycle Bin)...",
            Command::CopyPath => "Explorer: Copy Path",
            Command::RevealInOs => "Explorer: Reveal in File Explorer",
            Command::RefreshExplorer => "Explorer: Refresh",
            Command::CollapseAll => "Explorer: Collapse Folders",
            Command::GoToLine => "Go: Go to Line...",
            Command::ShowProblems => "View: Show Problems",
            Command::ShowOutput => "View: Show Output",
            Command::ClearOutput => "Output: Clear",
            Command::InstallVsix => "Extensions: Install from VSIX...",
            Command::RunTask => "Run: Show Tasks",
            Command::About => "Help: About nanti.",
            Command::Exit => "File: Exit",
            Command::OpenSettings => "Preferences: Open Settings",
            Command::ToggleComment => "Edit: Toggle Line Comment",
            Command::MoveLineUp => "Edit: Move Line Up",
            Command::MoveLineDown => "Edit: Move Line Down",
            Command::DuplicateLineUp => "Edit: Copy Line Up",
            Command::DuplicateLineDown => "Edit: Copy Line Down",
            Command::DeleteLine => "Edit: Delete Line",
            Command::IndentLines => "Edit: Indent Lines",
            Command::OutdentLines => "Edit: Outdent Lines",
            Command::SwitchBranch => "Git: Switch Branch...",
        }
    }

    pub fn menu_label(self) -> &'static str {
        let t = self.title();
        t.split_once(": ").map_or(t, |(_, rest)| rest)
    }

    pub fn keybinding(self) -> Option<&'static str> {
        match self {
            Command::CommandPalette => Some("Ctrl+Shift+P"),
            Command::QuickOpen => Some("Ctrl+P"),
            Command::OpenFile => Some("Ctrl+O"),
            Command::OpenFolder => Some("Ctrl+K Ctrl+O"),
            Command::Save => Some("Ctrl+S"),
            Command::CloseTab => Some("Ctrl+W"),
            Command::NextTab => Some("Ctrl+Tab"),
            Command::PrevTab => Some("Ctrl+Shift+Tab"),
            Command::ToggleSidebar => Some("Ctrl+B"),
            Command::Undo => Some("Ctrl+Z"),
            Command::Redo => Some("Ctrl+Y"),
            Command::SelectAll => Some("Ctrl+A"),
            Command::Find => Some("Ctrl+F"),
            Command::Replace => Some("Ctrl+H"),
            Command::FindNext => Some("F3"),
            Command::FindPrev => Some("Shift+F3"),
            Command::FindInFiles => Some("Ctrl+Shift+F"),
            Command::NewTerminal => Some("Ctrl+Shift+`"),
            Command::ToggleTerminal => Some("Ctrl+`"),
            Command::TogglePanel => Some("Ctrl+J"),
            Command::MaximizePanel => Some("Ctrl+Shift+M"),
            Command::ToggleRightPanel => Some("Ctrl+Alt+B"),
            Command::ShowSourceControl => Some("Ctrl+Shift+G"),
            Command::GoToLine => Some("Ctrl+G"),
            Command::ToggleComment => Some("Ctrl+/"),
            Command::MoveLineUp => Some("Alt+Up"),
            Command::MoveLineDown => Some("Alt+Down"),
            Command::DuplicateLineUp => Some("Shift+Alt+Up"),
            Command::DuplicateLineDown => Some("Shift+Alt+Down"),
            Command::DeleteLine => Some("Ctrl+Shift+K"),
            Command::RenameItem => Some("F2"),
            Command::OpenSettings => Some("Ctrl+,"),
            _ => None,
        }
    }

    pub fn from_ctrl_key(key: &str, shift: bool) -> Option<Command> {
        Some(match (key, shift) {
            ("p", true) => Command::CommandPalette,
            ("p", false) => Command::QuickOpen,
            ("s", _) => Command::Save,
            ("w", _) => Command::CloseTab,
            ("b", _) => Command::ToggleSidebar,
            (",", _) => Command::OpenSettings,
            ("tab", false) | ("pagedown", _) => Command::NextTab,
            ("tab", true) | ("pageup", _) => Command::PrevTab,
            ("z", false) => Command::Undo,
            ("z", true) | ("y", _) => Command::Redo,
            ("a", _) => Command::SelectAll,
            ("f", true) => Command::FindInFiles,
            ("f", false) => Command::Find,
            ("g", true) => Command::ShowSourceControl,
            ("g", false) => Command::GoToLine,
            ("/", _) => Command::ToggleComment,
            ("k", true) => Command::DeleteLine,
            ("h", _) => Command::Replace,
            ("`", true) => Command::NewTerminal,
            ("`", false) => Command::ToggleTerminal,
            ("j", _) => Command::TogglePanel,
            ("m", true) => Command::MaximizePanel,
            _ => return None,
        })
    }
}

pub fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let cand: Vec<char> = candidate.chars().collect();
    let name_start = cand
        .iter()
        .rposition(|c| *c == '/' || *c == '\\')
        .map_or(0, |i| i + 1);
    let mut score = 0i32;
    let mut ci = 0usize;
    let mut prev_match: Option<usize> = None;
    for qc in query.chars() {
        let qc = qc.to_ascii_lowercase();
        let found = (ci..cand.len()).find(|&i| cand[i].to_ascii_lowercase() == qc)?;
        score += 10;
        if prev_match == Some(found.wrapping_sub(1)) && found > 0 {
            score += 15;
        }
        let word_start =
            found == 0 || matches!(cand[found - 1], '/' | '\\' | '_' | '-' | ' ' | '.' | ':');
        if word_start {
            score += 12;
        }
        if found >= name_start {
            score += 6;
        }
        score -= (found - ci) as i32;
        prev_match = Some(found);
        ci = found + 1;
    }
    Some(score - (cand.len() as i32) / 8)
}

#[derive(Debug, Default, Clone)]
pub struct TextField {
    text: String,
    cursor: usize,
    marked: Option<Range<usize>>,
}

impl TextField {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn marked_range(&self) -> Option<Range<usize>> {
        self.marked.clone()
    }

    fn len(&self) -> usize {
        self.text.chars().count()
    }

    fn byte_at(&self, char_idx: usize) -> usize {
        self.text
            .char_indices()
            .nth(char_idx)
            .map_or(self.text.len(), |(b, _)| b)
    }

    fn replace(&mut self, range: Range<usize>, with: &str) {
        let (a, b) = (self.byte_at(range.start), self.byte_at(range.end));
        self.text.replace_range(a..b, with);
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.marked = None;
    }

    pub fn insert(&mut self, s: &str) {
        self.marked = None;
        let s: String = s.chars().filter(|c| !matches!(c, '\n' | '\r')).collect();
        self.replace(self.cursor..self.cursor, &s);
        self.cursor += s.chars().count();
    }

    pub fn backspace(&mut self) {
        self.marked = None;
        if self.cursor > 0 {
            self.replace(self.cursor - 1..self.cursor, "");
            self.cursor -= 1;
        }
    }

    pub fn delete(&mut self) {
        self.marked = None;
        if self.cursor < self.len() {
            self.replace(self.cursor..self.cursor + 1, "");
        }
    }

    pub fn left(&mut self) {
        self.marked = None;
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.marked = None;
        self.cursor = (self.cursor + 1).min(self.len());
    }

    pub fn home(&mut self) {
        self.marked = None;
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.marked = None;
        self.cursor = self.len();
    }

    pub fn compose(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
    ) {
        let text: String = text.chars().filter(|c| !matches!(c, '\n' | '\r')).collect();
        let target = self
            .marked
            .take()
            .or(range)
            .unwrap_or(self.cursor..self.cursor);
        let target = target.start.min(self.len())..target.end.min(self.len());
        let start = target.start;
        let n = text.chars().count();
        self.replace(target, &text);
        self.marked = Some(start..start + n);
        self.cursor = start + selected.map_or(n, |s| s.end.min(n));
    }

    pub fn commit(&mut self, range: Option<Range<usize>>, text: &str) {
        let text: String = text.chars().filter(|c| !matches!(c, '\n' | '\r')).collect();
        let target = self
            .marked
            .take()
            .or(range)
            .unwrap_or(self.cursor..self.cursor);
        let target = target.start.min(self.len())..target.end.min(self.len());
        let start = target.start;
        self.replace(target, &text);
        self.cursor = start + text.chars().count();
    }

    pub fn end_composition(&mut self) {
        self.marked = None;
    }

    pub fn char_to_utf16(&self, char_idx: usize) -> usize {
        self.text.chars().take(char_idx).map(char::len_utf16).sum()
    }

    pub fn utf16_to_char(&self, utf16_idx: usize) -> usize {
        let mut units = 0;
        for (i, c) in self.text.chars().enumerate() {
            if units >= utf16_idx {
                return i;
            }
            units += c.len_utf16();
        }
        self.len()
    }

    pub fn slice(&self, range: Range<usize>) -> String {
        self.text
            .chars()
            .skip(range.start)
            .take(range.end.saturating_sub(range.start))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteMode {
    Commands,
    Files,
    Branches,
    Input(InputKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputKind {
    GoToLine,
    NewFile(PathBuf),
    NewFolder(PathBuf),
    Rename(PathBuf),
    InstallVsix,
    NewBranch,
}

impl InputKind {
    pub fn placeholder(&self) -> &'static str {
        match self {
            InputKind::GoToLine => "Line number, or line:column",
            InputKind::NewFile(_) => "File name (a/b/c.rs creates folders too)",
            InputKind::NewFolder(_) => "Folder name",
            InputKind::Rename(_) => "New name",
            InputKind::InstallVsix => "Full path of a .vsix file",
            InputKind::NewBranch => "New branch name",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteAction {
    Command(Command),
    OpenFile(PathBuf),
    Checkout(String),
    NewBranchPrompt,
    Submit(InputKind, String),
}

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub label: String,
    pub detail: Option<String>,
    pub action: PaletteAction,
}

pub struct Palette {
    pub mode: PaletteMode,
    pub field: TextField,
    items: Vec<PaletteItem>,
    filtered: Vec<usize>,
    pub selected: usize,
    pub error: Option<String>,
}

impl Palette {
    pub fn commands() -> Self {
        let items = Command::ALL
            .iter()
            .map(|c| PaletteItem {
                label: c.title().to_owned(),
                detail: c.keybinding().map(str::to_owned),
                action: PaletteAction::Command(*c),
            })
            .collect();
        Self::build(PaletteMode::Commands, items)
    }

    pub fn files(root: &Path) -> Self {
        let items = list_files(root)
            .into_iter()
            .map(|(rel, abs)| PaletteItem {
                label: rel,
                detail: None,
                action: PaletteAction::OpenFile(abs),
            })
            .collect();
        Self::build(PaletteMode::Files, items)
    }

    fn build(mode: PaletteMode, items: Vec<PaletteItem>) -> Self {
        let mut p = Self {
            mode,
            field: TextField::default(),
            items,
            filtered: Vec::new(),
            selected: 0,
            error: None,
        };
        p.refilter();
        p
    }

    pub fn branches(list: &[git::Branch]) -> Self {
        let mut items = vec![PaletteItem {
            label: "Create new branch...".to_owned(),
            detail: None,
            action: PaletteAction::NewBranchPrompt,
        }];
        items.extend(list.iter().map(|b| PaletteItem {
            label: b.name.clone(),
            detail: b.current.then(|| "current".to_owned()),
            action: PaletteAction::Checkout(b.name.clone()),
        }));
        Self::build(PaletteMode::Branches, items)
    }

    pub fn input(kind: InputKind, initial: &str) -> Self {
        let mut p = Self::build(PaletteMode::Input(kind), Vec::new());
        p.field.insert(initial);
        p
    }

    pub fn refilter(&mut self) {
        self.error = None;
        if matches!(self.mode, PaletteMode::Input(_)) {
            return;
        }
        let q = self.field.text().trim().to_owned();
        let mut scored: Vec<(i32, usize)> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, it)| fuzzy_score(&q, &it.label).map(|s| (s, i)))
            .collect();
        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| self.items[a.1].label.cmp(&self.items[b.1].label))
        });
        self.filtered = scored
            .into_iter()
            .take(MAX_RESULTS)
            .map(|(_, i)| i)
            .collect();
        self.selected = 0;
    }

    pub fn results(&self) -> impl Iterator<Item = &PaletteItem> {
        self.filtered.iter().map(|&i| &self.items[i])
    }

    pub fn result_count(&self) -> usize {
        self.filtered.len()
    }

    pub fn move_selection(&mut self, delta: isize) {
        let n = self.filtered.len();
        if n == 0 {
            return;
        }
        self.selected = (self.selected as isize + delta).rem_euclid(n as isize) as usize;
    }

    pub fn select(&mut self, index: usize) {
        if index < self.filtered.len() {
            self.selected = index;
        }
    }

    pub fn selected_action(&self) -> Option<PaletteAction> {
        if let PaletteMode::Input(kind) = &self.mode {
            return Some(PaletteAction::Submit(
                kind.clone(),
                self.field.text().to_owned(),
            ));
        }
        self.filtered
            .get(self.selected)
            .map(|&i| self.items[i].action.clone())
    }

    pub fn placeholder(&self) -> &'static str {
        match &self.mode {
            PaletteMode::Commands => "Type a command",
            PaletteMode::Files => "Type a file name to open",
            PaletteMode::Branches => "Select a branch to switch to",
            PaletteMode::Input(k) => k.placeholder(),
        }
    }
}

pub fn list_files(root: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.filter_map(Result::ok) {
            let Ok(ft) = entry.file_type() else { continue };
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            if ft.is_dir() {
                if !SKIP_DIRS.contains(&name.as_str()) {
                    stack.push(path);
                }
            } else if ft.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, path));
                if out.len() >= MAX_FILES {
                    out.sort();
                    return out;
                }
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_requires_ordered_subsequence_case_insensitively() {
        assert!(fuzzy_score("sav", "File: Save").is_some());
        assert!(fuzzy_score("FS", "File: Save").is_some());
        assert!(fuzzy_score("zq", "File: Save").is_none());
        assert!(fuzzy_score("", "anything").is_some());
        assert!(fuzzy_score("ba", "ab").is_none(), "order matters");
    }

    #[test]
    fn fuzzy_prefers_consecutive_word_start_and_filename_matches() {
        let exact = fuzzy_score("main", "crates/app/src/main.rs").unwrap();
        let scattered = fuzzy_score("main", "my_awesome_item_name.rs").unwrap();
        assert!(exact > scattered, "{exact} vs {scattered}");
        let in_name = fuzzy_score("lib", "crates/ui/src/lib.rs").unwrap();
        let in_dir = fuzzy_score("lib", "lib/other/zzz.rs").unwrap();
        assert!(in_name > 0 && in_dir > 0);
        let short = fuzzy_score("rs", "a.rs").unwrap();
        let long = fuzzy_score("rs", "a/very/long/directory/structure/file.rs").unwrap();
        assert!(short > long);
    }

    #[test]
    fn command_palette_lists_only_real_commands_and_filters() {
        let mut p = Palette::commands();
        assert_eq!(p.result_count(), Command::ALL.len());
        p.field.insert("save");
        p.refilter();
        assert_eq!(
            p.selected_action(),
            Some(PaletteAction::Command(Command::Save))
        );
        p.field.clear();
        p.field.insert("zzzz");
        p.refilter();
        assert_eq!(p.result_count(), 0);
        assert_eq!(p.selected_action(), None);
    }

    #[test]
    fn selection_wraps_and_resets_on_new_query() {
        let mut p = Palette::commands();
        p.move_selection(-1);
        assert_eq!(p.selected, Command::ALL.len() - 1);
        p.move_selection(1);
        assert_eq!(p.selected, 0);
        p.move_selection(2);
        p.field.insert("v");
        p.refilter();
        assert_eq!(p.selected, 0);
    }

    #[test]
    fn ctrl_shortcuts_map_to_commands() {
        assert_eq!(
            Command::from_ctrl_key("p", true),
            Some(Command::CommandPalette)
        );
        assert_eq!(Command::from_ctrl_key("p", false), Some(Command::QuickOpen));
        assert_eq!(Command::from_ctrl_key("s", false), Some(Command::Save));
        assert_eq!(Command::from_ctrl_key("tab", true), Some(Command::PrevTab));
        assert_eq!(Command::from_ctrl_key("z", true), Some(Command::Redo));
        assert_eq!(Command::from_ctrl_key("q", false), None);
        for c in Command::ALL {
            if let Some(kb) = c.keybinding() {
                assert!(
                    kb.contains("Ctrl+") || kb.contains("Alt+") || kb.contains("F3") || kb == "F2",
                    "{kb}"
                );
            }
        }
    }

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nanti-pal-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("src/deep")).unwrap();
        std::fs::create_dir_all(d.join("node_modules/x")).unwrap();
        std::fs::create_dir_all(d.join(".git")).unwrap();
        std::fs::write(d.join("README.md"), "").unwrap();
        std::fs::write(d.join("src/main.rs"), "").unwrap();
        std::fs::write(d.join("src/deep/util.rs"), "").unwrap();
        std::fs::write(d.join("node_modules/x/i.js"), "").unwrap();
        std::fs::write(d.join(".git/HEAD"), "").unwrap();
        d
    }

    #[test]
    fn quick_open_lists_files_skipping_heavy_dirs_and_ranks_matches() {
        let d = tmp("files");
        let names: Vec<String> = list_files(&d).into_iter().map(|(r, _)| r).collect();
        assert_eq!(names, ["README.md", "src/deep/util.rs", "src/main.rs"]);
        let mut p = Palette::files(&d);
        p.field.insert("main");
        p.refilter();
        match p.selected_action() {
            Some(PaletteAction::OpenFile(path)) => {
                assert!(path.ends_with("src/main.rs") || path.ends_with("src\\main.rs"))
            }
            other => panic!("{other:?}"),
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn text_field_edits_with_unicode() {
        let mut f = TextField::default();
        f.insert("héllo");
        f.left();
        f.left();
        f.insert("日");
        assert_eq!((f.text(), f.cursor()), ("hél日lo", 4));
        f.backspace();
        f.delete();
        assert_eq!(f.text(), "hélo");
        f.home();
        f.delete();
        f.end();
        assert_eq!((f.text(), f.cursor()), ("élo", 3));
        f.insert("a\nb");
        assert_eq!(f.text(), "éloab");
    }

    #[test]
    fn text_field_ime_composition_replaces_marked_text_then_commits() {
        let mut f = TextField::default();
        f.insert("ab");
        f.left();
        f.compose(None, "n", Some(1..1));
        assert_eq!((f.text(), f.marked_range()), ("anb", Some(1..2)));
        f.compose(None, "ni", Some(2..2));
        assert_eq!(f.text(), "anib");
        f.commit(None, "你");
        assert_eq!((f.text(), f.cursor(), f.marked_range()), ("a你b", 2, None));
    }

    #[test]
    fn text_field_utf16_mapping_counts_surrogates() {
        let mut f = TextField::default();
        f.insert("a🌍b");
        assert_eq!(f.char_to_utf16(2), 3);
        assert_eq!(f.utf16_to_char(3), 2);
        assert_eq!(f.utf16_to_char(99), 3);
        assert_eq!(f.slice(1..2), "🌍");
    }
}
