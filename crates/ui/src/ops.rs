//! `Model` operations behind the menus, the explorer toolbar and the side views: file operations,
//! go to line, run tasks, extensions, output and problems. GUI-free; the UI only calls these.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::extensions;
use crate::menu::MenuKind;
use crate::model::{ContextMenu, Model, PanelTab, Prompt, SaveOutcome, Tab, TabContent};
use crate::palette::{InputKind, Palette, PaletteMode};
use crate::problems::DEBOUNCE;
use crate::tasks;
use crate::workspace_ops as ops;

/// Marker path of the About tab (never a real file).
const ABOUT_PATH: &str = "nanti://about";

impl Model {
    // ---- messages and the Output panel -----------------------------------------------------------

    /// Show `text` in the status bar and record it in the Output panel.
    pub fn say(&mut self, channel: &str, text: &str) {
        self.output.push(channel, text);
        self.message = Some(text.lines().next().unwrap_or_default().to_owned());
    }

    pub fn show_panel_tab(&mut self, tab: PanelTab) {
        self.layout.panel = true;
        self.panel_tab = tab;
        self.terminal_focused = false;
    }

    // ---- explorer selection and menus ------------------------------------------------------------

    pub fn select_entry(&mut self, path: &Path) {
        self.explorer_sel = Some(path.to_path_buf());
    }

    /// Folder new items go into: the selected folder, the folder of the selected file, or the root.
    pub fn selected_dir(&self) -> PathBuf {
        match &self.explorer_sel {
            Some(p) if p.is_dir() => p.clone(),
            Some(p) => p
                .parent()
                .map_or_else(|| self.root.clone(), Path::to_path_buf),
            None => self.root.clone(),
        }
    }

    /// What Rename / Delete / Copy Path act on: the selected explorer entry, else the active file.
    pub fn target_path(&self) -> Option<PathBuf> {
        if let Some(p) = &self.explorer_sel
            && p.exists()
        {
            return Some(p.clone());
        }
        self.active()
            .filter(|t| {
                matches!(t.content, TabContent::Text(_) | TabContent::Unavailable(_))
                    && t.path.exists()
            })
            .map(|t| t.path.clone())
    }

    pub fn open_menu(&mut self, kind: MenuKind) {
        self.context_menu = None;
        self.menu_open = Some(kind);
    }

    pub fn close_menus(&mut self) {
        self.menu_open = None;
        self.context_menu = None;
    }

    pub fn open_context_menu(&mut self, path: &Path, is_dir: bool, x: f32, y: f32) {
        self.menu_open = None;
        self.select_entry(path);
        self.context_menu = Some(ContextMenu {
            path: path.to_path_buf(),
            is_dir,
            x,
            y,
        });
    }

    pub fn collapse_all(&mut self) {
        self.expanded.clear();
        self.expanded.insert(self.root.clone());
    }

    /// Expand every folder above `path` so the file is visible in the explorer.
    pub fn reveal_in_explorer(&mut self, path: &Path) {
        let mut cur = path.parent();
        while let Some(dir) = cur {
            if !dir.starts_with(&self.root) {
                break;
            }
            self.expanded.insert(dir.to_path_buf());
            cur = dir.parent();
        }
    }

    // ---- prompts (palette input mode) ------------------------------------------------------------

    pub fn open_input(&mut self, kind: InputKind, initial: &str) {
        self.context_menu = None;
        self.menu_open = None;
        self.palette = Some(Palette::input(kind, initial));
    }

    pub(crate) fn cmd_new(&mut self, folder: bool) {
        let dir = self.selected_dir();
        let kind = if folder {
            InputKind::NewFolder(dir)
        } else {
            InputKind::NewFile(dir)
        };
        self.open_input(kind, "");
    }

    pub(crate) fn cmd_rename(&mut self) {
        let Some(path) = self.target_path() else {
            return self.say("Explorer", "Select a file or folder to rename first");
        };
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        self.open_input(InputKind::Rename(path), &name);
    }

