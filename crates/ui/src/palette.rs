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
    ChangeLanguageMode,
}

impl Command {
    pub const ALL: [Command; 70] = [
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
        Command::ChangeLanguageMode,
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
            Command::ChangeLanguageMode => "Change Language Mode",
        }
    }

    pub fn menu_label(self) -> &'static str {
        let t = self.title();
        t.split_once(": ").map_or(t, |(_, rest)| rest)
    }

    pub fn keybinding(self) -> Option<&'static str> {
        match self {
            Command::CommandPalette => Some("Ctrl+Shift+P"),
            Command::ChangeLanguageMode => Some("Ctrl+K M"),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageBadge {
    pub glyph: &'static str,
    pub color: u32,
    pub is_bracket: bool,
}

impl LanguageBadge {
    pub const fn bracket(color: u32) -> Self {
        Self {
            glyph: "{ }",
            color,
            is_bracket: true,
        }
    }

    pub const fn text(glyph: &'static str, color: u32) -> Self {
        Self {
            glyph,
            color,
            is_bracket: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageEntry {
    pub name: &'static str,
    pub id: &'static str,
    pub exts: &'static str,
    pub badge: LanguageBadge,
}

pub const LANGUAGE_LIST: &[LanguageEntry] = &[
    LanguageEntry {
        name: "Ada",
        id: "ada",
        exts: ".adb, .ads",
        badge: LanguageBadge::text("ADA", 0x02f88c),
    },
    LanguageEntry {
        name: "Agent",
        id: "chatagent",
        exts: ".agent",
        badge: LanguageBadge::text("📄", 0x888888),
    },
    LanguageEntry {
        name: "Assembly",
        id: "assembly",
        exts: ".asm, .s",
        badge: LanguageBadge::text("ASM", 0x6e4c13),
    },
    LanguageEntry {
        name: "Batch",
        id: "bat",
        exts: ".bat, .cmd",
        badge: LanguageBadge::text(">_", 0x4bc589),
    },
    LanguageEntry {
        name: "BibTeX",
        id: "bibtex",
        exts: ".bib",
        badge: LanguageBadge::text("📄", 0x888888),
    },
    LanguageEntry {
        name: "Binary",
        id: "code-text-binary",
        exts: ".bin",
        badge: LanguageBadge::text("📄", 0x888888),
    },
    LanguageEntry {
        name: "Blade",
        id: "blade",
        exts: ".blade.php",
        badge: LanguageBadge::text("📄", 0xf05340),
    },
    LanguageEntry {
        name: "C",
        id: "c",
        exts: ".c, .h",
        badge: LanguageBadge::text("C", 0x00599c),
    },
    LanguageEntry {
        name: "C#",
        id: "csharp",
        exts: ".cs",
        badge: LanguageBadge::text("C#", 0x178600),
    },
    LanguageEntry {
        name: "C++",
        id: "cpp",
        exts: ".cpp, .hpp, .cc",
        badge: LanguageBadge::text("C++", 0x00599c),
    },
    LanguageEntry {
        name: "Clojure",
        id: "clojure",
        exts: ".clj",
        badge: LanguageBadge::text("λ", 0x5881d8),
    },
    LanguageEntry {
        name: "CMake",
        id: "cmake",
        exts: "CMakeLists.txt, .cmake",
        badge: LanguageBadge::text("📄", 0x064f8c),
    },
    LanguageEntry {
        name: "COBOL",
        id: "cobol",
        exts: ".cob, .cbl",
        badge: LanguageBadge::text("COB", 0x2b3269),
    },
    LanguageEntry {
        name: "Code Snippets",
        id: "snippets",
        exts: ".code-snippets",
        badge: LanguageBadge::text("📄", 0x888888),
    },
    LanguageEntry {
        name: "Codex Rules",
        id: "codex-rules",
        exts: ".codexrules",
        badge: LanguageBadge::text("📄", 0x888888),
    },
    LanguageEntry {
        name: "CoffeeScript",
        id: "coffeescript",
        exts: ".coffee",
        badge: LanguageBadge::text("☕", 0x244776),
    },
    LanguageEntry {
        name: "Compose",
        id: "dockercompose",
        exts: "docker-compose.yml",
        badge: LanguageBadge::text("🐳", 0x2496ed),
    },
    LanguageEntry {
        name: "Crystal",
        id: "crystal",
        exts: ".cr",
        badge: LanguageBadge::text("CR", 0x000100),
    },
    LanguageEntry {
        name: "CSS",
        id: "css",
        exts: ".css",
        badge: LanguageBadge::bracket(0x42a5f5),
    },
    LanguageEntry {
        name: "CUDA C++",
        id: "cuda-cpp",
        exts: ".cu, .cuh",
        badge: LanguageBadge::text("📄", 0x3a4e3a),
    },
    LanguageEntry {
        name: "D",
        id: "d",
        exts: ".d",
        badge: LanguageBadge::text("D", 0xba595e),
    },
    LanguageEntry {
        name: "Dart",
        id: "dart",
        exts: ".dart",
        badge: LanguageBadge::text("◈", 0x00b4ab),
    },
    LanguageEntry {
        name: "Delphi / Object Pascal",
        id: "delphi",
        exts: ".pas, .dpr",
        badge: LanguageBadge::text("D", 0xb0ce4e),
    },
    LanguageEntry {
        name: "Diff",
        id: "diff",
        exts: ".diff, .patch",
        badge: LanguageBadge::text("📄", 0x888888),
    },
    LanguageEntry {
        name: "Docker",
        id: "dockerfile",
        exts: "Dockerfile, .dockerfile",
        badge: LanguageBadge::text("🐳", 0x2496ed),
    },
    LanguageEntry {
        name: "Dotenv",
        id: "dotenv",
        exts: ".env",
        badge: LanguageBadge::text("📄", 0x888888),
    },
    LanguageEntry {
        name: "Elixir",
        id: "elixir",
        exts: ".ex, .exs",
        badge: LanguageBadge::text("💧", 0x6e4a7e),
    },
    LanguageEntry {
        name: "Elm",
        id: "elm",
        exts: ".elm",
        badge: LanguageBadge::text("◈", 0x60b5cc),
    },
    LanguageEntry {
        name: "Erlang",
        id: "erlang",
        exts: ".erl, .hrl",
        badge: LanguageBadge::text("◈", 0xb83998),
    },
    LanguageEntry {
        name: "F#",
        id: "fsharp",
        exts: ".fs",
        badge: LanguageBadge::text("F#", 0x30b980),
    },
    LanguageEntry {
        name: "Fortran",
        id: "fortran",
        exts: ".f90, .f95",
        badge: LanguageBadge::text("F", 0x4d41b1),
    },
    LanguageEntry {
        name: "Git Commit Message",
        id: "git-commit",
        exts: "COMMIT_EDITMSG",
        badge: LanguageBadge::text("⑂", 0xf05033),
    },
    LanguageEntry {
        name: "Git Rebase Message",
        id: "git-rebase",
        exts: "git-rebase-todo",
        badge: LanguageBadge::text("⑂", 0xf05033),
    },
    LanguageEntry {
        name: "GitHub Actions Workflow",
        id: "github-actions-workflow",
        exts: ".github/workflows/*.yml",
        badge: LanguageBadge::text("⑂", 0x2088ff),
    },
    LanguageEntry {
        name: "Go",
        id: "go",
        exts: ".go",
        badge: LanguageBadge::text("GO", 0x00add8),
    },
    LanguageEntry {
        name: "Go Assembly",
        id: "go.asm",
        exts: ".s",
        badge: LanguageBadge::text("📄", 0x00add8),
    },
    LanguageEntry {
        name: "Go Checksum File",
        id: "go.sum",
        exts: "go.sum",
        badge: LanguageBadge::text("📄", 0x00add8),
    },
    LanguageEntry {
        name: "Go Module File",
        id: "go.mod",
        exts: "go.mod",
        badge: LanguageBadge::text("📄", 0x00add8),
    },
    LanguageEntry {
        name: "Go Template File",
        id: "gotmpl",
        exts: ".gotmpl, .tmpl",
        badge: LanguageBadge::text("📄", 0x00add8),
    },
    LanguageEntry {
        name: "Go Work File",
        id: "go.work",
        exts: "go.work",
        badge: LanguageBadge::text("📄", 0x00add8),
    },
    LanguageEntry {
        name: "Gradle",
        id: "gradle",
        exts: ".gradle",
        badge: LanguageBadge::text("📄", 0x02303a),
    },
    LanguageEntry {
        name: "gradle-build",
        id: "gradle-build",
        exts: "build.gradle",
        badge: LanguageBadge::text("📄", 0x02303a),
    },
    LanguageEntry {
        name: "GraphQL",
        id: "graphql",
        exts: ".graphql, .gql",
        badge: LanguageBadge::text("⬡", 0xe10098),
    },
    LanguageEntry {
        name: "Groovy",
        id: "groovy",
        exts: ".groovy",
        badge: LanguageBadge::text("📄", 0x4298b8),
    },
    LanguageEntry {
        name: "Handlebars",
        id: "handlebars",
        exts: ".hbs, .handlebars",
        badge: LanguageBadge::bracket(0xf0772b),
    },
    LanguageEntry {
        name: "Haskell",
        id: "haskell",
        exts: ".hs",
        badge: LanguageBadge::text("λ", 0x5e5086),
    },
    LanguageEntry {
        name: "HLSL",
        id: "hlsl",
        exts: ".hlsl",
        badge: LanguageBadge::bracket(0x6c5ce7),
    },
    LanguageEntry {
        name: "HTML",
        id: "html",
        exts: ".html, .htm",
        badge: LanguageBadge::bracket(0xe44d26),
    },
    LanguageEntry {
        name: "Ignore",
        id: "ignore",
        exts: ".gitignore, .dockerignore",
        badge: LanguageBadge::text("📄", 0x888888),
    },
    LanguageEntry {
        name: "Ini",
        id: "ini",
        exts: ".ini, .cfg",
        badge: LanguageBadge::text("📄", 0x888888),
    },
    LanguageEntry {
        name: "Instructions",
        id: "instructions",
        exts: ".instructions",
        badge: LanguageBadge::text("📄", 0x888888),
    },
    LanguageEntry {
        name: "Java",
        id: "java",
        exts: ".java",
        badge: LanguageBadge::text("☕", 0xb07219),
    },
    LanguageEntry {
        name: "JavaScript",
        id: "javascript",
        exts: ".js, .mjs, .cjs",
        badge: LanguageBadge::text("JS", 0xf7df1e),
    },
    LanguageEntry {
        name: "JavaScript JSX",
        id: "javascriptreact",
        exts: ".jsx",
        badge: LanguageBadge::text("⚛", 0x61dafb),
    },
    LanguageEntry {
        name: "Jinja",
        id: "jinja",
        exts: ".jinja, .j2",
        badge: LanguageBadge::bracket(0xb41717),
    },
    LanguageEntry {
        name: "JSON",
        id: "json",
        exts: ".json",
        badge: LanguageBadge::bracket(0xcbcb41),
    },
    LanguageEntry {
        name: "JSON Lines",
        id: "jsonl",
        exts: ".jsonl",
        badge: LanguageBadge::text("📄", 0xcbcb41),
    },
    LanguageEntry {
        name: "JSON with Comments",
        id: "jsonc",
        exts: ".jsonc",
        badge: LanguageBadge::bracket(0xcbcb41),
    },
    LanguageEntry {
        name: "Julia",
        id: "julia",
        exts: ".jl",
        badge: LanguageBadge::text("JL", 0xa270ba),
    },
    LanguageEntry {
        name: "Julia Markdown",
        id: "juliamarkdown",
        exts: ".jmd",
        badge: LanguageBadge::text("📄", 0xa270ba),
    },
    LanguageEntry {
        name: "Kotlin",
        id: "kotlin",
        exts: ".kt, .kts",
        badge: LanguageBadge::text("KT", 0xa97bff),
    },
    LanguageEntry {
        name: "LaTeX",
        id: "latex",
        exts: ".tex, .latex",
        badge: LanguageBadge::text("T", 0x008080),
    },
    LanguageEntry {
        name: "Less",
        id: "less",
        exts: ".less",
        badge: LanguageBadge::bracket(0x1d365d),
    },
    LanguageEntry {
        name: "Lisp",
        id: "lisp",
        exts: ".lisp, .lsp",
        badge: LanguageBadge::text("λ", 0x3fb68b),
    },
    LanguageEntry {
        name: "Lua",
        id: "lua",
        exts: ".lua",
        badge: LanguageBadge::text("Lua", 0x000080),
    },
    LanguageEntry {
        name: "Makefile",
        id: "makefile",
        exts: "Makefile, .mk",
        badge: LanguageBadge::text("📄", 0x427819),
    },
    LanguageEntry {
        name: "Markdown",
        id: "markdown",
        exts: ".md",
        badge: LanguageBadge::text("M↓", 0x083fa1),
    },
    LanguageEntry {
        name: "MATLAB",
        id: "matlab",
        exts: ".m",
        badge: LanguageBadge::text("M", 0xe16737),
    },
    LanguageEntry {
        name: "Move",
        id: "move",
        exts: ".move",
        badge: LanguageBadge::text("◈", 0x4a90e2),
    },
    LanguageEntry {
        name: "Nim",
        id: "nim",
        exts: ".nim",
        badge: LanguageBadge::text("NIM", 0xffc200),
    },
    LanguageEntry {
        name: "Nix",
        id: "nix",
        exts: ".nix",
        badge: LanguageBadge::text("❄", 0x7e7eff),
    },
    LanguageEntry {
        name: "Objective-C",
        id: "objective-c",
        exts: ".m, .mm",
        badge: LanguageBadge::text("M", 0x438eff),
    },
    LanguageEntry {
        name: "OCaml",
        id: "ocaml",
        exts: ".ml",
        badge: LanguageBadge::text("◈", 0x3be133),
    },
    LanguageEntry {
        name: "Octave",
        id: "octave",
        exts: ".m",
        badge: LanguageBadge::text("M", 0x0794bc),
    },
    LanguageEntry {
        name: "Pascal",
        id: "pascal",
        exts: ".pas",
        badge: LanguageBadge::text("P", 0xe3f171),
    },
    LanguageEntry {
        name: "Perl",
        id: "perl",
        exts: ".pl, .pm",
        badge: LanguageBadge::text("PL", 0x0298c3),
    },
    LanguageEntry {
        name: "PHP",
        id: "php",
        exts: ".php",
        badge: LanguageBadge::text("PHP", 0x4f5d95),
    },
    LanguageEntry {
        name: "Plain Text",
        id: "plaintext",
        exts: ".txt",
        badge: LanguageBadge::text("📄", 0x888888),
    },
    LanguageEntry {
        name: "PowerShell",
        id: "powershell",
        exts: ".ps1",
        badge: LanguageBadge::text("PS", 0x012456),
    },
    LanguageEntry {
        name: "Prolog",
        id: "prolog",
        exts: ".pl",
        badge: LanguageBadge::text("◈", 0x74283c),
    },
    LanguageEntry {
        name: "Protocol Buffer",
        id: "proto",
        exts: ".proto",
        badge: LanguageBadge::text("📄", 0x438eff),
    },
    LanguageEntry {
        name: "PureScript",
        id: "purescript",
        exts: ".purs",
        badge: LanguageBadge::text("◈", 0x1d222d),
    },
    LanguageEntry {
        name: "Python",
        id: "python",
        exts: ".py, .pyw",
        badge: LanguageBadge::text("PY", 0x3572a5),
    },
    LanguageEntry {
        name: "R",
        id: "r",
        exts: ".r, .R",
        badge: LanguageBadge::text("R", 0x198ce7),
    },
    LanguageEntry {
        name: "Racket",
        id: "racket",
        exts: ".rkt",
        badge: LanguageBadge::text("λ", 0x3c5caa),
    },
    LanguageEntry {
        name: "Ruby",
        id: "ruby",
        exts: ".rb",
        badge: LanguageBadge::text("RB", 0x701516),
    },
    LanguageEntry {
        name: "Rust",
        id: "rust",
        exts: ".rs",
        badge: LanguageBadge::text("RS", 0xdea584),
    },
    LanguageEntry {
        name: "Sass",
        id: "sass",
        exts: ".sass",
        badge: LanguageBadge::bracket(0xa53b70),
    },
    LanguageEntry {
        name: "SCSS",
        id: "scss",
        exts: ".scss",
        badge: LanguageBadge::bracket(0xc6538c),
    },
    LanguageEntry {
        name: "Scala",
        id: "scala",
        exts: ".scala",
        badge: LanguageBadge::text("S", 0xc22d40),
    },
    LanguageEntry {
        name: "Scheme",
        id: "scheme",
        exts: ".scm",
        badge: LanguageBadge::text("λ", 0x1e4aec),
    },
    LanguageEntry {
        name: "Shell Script",
        id: "bash",
        exts: ".sh, .bash",
        badge: LanguageBadge::text(">_", 0x89e051),
    },
    LanguageEntry {
        name: "Smalltalk",
        id: "smalltalk",
        exts: ".st",
        badge: LanguageBadge::text("◈", 0x596706),
    },
    LanguageEntry {
        name: "Solidity",
        id: "solidity",
        exts: ".sol",
        badge: LanguageBadge::text("◈", 0xaa6746),
    },
    LanguageEntry {
        name: "SQL",
        id: "sql",
        exts: ".sql",
        badge: LanguageBadge::text("🗄", 0xe38c00),
    },
    LanguageEntry {
        name: "Svelte",
        id: "svelte",
        exts: ".svelte",
        badge: LanguageBadge::text("S", 0xff3e00),
    },
    LanguageEntry {
        name: "Swift",
        id: "swift",
        exts: ".swift",
        badge: LanguageBadge::text("SW", 0xf05138),
    },
    LanguageEntry {
        name: "TOML",
        id: "toml",
        exts: ".toml",
        badge: LanguageBadge::bracket(0x9c4221),
    },
    LanguageEntry {
        name: "TypeScript",
        id: "typescript",
        exts: ".ts, .mts",
        badge: LanguageBadge::text("TS", 0x3178c6),
    },
    LanguageEntry {
        name: "TypeScript React",
        id: "typescriptreact",
        exts: ".tsx",
        badge: LanguageBadge::text("⚛", 0x3178c6),
    },
    LanguageEntry {
        name: "V",
        id: "v",
        exts: ".v",
        badge: LanguageBadge::text("V", 0x4f87c4),
    },
    LanguageEntry {
        name: "Visual Basic",
        id: "vb",
        exts: ".vb",
        badge: LanguageBadge::text("VB", 0x945db7),
    },
    LanguageEntry {
        name: "Vue",
        id: "vue",
        exts: ".vue",
        badge: LanguageBadge::text("V", 0x41b883),
    },
    LanguageEntry {
        name: "WebAssembly Text",
        id: "wat",
        exts: ".wat",
        badge: LanguageBadge::text("WAT", 0x654ff0),
    },
    LanguageEntry {
        name: "XML",
        id: "xml",
        exts: ".xml",
        badge: LanguageBadge::bracket(0x0060aa),
    },
    LanguageEntry {
        name: "YAML",
        id: "yaml",
        exts: ".yaml, .yml",
        badge: LanguageBadge::bracket(0xcb171e),
    },
    LanguageEntry {
        name: "Zig",
        id: "zig",
        exts: ".zig",
        badge: LanguageBadge::text("ZIG", 0xec915c),
    },
];

pub fn find_language_by_id(id: &str) -> Option<&'static LanguageEntry> {
    LANGUAGE_LIST.iter().find(|e| e.id.eq_ignore_ascii_case(id))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteMode {
    Commands,
    Files,
    Branches,
    LanguageMode,
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
    SetLanguageMode(Option<String>),
    ExtensionCommand {
        id: String,
        label: String,
        ext_dir: Option<PathBuf>,
        view_type: Option<String>,
    },
    Submit(InputKind, String),
}

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub label: String,
    pub detail: Option<String>,
    pub action: PaletteAction,
    pub badge: Option<LanguageBadge>,
}

pub struct Palette {
    pub mode: PaletteMode,
    pub field: TextField,
    items: Vec<PaletteItem>,
    filtered: Vec<usize>,
    pub selected: usize,
    pub error: Option<String>,
    pub loading: bool,
}

impl Palette {
    pub fn commands(extensions: &[crate::extensions::Extension]) -> Self {
        let mut items: Vec<PaletteItem> = Command::ALL
            .iter()
            .map(|c| PaletteItem {
                label: c.title().to_owned(),
                detail: c.keybinding().map(str::to_owned),
                action: PaletteAction::Command(*c),
                badge: None,
            })
            .collect();

        for ext in extensions {
            for cmd in &ext.contributed_commands {
                items.push(PaletteItem {
                    label: cmd.label(),
                    detail: Some(ext.display_name.clone()),
                    action: PaletteAction::ExtensionCommand {
                        id: cmd.id.clone(),
                        label: cmd.label(),
                        ext_dir: Some(ext.dir.clone()),
                        view_type: ext.view_type.clone(),
                    },
                    badge: None,
                });
            }
        }

        Self::build(PaletteMode::Commands, items)
    }

    pub fn language_modes() -> Self {
        let mut items = vec![PaletteItem {
            label: "Auto Detect".to_owned(),
            detail: Some("Auto detect from file extension".to_owned()),
            action: PaletteAction::SetLanguageMode(None),
            badge: Some(LanguageBadge::text("✨", 0x888888)),
        }];
        for entry in LANGUAGE_LIST {
            items.push(PaletteItem {
                label: format!("{} ({})", entry.name, entry.id),
                detail: Some(entry.exts.to_string()),
                action: PaletteAction::SetLanguageMode(Some(entry.id.to_string())),
                badge: Some(entry.badge),
            });
        }
        Self::build(PaletteMode::LanguageMode, items)
    }

    pub fn files(root: &Path) -> Self {
        Self::files_from(list_files(root))
    }

    fn file_items(list: Vec<(String, PathBuf)>) -> Vec<PaletteItem> {
        list.into_iter()
            .map(|(rel, abs)| PaletteItem {
                label: rel,
                detail: None,
                action: PaletteAction::OpenFile(abs),
                badge: None,
            })
            .collect()
    }

    pub fn files_from(list: Vec<(String, PathBuf)>) -> Self {
        Self::build(PaletteMode::Files, Self::file_items(list))
    }

    pub fn files_loading() -> Self {
        let mut p = Self::build(PaletteMode::Files, Vec::new());
        p.loading = true;
        p
    }

    pub fn set_files(&mut self, list: Vec<(String, PathBuf)>) {
        self.items = Self::file_items(list);
        self.loading = false;
        self.refilter();
    }

    fn build(mode: PaletteMode, items: Vec<PaletteItem>) -> Self {
        let mut p = Self {
            mode,
            field: TextField::default(),
            items,
            filtered: Vec::new(),
            selected: 0,
            error: None,
            loading: false,
        };
        p.refilter();
        p
    }

    pub fn branches(list: &[git::Branch]) -> Self {
        let mut items = vec![PaletteItem {
            label: "Create new branch...".to_owned(),
            detail: None,
            action: PaletteAction::NewBranchPrompt,
            badge: None,
        }];
        items.extend(list.iter().map(|b| PaletteItem {
            label: b.name.clone(),
            detail: b.current.then(|| "current".to_owned()),
            action: PaletteAction::Checkout(b.name.clone()),
            badge: None,
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
        if q.is_empty() {
            self.filtered = (0..self.items.len()).take(MAX_RESULTS).collect();
            self.selected = 0;
            return;
        }
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
            PaletteMode::LanguageMode => "Select Language Mode",
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
        let mut p = Palette::commands(&[]);
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
        let mut p = Palette::commands(&[]);
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
