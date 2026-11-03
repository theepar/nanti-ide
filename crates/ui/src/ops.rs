use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::extensions;
use crate::menu::MenuKind;
use crate::model::{
    ContextKind, ContextMenu, Model, PanelTab, Prompt, SaveOutcome, Tab, TabContent,
};
use crate::palette::{InputKind, Palette, PaletteMode};
use crate::problems::DEBOUNCE;
use crate::tasks;
use crate::workspace_ops as ops;

const ABOUT_PATH: &str = "nanti://about";

impl Model {
    pub fn say(&mut self, channel: &str, text: &str) {
        self.output.push(channel, text);
        self.message = Some(text.lines().next().unwrap_or_default().to_owned());
    }

    pub fn show_panel_tab(&mut self, tab: PanelTab) {
        self.layout.panel = true;
        self.panel_tab = tab;
        self.terminal_focused = false;
    }

    pub fn select_entry(&mut self, path: &Path) {
        self.explorer_sel = Some(path.to_path_buf());
    }

    pub fn selected_dir(&self) -> PathBuf {
        match &self.explorer_sel {
            Some(p) if p.is_dir() => p.clone(),
            Some(p) => p
                .parent()
                .map_or_else(|| self.root.clone(), Path::to_path_buf),
            None => self.root.clone(),
        }
    }

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
            kind: ContextKind::Item,
            path: path.to_path_buf(),
            is_dir,
            x,
            y,
        });
    }

    pub fn load_format_settings(&mut self) {
        self.format_on_save = self
            .config_dir
            .as_deref()
            .is_some_and(crate::format::load_format_on_save);
    }

    pub(crate) fn toggle_format_on_save(&mut self) {
        self.format_on_save = !self.format_on_save;
        if let Some(dir) = &self.config_dir {
            let _ = crate::format::save_format_on_save(dir, self.format_on_save);
        }
        let state = if self.format_on_save { "on" } else { "off" };
        self.say("Format", &format!("Format on save is {state}"));
    }

    /// Runs the file's formatter on the whole document and applies the result as one undo step.
    pub fn format_tab(&mut self, index: usize, announce: bool) -> bool {
        let Some(tab) = self.tabs.get(index) else {
            return false;
        };
        let TabContent::Text(editor) = &tab.content else {
            return false;
        };
        let (path, before) = (tab.path.clone(), editor.text());
        let dirs: Vec<PathBuf> = self.extensions.iter().map(|x| x.dir.clone()).collect();
        let Some(plan) = crate::format::plan(&path, &dirs) else {
            if announce {
                self.say("Format", "There is no formatter for this file type");
            }
            return false;
        };
        match crate::format::run(&plan, &path, &before) {
            Ok(out) => {
                let changed = match self.tabs.get_mut(index).map(|t| &mut t.content) {
                    Some(TabContent::Text(e)) => e.replace_document(&out),
                    _ => false,
                };
                if announce {
                    let text = if changed {
                        format!("Formatted with {}", plan.name)
                    } else {
                        "Already formatted".to_owned()
                    };
                    self.say("Format", &text);
                }
                changed
            }
            Err(msg) => {
                // A formatter that fails (syntax error, tool missing) must never stop a save.
                self.say("Format", &msg);
                false
            }
        }
    }

    pub(crate) fn begin_rename_symbol(&mut self) {
        let Some(e) = self.active_editor_mut() else {
            return;
        };
        if !e.has_selection() {
            let at = e.cursor();
            e.select_word_at(at);
        }
        let word = e.selected_text();
        if word.is_empty() || word.contains(char::is_whitespace) {
            return self.say("Rename", "Put the cursor on a name first");
        }
        self.open_input(InputKind::RenameSymbol(word.clone()), &word);
    }

    pub(crate) fn rename_symbol_in_file(&mut self, old: &str, new: &str) -> Result<(), String> {
        use editor_core::find::{FindOptions, Finder};
        let new = new.trim();
        if new.is_empty() || new == old {
            return Err("enter a different name".into());
        }
        if !new
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
        {
            return Err("a name may only have letters, digits, _ and $".into());
        }
        let finder = Finder::new(
            old,
            FindOptions {
                case_sensitive: true,
                whole_word: true,
                regex: false,
            },
        )
        .map_err(|e| e.0)?
        .ok_or("nothing to rename")?;
        let Some(e) = self.active_editor_mut() else {
            return Err("open a file first".into());
        };
        let n = e.replace_matches(&finder, new, None);
        self.say(
            "Rename",
            &format!("Renamed {n} whole-word match(es) of {old} in this file"),
        );
        Ok(())
    }

    pub fn open_editor_menu(&mut self, x: f32, y: f32) {
        self.menu_open = None;
        self.context_menu = Some(ContextMenu {
            kind: ContextKind::Editor,
            path: PathBuf::new(),
            is_dir: false,
            x,
            y,
        });
    }

    pub fn open_tab_menu(&mut self, index: usize, x: f32, y: f32) {
        self.menu_open = None;
        self.activate_tab(index);
        self.explorer_sel = None;
        self.context_menu = Some(ContextMenu {
            kind: ContextKind::Tab,
            path: PathBuf::new(),
            is_dir: false,
            x,
            y,
        });
    }

    pub fn collapse_all(&mut self) {
        self.expanded.clear();
        self.expanded.insert(self.root.clone());
    }

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

    pub fn open_input(&mut self, kind: InputKind, initial: &str) {
        self.context_menu = None;
        self.menu_open = None;
        self.palette = Some(Palette::input(kind, initial));
    }

    pub(crate) fn cmd_new(&mut self, folder: bool) {
        if !self.folder_open {
            return self.say("Explorer", "Open a folder first");
        }
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

    pub(crate) fn open_branch_picker(&mut self) {
        if self.scm.status.is_none() {
            return self.say("Git", "No Git repository");
        }
        self.context_menu = None;
        self.menu_open = None;
        self.palette = Some(Palette::branches(&self.scm.branches));
    }

    pub fn confirm_discard(&mut self, rel: &str, untracked: bool) {
        self.prompt = Some(Prompt::DiscardChanges {
            rel: rel.to_owned(),
            untracked,
        });
    }

    pub(crate) fn discard_changes(&mut self, rel: &str, untracked: bool) {
        if !untracked {
            return self.scm.discard(rel);
        }
        let Some(repo_root) = self.scm.repo_root().map(Path::to_path_buf) else {
            return;
        };
        self.delete_item(&repo_root.join(rel.trim_end_matches('/')));
    }

    pub(crate) fn reload_clean_tabs(&mut self) {
        for i in 0..self.tabs.len() {
            let tab = &self.tabs[i];
            let clean_text = matches!(&tab.content, TabContent::Text(e) if !e.is_dirty());
            if clean_text && crate::model::DiskStamp::of(&tab.path) != tab.stamp {
                self.reload_tab(i);
            }
        }
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
        if self.native_dialogs {
            self.pick_request = Some(crate::model::PickRequest::Vsix);
        } else {
            self.open_input(InputKind::InstallVsix, "");
        }
    }

    pub(crate) fn cmd_open_file(&mut self) {
        if self.native_dialogs {
            self.pick_request = Some(crate::model::PickRequest::File);
        }
    }

    pub(crate) fn cmd_open_folder(&mut self) {
        if self.native_dialogs {
            self.pick_request = Some(crate::model::PickRequest::Folder);
        }
    }

    pub fn apply_picked(&mut self, req: crate::model::PickRequest, path: PathBuf) {
        use crate::model::PickRequest;
        match req {
            PickRequest::File => {
                if path.is_file() {
                    self.open_file(&path);
                }
            }
            PickRequest::Folder => self.open_folder(path),
            PickRequest::Vsix => {
                let Some(dir) = self.config_dir.clone() else {
                    return;
                };
                let is_vsix = path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("vsix"));
                if !is_vsix {
                    return self.say("Extensions", "Choose a .vsix file");
                }
                match extensions::install_vsix(&dir, &path) {
                    Ok(ext) => self.after_extension_installed(&ext),
                    Err(e) => self.say("Extensions", &format!("Install failed: {e}")),
                }
            }
        }
    }

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
            InputKind::RenameSymbol(old) => self.rename_symbol_in_file(&old, text),
            InputKind::NewBranch => {
                let name = text.trim();
                if name.is_empty() {
                    return Err("enter a branch name".into());
                }
                self.scm.new_branch(name);
                Ok(())
            }
            InputKind::InstallVsix => {
                let dir = self.config_dir.clone().ok_or("no configuration folder")?;
                let path = PathBuf::from(text.trim().trim_matches('"'));
                let ext = extensions::install_vsix(&dir, &path)?;
                self.after_extension_installed(&ext);
                Ok(())
            }
        }
    }

    fn after_create(&mut self, path: &Path, folder: bool) {
        self.invalidate_explorer();
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
        self.invalidate_explorer();
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

    pub fn delete_item(&mut self, path: &Path) {
        match ops::send_to_recycle_bin(&self.root, path) {
            Ok(()) => {
                self.invalidate_explorer();
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

    pub fn go_to_line(&mut self, line: usize, col: usize) -> Result<(), String> {
        let e = self.active_editor_mut().ok_or("open a text file first")?;
        let last = e.buffer().len_lines().saturating_sub(1);
        let l = (line - 1).min(last);
        let start = e.buffer().line_start(l);
        let end = e.buffer().line_end(l);
        e.set_cursor((start + col - 1).min(end), false);
        Ok(())
    }

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

    pub(crate) fn open_about(&mut self) {
        let path = PathBuf::from(ABOUT_PATH);
        if let Some(i) = self.tabs.iter().position(|t| t.path == path) {
            self.active_tab = Some(i);
            return;
        }
        let text = format!(
            "nanti  version {}\n\nCode now. Not later.\nA lightweight code editor built natively in Rust.\n\n\
             Built with open-source software: GPUI (Apache-2.0), alacritty_terminal (Apache-2.0), tree-sitter (MIT), \
             ropey (MIT), wry (Apache-2.0 / MIT) and others. Every dependency, its license and its authors are listed in \
             THIRD_PARTY_NOTICES.md and in the README credits; their licenses are kept unmodified.\n\n\
             Third-party extensions (for example Claude Code or Codex) are not included; you install them yourself \
             under their own licenses. nanti is not affiliated with Microsoft, Zed Industries, Anthropic, OpenAI or Google.",
            env!("CARGO_PKG_VERSION")
        );
        self.tabs.push(Tab {
            path,
            title: "About nanti".to_owned(),
            content: TabContent::Unavailable(text),
            stamp: None,
            language_override: None,
        });
        self.active_tab = Some(self.tabs.len() - 1);
    }

    pub fn refresh_tasks(&mut self) {
        self.tasks = tasks::detect(&self.root);
    }

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

    pub fn refresh_extensions(&mut self) {
        self.extensions = match &self.config_dir {
            Some(dir) => extensions::list_installed(dir, self.panel_extension.as_deref()),
            None => Vec::new(),
        };
        if self.extensions.is_empty() {
            self.recommend_tried = false;
        }
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

    pub fn extension_title(&self) -> String {
        let Some(dir) = self.panel_extension.as_deref() else {
            return "Extension".to_owned();
        };
        let from_manifest = std::fs::read_to_string(dir.join("package.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| {
                v.pointer("/contributes/views")?
                    .as_object()?
                    .values()
                    .flat_map(|list| list.as_array().into_iter().flatten())
                    .filter_map(|view| view.get("name")?.as_str())
                    .find(|n| !n.trim().is_empty() && !n.starts_with('%'))
                    .map(str::to_owned)
            });
        from_manifest.unwrap_or_else(|| {
            self.extensions
                .iter()
                .find(|e| e.dir == dir)
                .map_or_else(|| "Extension".to_owned(), |e| e.display_name.clone())
        })
    }

    fn extension_vector_icon(&self) -> Option<PathBuf> {
        let dir = self.panel_extension.as_deref()?;
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("package.json")).ok()?).ok()?;
        let containers = manifest
            .pointer("/contributes/viewsContainers")?
            .as_object()?;
        containers
            .values()
            .flat_map(|list| list.as_array().into_iter().flatten())
            .filter_map(|c| c.get("icon")?.as_str())
            .filter(|rel| rel.to_ascii_lowercase().ends_with(".svg"))
            .map(|rel| dir.join(rel))
            .find(|p| p.starts_with(dir) && p.is_file())
    }

    pub fn open_extension_tab(&mut self) {
        self.panel_icon = self.extension_vector_icon();
        self.ext_detail = None;
        self.settings_open = false;
        let path = PathBuf::from("nanti://extension-panel");
        if let Some(i) = self.tabs.iter().position(|t| t.path == path) {
            self.tabs[i].title = self.extension_title();
            self.active_tab = Some(i);
            return;
        }
        let title = self.extension_title();
        self.tabs.push(Tab {
            path,
            title,
            content: TabContent::Webview,
            stamp: None,
            language_override: None,
        });
        self.active_tab = Some(self.tabs.len() - 1);
    }

    pub fn after_extension_installed(&mut self, ext: &extensions::Extension) {
        self.refresh_extensions();
        let note = match (&ext.view_type, &self.panel_extension) {
            (Some(view), None) => {
                self.use_as_panel_extension(&ext.dir, view);
                return;
            }
            (Some(_), Some(_)) => " (choose Use as extension panel to show it)",
            _ => "",
        };
        self.say(
            "Extensions",
            &format!("Installed {} {}{note}", ext.id, ext.version),
        );
    }

    pub fn use_as_panel_extension(&mut self, dir: &Path, view: &str) {
        let Some(cfg) = self.config_dir.clone() else {
            return;
        };
        match extensions::save_panel_choice(&cfg, dir, view) {
            Ok(()) => {
                self.panel_request = Some((dir.to_path_buf(), view.to_owned()));
                self.say("Extensions", "Opening the extension panel");
            }
            Err(e) => self.say("Extensions", &format!("Cannot save the choice: {e}")),
        }
    }

    pub fn search_marketplace(&mut self, query: &str) {
        let q = query.trim().to_owned();
        self.marketplace_message = None;
        if self.marketplace_recommended {
            self.marketplace_results.clear();
        }
        self.marketplace_recommended = false;
        if q.is_empty() {
            self.marketplace_results.clear();
            self.marketplace_searching = false;
            self.marketplace_query = None;
            self.marketplace_rx = None;
            self.recommend_tried = false;
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let asked = q.clone();
        std::thread::spawn(move || {
            let _ = tx.send(extensions::search_marketplace(&asked));
        });
        self.marketplace_rx = Some(rx);
        self.marketplace_query = Some(q);
        self.marketplace_searching = true;
    }

    fn market_ident(&self, download_url: &str) -> Option<(String, String)> {
        if let Some(m) = self
            .marketplace_results
            .iter()
            .find(|m| m.download_url == download_url)
        {
            return Some((m.publisher.clone(), m.name.clone()));
        }
        let d = self
            .ext_detail
            .as_ref()
            .filter(|d| d.download_url.as_deref() == Some(download_url))?;
        let name = d.id.strip_prefix(&format!("{}.", d.publisher))?;
        Some((d.publisher.clone(), name.to_owned()))
    }

    pub fn install_from_marketplace(&mut self, download_url: &str) {
        if self.installing.is_some() {
            return;
        }
        let Some(cfg) = self.config_dir.clone() else {
            self.say("Extensions", "Cannot install: no configuration directory");
            return;
        };
        let (publisher, name) = self.market_ident(download_url).unwrap_or_default();
        let (tx, rx) = std::sync::mpsc::channel();
        let url = download_url.to_owned();
        let progress = self.install_progress.clone();
        progress.done.store(0, std::sync::atomic::Ordering::Relaxed);
        progress
            .total
            .store(0, std::sync::atomic::Ordering::Relaxed);
        std::thread::spawn(move || {
            let _ = tx.send(extensions::download_and_install(
                &cfg, &publisher, &name, &url, &progress,
            ));
        });
        self.installing = Some(download_url.to_owned());
        self.install_rx = Some(rx);
        self.progress_seen = 0;
    }

    pub fn install_label(&self) -> String {
        use std::sync::atomic::Ordering;
        let (done, total) = (
            self.install_progress.done.load(Ordering::Relaxed),
            self.install_progress.total.load(Ordering::Relaxed),
        );
        if total > 0 && done >= total {
            "Unpacking...".to_owned()
        } else if let Some(percent) = (done * 100).checked_div(total) {
            format!("Installing {percent}%")
        } else if done > 0 {
            format!("Installing {} MB", done / 1_000_000)
        } else {
            "Installing...".to_owned()
        }
    }

    fn request_icons(&mut self) {
        let Some(cfg) = self.config_dir.clone() else {
            return;
        };
        let mut jobs: Vec<(String, String)> = Vec::new();
        for m in &self.marketplace_results {
            if self.ext_icons.contains_key(&m.id) || self.icons_requested.contains(&m.id) {
                continue;
            }
            if let Some(p) = extensions::cached_icon(&cfg, &m.id) {
                self.ext_icons.insert(m.id.clone(), p);
            } else if let Some(url) = &m.icon_url {
                self.icons_requested.insert(m.id.clone());
                jobs.push((m.id.clone(), url.clone()));
            }
        }
        if jobs.is_empty() {
            return;
        }
        let tx = self.icon_tx.clone();
        std::thread::spawn(move || {
            for batch in jobs.chunks(8) {
                for item in extensions::download_icons(&cfg, batch) {
                    let _ = tx.send(item);
                }
            }
        });
    }

    pub fn open_market_detail(&mut self, index: usize) {
        self.settings_open = false;
        use crate::extensions::Readme;
        let Some(m) = self.marketplace_results.get(index).cloned() else {
            return;
        };
        self.readme_rx = None;
        self.ext_detail_tab = crate::model::DetailTab::Details;
        {
            let (tx, rx) = std::sync::mpsc::channel();
            let (id, publisher, name) = (m.id.clone(), m.publisher.clone(), m.name.clone());
            std::thread::spawn(move || {
                let _ = tx.send((id, extensions::download_extra(&publisher, &name)));
            });
            self.readme_rx = Some(rx);
        }
        if self.marketplace_enabled
            && let Some(cfg) = self.config_dir.clone()
        {
            let (publisher, name, url) =
                (m.publisher.clone(), m.name.clone(), m.download_url.clone());
            std::thread::spawn(move || {
                let _ = extensions::prefetch_package(&cfg, &publisher, &name, &url);
            });
        }
        let readme = Readme::Loading;
        self.ext_detail = Some(extensions::ExtDetail {
            id: m.id,
            display_name: m.display_name,
            publisher: m.publisher,
            version: m.version,
            description: m.description,
            download_url: Some(m.download_url),
            downloads: Some(m.downloads),
            rating: m.rating,
            rating_count: m.rating_count,
            updated: m.updated,
            license: None,
            categories: Vec::new(),
            readme,
            manifest: crate::extensions::ManifestState::Loading,
        });
    }

    pub fn open_installed_detail(&mut self, index: usize) {
        self.settings_open = false;
        use crate::extensions::Readme;
        let Some(x) = self.extensions.get(index).cloned() else {
            return;
        };
        self.readme_rx = None;
        self.ext_detail_tab = crate::model::DetailTab::Details;
        let manifest = match extensions::read_local_manifest_info(&x.dir) {
            Some(info) => extensions::ManifestState::Ready(info),
            None => extensions::ManifestState::Failed("the package.json cannot be read".into()),
        };
        let readme = match extensions::read_local_readme(&x.dir) {
            Some(text) => Readme::Ready(text),
            None => Readme::None,
        };
        self.ext_detail = Some(extensions::ExtDetail {
            id: x.id,
            display_name: x.display_name,
            publisher: x.publisher,
            version: x.version,
            description: x.description,
            download_url: None,
            downloads: None,
            rating: None,
            rating_count: 0,
            updated: None,
            license: None,
            categories: Vec::new(),
            readme,
            manifest,
        });
    }

    pub fn close_settings(&mut self) {
        self.settings_open = false;
    }

    pub fn close_ext_detail(&mut self) {
        self.ext_detail = None;
        self.readme_rx = None;
    }

    pub fn on_ext_search_edited(&mut self) {
        let q = self.ext_query.text().trim().to_owned();
        if q.is_empty() {
            self.marketplace_query = None;
            self.marketplace_results.clear();
            self.marketplace_searching = false;
            self.marketplace_recommended = false;
            self.ext_search_debounce = None;
        } else {
            self.ext_search_debounce = Some(std::time::Instant::now());
        }
    }

    fn recommend_if_idle(&mut self) -> bool {
        if !self.marketplace_enabled
            || self.activity != crate::model::Activity::Extensions
            || self.recommend_tried
            || self.marketplace_searching
            || self.marketplace_query.is_some()
            || !self.marketplace_results.is_empty()
        {
            return false;
        }
        self.recommend_tried = true;
        let (tx, rx) = std::sync::mpsc::channel();
        let cfg = self.config_dir.clone();
        std::thread::spawn(move || {
            let _ = tx.send(extensions::recommended_cached(cfg.as_deref()));
        });
        self.marketplace_rx = Some(rx);
        self.marketplace_recommended = true;
        self.marketplace_searching = true;
        true
    }

    pub fn pump_marketplace(&mut self) -> bool {
        use std::sync::mpsc::TryRecvError;
        let mut changed = self.recommend_if_idle();
        if let Some(t) = self.ext_search_debounce
            && t.elapsed() >= std::time::Duration::from_millis(150)
        {
            self.ext_search_debounce = None;
            let q = self.ext_query.text().trim().to_owned();
            if !q.is_empty() && self.marketplace_query.as_deref() != Some(&q) {
                self.search_marketplace(&q);
                changed = true;
            }
        }
        if self.installing.is_some() {
            let now = self
                .install_progress
                .done
                .load(std::sync::atomic::Ordering::Relaxed);
            if now != self.progress_seen {
                self.progress_seen = now;
                changed = true;
            }
        }
        while let Ok((id, path)) = self.icon_rx.try_recv() {
            self.ext_icons.insert(id, path);
            changed = true;
        }
        if let Some(polled) = self.readme_rx.as_ref().map(|rx| rx.try_recv()) {
            use crate::extensions::Readme;
            let outcome = match polled {
                Ok((id, res)) => Some((id, res)),
                Err(TryRecvError::Disconnected) => self.ext_detail.as_ref().map(|d| {
                    (
                        d.id.clone(),
                        Err("the download stopped unexpectedly".to_owned()),
                    )
                }),
                Err(TryRecvError::Empty) => None,
            };
            if let Some((id, res)) = outcome {
                self.readme_rx = None;
                if let Some(d) = self.ext_detail.as_mut().filter(|d| d.id == id) {
                    match res {
                        Ok(extra) => {
                            d.readme = extra.readme.map_or(Readme::None, Readme::Ready);
                            d.license = extra.license;
                            d.categories = extra.categories;
                            d.manifest = match extra.manifest {
                                Some(info) => crate::extensions::ManifestState::Ready(info),
                                None => crate::extensions::ManifestState::Failed(
                                    "the package details could not be loaded".into(),
                                ),
                            };
                        }
                        Err(e) => {
                            d.manifest = crate::extensions::ManifestState::Failed(e.clone());
                            d.readme = Readme::Failed(e);
                        }
                    }
                }
                changed = true;
            }
        }
        if let Some(polled) = self.marketplace_rx.as_ref().map(|rx| rx.try_recv()) {
            match polled {
                Ok(res) => {
                    self.marketplace_rx = None;
                    self.marketplace_searching = false;
                    match res {
                        Ok(results) => {
                            self.marketplace_results = results;
                            self.request_icons();
                        }
                        Err(e) => self.marketplace_message = Some(e),
                    }
                    changed = true;
                }
                Err(TryRecvError::Disconnected) => {
                    self.marketplace_rx = None;
                    self.marketplace_searching = false;
                    self.marketplace_message = Some("The search stopped unexpectedly".into());
                    changed = true;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(polled) = self.install_rx.as_ref().map(|rx| rx.try_recv()) {
            let outcome = match polled {
                Ok(res) => Some(res),
                Err(TryRecvError::Disconnected) => {
                    Some(Err("the download stopped unexpectedly".to_owned()))
                }
                Err(TryRecvError::Empty) => None,
            };
            if let Some(res) = outcome {
                self.install_rx = None;
                self.installing = None;
                match res {
                    Ok(ext) => self.after_extension_installed(&ext),
                    Err(e) => self.say("Extensions", &format!("Cannot install: {e}")),
                }
                changed = true;
            }
        }
        changed
    }

    pub fn pump_problems(&mut self) -> bool {
        self.problems.refresh(&self.tabs, Instant::now(), DEBOUNCE)
    }

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

    pub(crate) fn run_discord_command(&mut self, command: crate::palette::Command) {
        if !crate::discord::AVAILABLE {
            return;
        }
        use crate::palette::Command as C;
        match command {
            C::ToggleDiscord => {
                self.discord.settings.enabled = !self.discord.settings.enabled;
            }
            C::DiscordConnect if !self.discord.settings.enabled => {
                self.discord.settings.enabled = true;
            }
            C::DiscordConnect => {
                self.discord.reconnect();
                return;
            }
            // nanti never holds a Discord login; logging out drops the link to the Discord app and clears the status.
            C::DiscordLogout => self.discord.settings.enabled = false,
            C::ToggleDiscordFile => {
                self.discord.settings.show_file = !self.discord.settings.show_file;
            }
            C::ToggleDiscordWorkspace => {
                self.discord.settings.show_workspace = !self.discord.settings.show_workspace;
            }
            C::ToggleDiscordTime => {
                self.discord.settings.show_time = !self.discord.settings.show_time;
            }
            _ => return,
        }
        self.discord_changed();
    }

    fn discord_changed(&mut self) {
        if !crate::discord::AVAILABLE {
            return;
        }
        if let Some(dir) = self.config_dir.clone()
            && let Err(e) = self.discord.settings.save(&dir)
        {
            self.say("Discord", &format!("Cannot save the Discord settings: {e}"));
        }
        self.discord.reconfigure();
    }

    pub fn load_discord_settings(&mut self) {
        if !crate::discord::AVAILABLE {
            return;
        }
        if let Some(dir) = self.config_dir.clone() {
            self.discord.settings = crate::discord::DiscordSettings::load(&dir);
            self.discord.reconfigure();
        }
    }

    fn discord_presence(&self) -> crate::discord::Presence {
        let s = &self.discord.settings;
        let tab = self
            .active()
            .filter(|t| matches!(t.content, TabContent::Text(_)));
        let details = match (tab, s.show_file) {
            (Some(t), true) => format!("Editing {}", t.title),
            _ => "Working in nanti.".to_owned(),
        };
        let mut parts: Vec<String> = Vec::new();
        if s.show_workspace && self.folder_open {
            let name = self.root.file_name().map_or_else(
                || self.root.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            );
            parts.push(format!("Workspace: {name}"));
        }
        if s.show_file
            && let Some(t) = tab
        {
            parts.push(crate::model::tab_language(t));
        }
        let small_image = tab.filter(|_| s.show_file).and_then(|t| {
            let ext = t.path.extension()?.to_str()?;
            let key = crate::discord::language_asset(ext)?;
            Some((key.to_owned(), crate::model::tab_language(t)))
        });
        crate::discord::Presence {
            details,
            state: (!parts.is_empty()).then(|| parts.join(" · ")),
            start_ms: s.show_time.then_some(self.discord.started_ms),
            small_image,
        }
    }

    pub fn pump_discord(&mut self) -> bool {
        if !crate::discord::AVAILABLE {
            return false;
        }
        let wanted = self
            .discord
            .settings
            .enabled
            .then(|| self.discord_presence());
        let mut changed = self.discord.pump(wanted);
        changed |= self.discord.pump_avatar(self.config_dir.as_deref());
        changed
    }

    pub fn palette_is_input(&self) -> bool {
        self.palette
            .as_ref()
            .is_some_and(|p| matches!(p.mode, PaletteMode::Input(_)))
    }
}