    pub(crate) fn cmd_delete(&mut self) {
        let Some(path) = self.target_path() else {
            return self.say("Explorer", "Select a file or folder to delete first");
        };
        if path == self.root {
            return self.say(
                "Explorer",
                "The workspace folder itself cannot be deleted here",
            );
        }
        self.context_menu = None;
        self.prompt = Some(Prompt::DeleteItem(path));
    }

    pub(crate) fn cmd_copy_path(&mut self) {
        match self.target_path() {
            Some(p) => {
                self.clipboard_request = Some(p.display().to_string());
                self.say("Explorer", "Path copied to the clipboard");
            }
            None => self.say("Explorer", "Select a file or folder first"),
        }
    }

    pub(crate) fn cmd_reveal(&mut self) {
        let Some(p) = self.target_path() else {
            return self.say("Explorer", "Select a file or folder first");
        };
        #[cfg(windows)]
        {
            let mut arg = std::ffi::OsString::from("/select,");
            arg.push(&p);
            let _ = std::process::Command::new("explorer").arg(arg).spawn();
        }
        #[cfg(not(windows))]
        let _ = p;
    }

    pub(crate) fn cmd_go_to_line(&mut self) {
        if self.active_editor_mut().is_none() {
            return self.say("Editor", "Open a text file to go to a line");
        }
        self.open_input(InputKind::GoToLine, "");
    }

    pub(crate) fn cmd_install_vsix(&mut self) {
        if self.config_dir.is_none() {
            return self.say(
                "Extensions",
                "No configuration folder: cannot install extensions",
            );
        }
        self.show_activity(crate::model::Activity::Extensions);
        self.open_input(InputKind::InstallVsix, "");
    }

    /// Carry out what a prompt asked. `Err` keeps the prompt open and is shown under its field.
    pub fn submit_input(&mut self, kind: InputKind, text: &str) -> Result<(), String> {
        match kind {
            InputKind::GoToLine => {
                let mut parts = text.trim().splitn(2, [':', ',']);
                let line: usize = parts
                    .next()
                    .and_then(|l| l.trim().parse().ok())
                    .filter(|l| *l >= 1)
                    .ok_or("enter a line number (1 or higher)")?;
                let col: usize = match parts.next() {
                    Some(c) => c
                        .trim()
                        .parse()
                        .ok()
                        .filter(|c| *c >= 1)
                        .ok_or("the column must be a number (1 or higher)")?,
                    None => 1,
                };
                self.go_to_line(line, col)
            }
            InputKind::NewFile(dir) => {
                let path = ops::resolve_new(&self.root, &dir, text)?;
                ops::create_file(&path)?;
                self.after_create(&path, false);
                Ok(())
            }
            InputKind::NewFolder(dir) => {
                let path = ops::resolve_new(&self.root, &dir, text)?;
                ops::create_folder(&path)?;
                self.after_create(&path, true);
                Ok(())
            }
            InputKind::Rename(from) => {
                let to = ops::rename(&self.root, &from, text)?;
                self.after_rename(&from, &to);
                Ok(())
            }
            InputKind::InstallVsix => {
                let dir = self.config_dir.clone().ok_or("no configuration folder")?;
                let path = PathBuf::from(text.trim().trim_matches('"'));
                let ext = extensions::install_vsix(&dir, &path)?;
                self.refresh_extensions();
                self.say(
                    "Extensions",
                    &format!("Installed {} {}", ext.id, ext.version),
                );
                Ok(())
            }
        }
    }

    fn after_create(&mut self, path: &Path, folder: bool) {
        self.reveal_in_explorer(path);
        if folder {
            self.expanded.insert(path.to_path_buf());
        }
        self.select_entry(path);
        if !folder {
            self.open_file(path);
        }
        self.scm.refresh();
        let rel = path
            .strip_prefix(&self.root)
            .unwrap_or(path)
            .display()
            .to_string();
        self.say("Explorer", &format!("Created {rel}"));
    }

    fn after_rename(&mut self, from: &Path, to: &Path) {
        for tab in &mut self.tabs {
            if matches!(tab.content, TabContent::Diff(_)) {
                continue;
            }
            if let Ok(rest) = tab.path.strip_prefix(from) {
                let new_path = if rest.as_os_str().is_empty() {
                    to.to_path_buf()
                } else {
                    to.join(rest)
                };
                if tab.path == from {
                    tab.title = new_path
                        .file_name()
                        .map_or_else(|| tab.title.clone(), |n| n.to_string_lossy().into_owned());
                }
                tab.path = new_path;
                tab.stamp = crate::model::DiskStamp::of(&tab.path);
            }
        }
        if self.expanded.remove(from) {
            self.expanded.insert(to.to_path_buf());
        }
        self.select_entry(to);
        self.scm.refresh();
        let name = to
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        self.say("Explorer", &format!("Renamed to {name}"));
    }

    /// Move `path` to the Recycle Bin and close the tabs showing it.
    pub fn delete_item(&mut self, path: &Path) {
        match ops::send_to_recycle_bin(&self.root, path) {
            Ok(()) => {
                let mut i = 0;
                while i < self.tabs.len() {
                    let t = &self.tabs[i];
                    if !matches!(t.content, TabContent::Diff(_)) && t.path.starts_with(path) {
                        self.tabs.remove(i);
                    } else {
                        i += 1;
                    }
                }
                self.active_tab = if self.tabs.is_empty() {
                    None
                } else {
                    Some(self.active_tab.unwrap_or(0).min(self.tabs.len() - 1))
                };
                if self
                    .explorer_sel
                    .as_deref()
                    .is_some_and(|s| s.starts_with(path))
                {
                    self.explorer_sel = None;
                }
                self.scm.refresh();
                let name = path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                self.say("Explorer", &format!("Moved {name} to the Recycle Bin"));
            }
            Err(e) => self.say("Explorer", &format!("Delete failed: {e}")),
        }
    }

    /// Move the caret of the active editor to 1-based (line, col); out-of-range values are clamped.
    pub fn go_to_line(&mut self, line: usize, col: usize) -> Result<(), String> {
        let e = self.active_editor_mut().ok_or("open a text file first")?;
        let last = e.buffer().len_lines().saturating_sub(1);
        let l = (line - 1).min(last);
        let start = e.buffer().line_start(l);
        let end = e.buffer().line_end(l);
        e.set_cursor((start + col - 1).min(end), false);
        Ok(())
    }

    // ---- tabs --------------------------------------------------------------------------------------

    pub(crate) fn save_all(&mut self) {
        let (mut saved, mut skipped) = (0, 0);
        for i in 0..self.tabs.len() {
            match self.save_tab(i, false) {
                SaveOutcome::Saved => saved += 1,
                SaveOutcome::Conflict(_) | SaveOutcome::Failed(_) => skipped += 1,
                SaveOutcome::NothingToSave => {}
            }
        }
        let text = match (saved, skipped) {
            (0, 0) => "Nothing to save".to_owned(),
            (s, 0) => format!("Saved {s} file{}", if s == 1 { "" } else { "s" }),
            (s, k) => {
                format!("Saved {s}; {k} could not be saved (changed on disk or write failed)")
            }
        };
        self.say("Files", &text);
    }

    /// Close every tab without unsaved changes; tabs with changes stay open and are reported.
    pub(crate) fn close_all_tabs(&mut self) {
        let before = self.tabs.len();
        self.tabs.retain(Tab::is_dirty);
        self.active_tab = if self.tabs.is_empty() { None } else { Some(0) };
        let left = self.tabs.len();
        if left > 0 {
            self.say(
                "Files",
                &format!(
                    "Closed {} tab(s); {left} with unsaved changes stay open",
                    before - left
                ),
            );
        }
    }

    // ---- About -------------------------------------------------------------------------------------

    pub(crate) fn open_about(&mut self) {
        let path = PathBuf::from(ABOUT_PATH);
        if let Some(i) = self.tabs.iter().position(|t| t.path == path) {
            self.active_tab = Some(i);
            return;
        }
        let text = format!(
            "nanti.  version {}\n\nCode now. Not later.\nA lightweight code editor built natively in Rust.\n\n\
             Built with open-source software: GPUI (Apache-2.0), alacritty_terminal (Apache-2.0), tree-sitter (MIT), \
             ropey (MIT), wry (Apache-2.0 / MIT) and others. Every dependency, its license and its authors are listed in \
             THIRD_PARTY_NOTICES.md and in the README credits; their licenses are kept unmodified.\n\n\
             Third-party extensions (for example Claude Code or Codex) are not included; you install them yourself \
             under their own licenses. nanti. is not affiliated with Microsoft, Zed Industries, Anthropic, OpenAI or Google.",
            env!("CARGO_PKG_VERSION")
        );
        self.tabs.push(Tab {
            path,
            title: "About nanti.".to_owned(),
            content: TabContent::Unavailable(text),
            stamp: None,
        });
        self.active_tab = Some(self.tabs.len() - 1);
    }

    // ---- Run view ----------------------------------------------------------------------------------

    pub fn refresh_tasks(&mut self) {
        self.tasks = tasks::detect(&self.root);
    }

    /// Run task `index` in the integrated terminal (the UI layer types it).
    pub fn run_task(&mut self, index: usize) {
        let Some(task) = self.tasks.get(index) else {
            return;
        };
        let command = task.command.clone();
        self.output.push("Run", &format!("$ {command}"));
        self.message = Some(format!("Running {command}"));
        self.show_terminal_panel();
        self.terminal_run = Some(command);
    }

    // ---- Extensions view -----------------------------------------------------------------------------

    pub fn refresh_extensions(&mut self) {
        self.extensions = match &self.config_dir {
            Some(dir) => extensions::list_installed(dir, self.panel_extension.as_deref()),
            None => Vec::new(),
        };
    }

    pub fn confirm_remove_extension(&mut self, dir: &Path) {
        self.prompt = Some(Prompt::RemoveExtension(dir.to_path_buf()));
    }

    pub fn remove_extension(&mut self, dir: &Path) {
        let Some(cfg) = self.config_dir.clone() else {
            return;
        };
        match extensions::uninstall(&cfg, dir) {
            Ok(()) => {
                self.refresh_extensions();
                self.say("Extensions", "Extension removed");
            }
            Err(e) => self.say("Extensions", &format!("Cannot remove: {e}")),
        }
    }

    /// Make `dir` fill the right-hand panel from the next start.
    pub fn use_as_panel_extension(&mut self, dir: &Path, view: &str) {
        let Some(cfg) = self.config_dir.clone() else {
            return;
        };
        match extensions::save_panel_choice(&cfg, dir, view) {
            Ok(()) => self.say(
                "Extensions",
                "Saved: it will fill the right panel the next time nanti. starts",
            ),
            Err(e) => self.say("Extensions", &format!("Cannot save the choice: {e}")),
        }
    }

    // ---- analysis ----------------------------------------------------------------------------------

    /// Re-check the open files for syntax problems (debounced). True if the list changed.
    pub fn pump_problems(&mut self) -> bool {
        self.problems.refresh(&self.tabs, Instant::now(), DEBOUNCE)
    }

    /// Symbols of the active file (cached per text version).
    pub fn outline(&self) -> Vec<syntax::Symbol> {
        let Some(tab) = self.active() else {
            return Vec::new();
        };
        let TabContent::Text(e) = &tab.content else {
            return Vec::new();
        };
        let Some(lang) = syntax::Language::from_path(&tab.path) else {
            return Vec::new();
        };
        let version = e.buffer().version();
        if let Some((p, v, symbols)) = &*self.outline_cache.borrow()
            && *p == tab.path
            && *v == version
        {
            return symbols.clone();
        }
        let symbols = syntax::outline(lang, &e.text());
        *self.outline_cache.borrow_mut() = Some((tab.path.clone(), version, symbols.clone()));
        symbols
    }

    /// Open the input prompt for the palette's `PaletteMode` (re-exported for completeness).
    pub fn palette_is_input(&self) -> bool {
        self.palette
            .as_ref()
            .is_some_and(|p| matches!(p.mode, PaletteMode::Input(_)))
    }
}
