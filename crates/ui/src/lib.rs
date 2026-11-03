use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;

use editor_core::Editor;
use gpui::{
    AnyElement, Bounds, ClickEvent, ClipboardItem, Context, ElementInputHandler,
    EntityInputHandler, FocusHandle, FontWeight, HighlightStyle, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Render, ScrollDelta,
    ScrollHandle, ScrollWheelEvent, ShapedLine, SharedString, StyledText, TextRun, UTF16Selection,
    Window, canvas, div, font, point, prelude::*, px, relative, rgb, rgba, size,
};
use syntax::{Language, LineSpan, SyntaxHighlighter, Token};
use terminal_ui::{GridMetrics, TerminalSession};

pub mod assets;
pub mod discord;
pub mod emmet;
pub mod extensions;
pub mod find;
pub mod format;
pub mod launch;
pub mod layout;
pub mod lsp;
pub mod menu;
pub mod model;
mod ops;
pub mod output;
pub mod palette;
pub mod panel;
pub mod problems;
pub mod resource;
pub mod scm;
pub mod search;
pub mod tasks;
mod terminal_ui;
mod views;
pub mod workspace_ops;

use find::FindFocus;
use layout::{Layout, PanelPos, Side};
use model::{Activity, Model, PanelTab, Prompt, SaveOutcome, Tab, TabContent};
use palette::{Command, TextField};

const PALETTE_ROWS: usize = 12;
const PALETTE_INPUT_PAD: f32 = 12.0;
const FIELD_PAD: f32 = 8.0;
use panel::{Panel, UiEvent};

const STATUS_BAR_HEIGHT: f32 = 22.0;
const MENU_BAR_HEIGHT: f32 = 30.0;
const SPLITTER: f32 = 1.0;
const SPLITTER_GRAB: f32 = 4.0;
const ACTIVITY_BAR_WIDTH: f32 = 48.0;
const TAB_BAR_HEIGHT: f32 = 35.0;
const MAX_RENDERED_LINES: usize = 20_000;
const DIFF_NOTE_H: f32 = 24.0;
const EDITOR_MAX_LINES: usize = 100_000;
pub(crate) const OVERSCAN_ROWS: usize = 10;
const EXPLORER_ROW_H: f32 = 22.0;
pub(crate) const SCM_ROW_H: f32 = 22.0;
pub(crate) const OUTPUT_ROW_H: f32 = 20.0;
const SCM_HEADER_H: f32 = 24.0;
const COMMIT_ROW_H: f32 = 46.0;
const HISTORY_HEADER_H: f32 = 28.0;
const EXPLORER_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(2);
const LINE_HEIGHT: f32 = 20.0;
const GUTTER_WIDTH: f32 = 56.0;
const FONT_SIZE: f32 = 14.0;
const FONT_FAMILY: &str = "Consolas";
const EDITOR_PAD_Y: f32 = 8.0;
const TAB_WIDTH: usize = 4;

pub mod theme {
    pub const SCROLL_TRACK: u32 = 0x1e1e1e;
    pub const SCROLL_THUMB: u32 = 0x424242;
    pub const ACTIVITY_BAR: u32 = 0x181818;
    pub const SIDEBAR: u32 = 0x1e1e1e;
    pub const EDITOR: u32 = 0x1e1e1e;
    pub const TAB_BAR: u32 = 0x181818;
    pub const PANEL: u32 = 0x181818;
    pub const STATUS_BAR: u32 = 0x181818;
    pub const BORDER: u32 = 0x2b2b2b;
    pub const TEXT: u32 = 0xcccccc;
    pub const TEXT_DIM: u32 = 0x858585;
    pub const ICON: u32 = 0xb4b4b4;
    pub const ACCENT: u32 = 0xe6e6e6;
    pub const LINE_NUMBER: u32 = 0x6e7681;
    pub const LINE_NUMBER_ACTIVE: u32 = 0xc6c6c6;
    pub const HOVER: u32 = 0x2a2d2e;
    pub const SELECTED: u32 = 0x37373d;
    pub const HOVER_STRONG: u32 = 0x3c3f41;
    pub const SELECTION: u32 = 0x264f7880;
    pub const BANNER: u32 = 0x2b2a1f;
    pub const WARN: u32 = 0xe5c07b;
    pub const FIND_MATCH: u32 = 0x515c6b66;
    pub const FIND_CURRENT: u32 = 0xe5c07b99;
    pub const MENU_BG: u32 = 0x252526;

    pub fn token_color(t: syntax::Token) -> u32 {
        use syntax::Token::*;
        match t {
            Keyword => 0xc586c0,
            String => 0xce9178,
            Comment => 0x6a9955,
            Number => 0xb5cea8,
            Constant => 0x569cd6,
            Type => 0x4ec9b0,
            Function => 0xdcdcaa,
            Attribute => 0x9cdcfe,
            Property => 0x9cdcfe,
            Operator => 0xd4d4d4,
            Tag => 0x569cd6,
            Heading => 0x569cd6,
            Code => 0xce9178,
            Link => 0x3794ff,
        }
    }
}

pub fn tone_color(t: scm::Tone) -> u32 {
    match t {
        scm::Tone::Modified => 0xe2c08d,
        scm::Tone::Added => 0x81b88b,
        scm::Tone::Removed => 0xc74e39,
        scm::Tone::Conflict => 0xe5534b,
    }
}

fn activity_icon(a: Activity) -> &'static str {
    match a {
        Activity::Explorer => "files",
        Activity::Search => "search",
        Activity::SourceControl => "git",
        Activity::Run => "run",
        Activity::Extensions => "extensions",
    }
}

struct RowWindow {
    rows: Vec<AnyElement>,
    y: f32,
    gap: f32,
    lo: f32,
    hi: f32,
}

impl RowWindow {
    fn new(lo: f32, hi: f32) -> Self {
        Self {
            rows: Vec::new(),
            y: 0.0,
            gap: 0.0,
            lo,
            hi,
        }
    }

    fn flush(&mut self) {
        if self.gap > 0.0 {
            self.rows
                .push(div().h(px(self.gap)).flex_none().into_any_element());
            self.gap = 0.0;
        }
    }

    fn push(&mut self, h: f32, make: impl FnOnce() -> AnyElement) {
        if self.y + h < self.lo || self.y > self.hi {
            self.gap += h;
        } else {
            self.flush();
            self.rows.push(make());
        }
        self.y += h;
    }

    fn finish(mut self) -> Vec<AnyElement> {
        self.flush();
        self.rows
    }
}

fn display_text(s: &str) -> String {
    s.replace('\t', &" ".repeat(TAB_WIDTH))
}

fn shape(window: &Window, text: String) -> ShapedLine {
    let run = TextRun {
        len: text.len(),
        font: font(FONT_FAMILY),
        color: gpui::black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(SharedString::from(text), px(FONT_SIZE), &[run], None)
}

fn x_of_col(window: &Window, line: &str, col: usize) -> Pixels {
    let prefix: String = line.chars().take(col).collect();
    let shown = display_text(&prefix);
    if shown.is_empty() {
        return px(0.);
    }
    shape(window, shown.clone()).x_for_index(shown.len())
}

fn col_at_x(window: &Window, line: &str, x: Pixels) -> usize {
    let shown = display_text(line);
    if shown.is_empty() {
        return 0;
    }
    let idx = shape(window, shown).closest_index_for_x(x);
    let mut disp = 0;
    for (col, c) in line.chars().enumerate() {
        let w = if c == '\t' { TAB_WIDTH } else { c.len_utf8() };
        if disp + w > idx {
            return col;
        }
        disp += w;
    }
    line.chars().count()
}

fn line_element(text: &str, spans: Option<&[LineSpan]>) -> AnyElement {
    let shown = SharedString::from(display_text(text));
    let Some(spans) = spans.filter(|s| !s.is_empty()) else {
        return div().child(shown).into_any_element();
    };
    let shift = |b: usize| b + 3 * text[..b.min(text.len())].matches('\t').count();
    let mut highlights: Vec<(Range<usize>, HighlightStyle)> = spans
        .iter()
        .filter(|s| {
            s.range.end <= text.len()
                && text.is_char_boundary(s.range.start)
                && text.is_char_boundary(s.range.end)
        })
        .map(|s| {
            let style = HighlightStyle {
                color: Some(rgb(theme::token_color(s.token)).into()),
                font_weight: (s.token == Token::Heading).then_some(FontWeight::BOLD),
                ..Default::default()
            };
            (shift(s.range.start)..shift(s.range.end), style)
        })
        .collect();
    highlights.sort_by_key(|(r, _)| r.start);
    StyledText::new(shown)
        .with_highlights(highlights)
        .into_any_element()
}

fn char_range_to_bytes(s: &str, r: Range<usize>) -> Range<usize> {
    let byte = |c: usize| s.char_indices().nth(c).map_or(s.len(), |(b, _)| b);
    byte(r.start)..byte(r.end)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Drag {
    Sidebar,
    Panel,
    PanelSide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Editor,
    Palette,
    FindQuery,
    FindReplace,
    Search,
    Commit,
    Terminal,
    ExtSearch,
}

#[derive(Default)]
struct Scrolls {
    explorer: ScrollHandle,
    outline: ScrollHandle,
    scm: ScrollHandle,
    tasks: ScrollHandle,
    ext: ScrollHandle,
    ext_detail: ScrollHandle,
    settings: ScrollHandle,
    problems: ScrollHandle,
    output: ScrollHandle,
    search: ScrollHandle,
}

type Spans = Rc<Vec<Vec<LineSpan>>>;
type HlCache = (PathBuf, u64, Option<Spans>);
type HlResult = (PathBuf, u64, Option<Vec<Vec<LineSpan>>>);

/// Files above this size are highlighted on a background thread so typing never waits for them.
const HIGHLIGHT_SYNC_BYTES: usize = 60_000;

struct HlJob {
    rx: std::sync::mpsc::Receiver<HlResult>,
}

#[derive(Default)]
struct HlState {
    exact: Option<HlCache>,
    last_good: Option<(PathBuf, Spans)>,
    provisional: Option<HlCache>,
    job: Option<HlJob>,
    ready: Option<HlResult>,
}

fn adjust_spans(old: &[Vec<LineSpan>], new_lines: usize, cursor_line: usize) -> Vec<Vec<LineSpan>> {
    let mut v = old.to_vec();
    if new_lines > v.len() {
        let at = cursor_line.min(v.len());
        for _ in 0..new_lines - v.len() {
            v.insert(at, Vec::new());
        }
    } else if new_lines < v.len() {
        let at = (cursor_line + 1).min(v.len());
        let remove = (v.len() - new_lines).min(v.len() - at);
        v.drain(at..at + remove);
        v.resize(new_lines, Vec::new());
    }
    v
}
type ExplorerCache = (std::time::Instant, u64, Rc<Vec<model::Entry>>);

pub struct Workbench {
    model: Model,
    panel: Option<Panel>,
    status_items: BTreeMap<String, String>,
    external_prompt: Option<(String, String)>,
    focus: FocusHandle,
    palette_focus: FocusHandle,
    palette_top: std::cell::Cell<usize>,
    palette_follow: RefCell<(usize, String)>,
    root_focus: FocusHandle,
    find_query_focus: FocusHandle,
    find_replace_focus: FocusHandle,
    search_focus: FocusHandle,
    commit_focus: FocusHandle,
    terminal_focus: FocusHandle,
    ext_search_focus: FocusHandle,
    drag: Option<Drag>,
    layout_path: Option<PathBuf>,
    layout_saved: Layout,
    webview_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    webview_applied: Option<(i32, i32, i32, i32)>,
    commit_card: Option<views::commit_tip::CardAnchor>,
    commit_tip: Option<gpui::Entity<views::commit_tip::CommitTip>>,
    card_gen: u64,
    hovered_commit: Option<String>,
    card_hovered: bool,
    card_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    terminals: Vec<TerminalSession>,
    active_terminal: usize,
    term_counter: usize,
    term_metrics: GridMetrics,
    highlighter: std::sync::Arc<std::sync::Mutex<SyntaxHighlighter>>,
    hl: RefCell<HlState>,
    explorer_cache: RefCell<Option<ExplorerCache>>,
    scroll: ScrollHandle,
    scrolls: Scrolls,
    scroll_drag: Option<ScrollHandle>,
    outline_open: bool,
    dragging: bool,
    quitting: bool,
    last_synced_versions: HashMap<PathBuf, u64>,
    lsp: lsp::LspManager,
}

impl Workbench {
    pub fn new(root: PathBuf, cx: &mut Context<Self>) -> Self {
        Self::with_model(Model::new(root), cx)
    }

    pub fn new_empty(cx: &mut Context<Self>) -> Self {
        Self::with_model(Model::without_folder(), cx)
    }

    fn with_model(model: Model, cx: &mut Context<Self>) -> Self {
        Self {
            model,
            panel: None,
            status_items: BTreeMap::new(),
            external_prompt: None,
            focus: cx.focus_handle(),
            palette_focus: cx.focus_handle(),
            palette_top: std::cell::Cell::new(0),
            palette_follow: RefCell::new((0, String::new())),
            root_focus: cx.focus_handle(),
            find_query_focus: cx.focus_handle(),
            find_replace_focus: cx.focus_handle(),
            search_focus: cx.focus_handle(),
            commit_focus: cx.focus_handle(),
            terminal_focus: cx.focus_handle(),
            ext_search_focus: cx.focus_handle(),
            drag: None,
            layout_path: None,
            layout_saved: Layout::default(),
            webview_bounds: Rc::new(Cell::new(None)),
            webview_applied: Some((-1, -1, 0, 0)),
            commit_card: None,
            commit_tip: None,
            card_gen: 0,
            hovered_commit: None,
            card_hovered: false,
            card_bounds: Rc::new(Cell::new(None)),
            terminals: Vec::new(),
            active_terminal: 0,
            term_counter: 0,
            term_metrics: GridMetrics::default(),
            highlighter: std::sync::Arc::new(std::sync::Mutex::new(SyntaxHighlighter::new())),
            hl: RefCell::new(HlState::default()),
            explorer_cache: RefCell::new(None),
            scroll: ScrollHandle::new(),
            scrolls: Scrolls::default(),
            scroll_drag: None,
            outline_open: true,
            dragging: false,
            quitting: false,
            last_synced_versions: HashMap::new(),
            lsp: lsp::LspManager::new(),
        }
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn model_mut(&mut self) -> &mut Model {
        &mut self.model
    }

    pub fn open_path(&mut self, path: &std::path::Path) {
        self.model.open_file(path);
    }

    pub fn editor_focus(&self) -> &FocusHandle {
        &self.focus
    }

    pub fn set_config_dir(&mut self, dir: PathBuf, panel_extension: Option<PathBuf>) {
        self.model.config_dir = Some(dir);
        self.model.load_discord_settings();
        self.model.load_format_settings();
        self.model.panel_extension = panel_extension;
        self.model.refresh_extensions();
    }

    pub fn enable_source_control(&mut self) {
        self.model.enable_source_control();
    }

    pub fn enable_marketplace(&mut self) {
        self.model.marketplace_enabled = true;
    }

    #[cfg(windows)]
    fn native_pick(mode: &str, title: &str) -> Option<PathBuf> {
        static PICKER_BYTES: &[u8] = include_bytes!("../res/nanti-picker.exe");
        let temp_picker = std::env::temp_dir().join("nanti-picker.exe");
        if !temp_picker.exists()
            || temp_picker.metadata().map(|m| m.len()).unwrap_or(0) != PICKER_BYTES.len() as u64
        {
            let _ = std::fs::write(&temp_picker, PICKER_BYTES);
        }
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let output = std::process::Command::new(&temp_picker)
            .args([mode, title])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        if output.status.success() && !output.stdout.is_empty() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(PathBuf::from(s));
            }
        }
        None
    }

    pub fn enable_native_dialogs(&mut self) {
        self.model.native_dialogs = true;
    }

    fn start_picker(
        &mut self,
        req: crate::model::PickRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::model::PickRequest;
        let (mode, title) = match req {
            PickRequest::File => ("file", "Open File"),
            PickRequest::Folder => ("folder", "Select Workspace Folder"),
            PickRequest::Vsix => ("vsix", "Install Extension from VSIX"),
        };
        #[cfg(windows)]
        {
            let mode_s = mode.to_string();
            let title_s = title.to_string();
            cx.spawn_in(window, async move |this, cx| {
                let chosen = cx
                    .background_executor()
                    .spawn(async move { Self::native_pick(&mode_s, &title_s) })
                    .await;
                let Some(path) = chosen else {
                    return;
                };
                this.update_in(cx, |wb, window, cx| {
                    wb.model.apply_picked(req, path);
                    if req == PickRequest::File {
                        wb.focus_editor(window);
                    }
                    wb.sync_focus(window);
                    wb.reveal_cursor();
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }
        #[cfg(not(windows))]
        {
            let (files, directories, prompt) = match req {
                PickRequest::File => (true, false, None),
                PickRequest::Folder => (false, true, Some("Select Folder")),
                PickRequest::Vsix => (true, false, Some("Install")),
            };
            let chosen = cx.prompt_for_paths(gpui::PathPromptOptions {
                files,
                directories,
                multiple: false,
                prompt: prompt.map(SharedString::new_static),
            });
            cx.spawn_in(window, async move |this, cx| {
                let Ok(Ok(Some(paths))) = chosen.await else {
                    return;
                };
                let Some(path) = paths.into_iter().next() else {
                    return;
                };
                this.update_in(cx, |wb, window, cx| {
                    wb.model.apply_picked(req, path);
                    if req == PickRequest::File {
                        wb.focus_editor(window);
                    }
                    wb.sync_focus(window);
                    wb.reveal_cursor();
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }
    }

    pub fn set_layout_path(&mut self, path: PathBuf) {
        self.model.layout = Layout::load(&path);
        self.model.layout.panel = false;
        self.model.layout.panel_maximized = false;
        if !self.model.folder_open {
            self.model.layout.sidebar = false;
        }
        self.layout_saved = self.model.layout.clone();
        self.layout_path = Some(path);
    }

    pub fn save_layout_now(&mut self) {
        let Some(path) = self.layout_path.clone() else {
            return;
        };
        if self.model.layout == self.layout_saved {
            return;
        }
        match self.model.layout.save(&path) {
            Ok(()) => self.layout_saved = self.model.layout.clone(),
            Err(e) => {
                self.layout_saved = self.model.layout.clone(); // do not retry every tick
                self.model.message = Some(format!("Could not save layout: {e}"));
            }
        }
    }

    pub fn attach_panel(&mut self, panel: Panel) {
        self.panel = Some(panel);
        self.model.panel_running = true;
        if self.model.panel_extension.is_some() {
            self.model.open_extension_tab();
        }
    }

    pub fn pump_ui(&mut self, window: &mut Window) -> bool {
        let changed = self.pump();
        if self.model.panel_request.is_some() {
            self.activate_panel(window);
            return true;
        }
        changed
    }

    fn activate_panel(&mut self, window: &mut Window) {
        let Some((dir, view)) = self.model.panel_request.take() else {
            return;
        };
        let workspace = self.model.folder_open.then(|| self.model.root.clone());
        let cfg = launch::panel_host_config(dir.clone(), view, workspace);
        let diag_log = cfg.selftest_log.clone();
        let (events, to_host, _shutdown) = panel::spawn_worker(cfg);
        match Panel::new(window, events, to_host, diag_log) {
            Ok(panel) => {
                self.panel = Some(panel);
                self.model.panel_running = true;
                self.webview_applied = Some((-1, -1, 0, 0));
                self.status_items.clear();
                self.model.panel_extension = Some(dir);
                self.model.refresh_extensions();
                self.model.open_extension_tab();
            }
            Err(e) => self.model.say(
                "Extensions",
                &format!("Cannot open the extension panel: {e}"),
            ),
        }
    }

    pub fn allow_window_close(&mut self) -> bool {
        let ok = self.quitting || self.model.request_quit();
        if ok {
            self.kill_all_terminals(); // never leave shells running behind a closed window
            self.lsp.shutdown_all();
        }
        ok
    }

    pub fn pump(&mut self) -> bool {
        if self.drag.is_none() {
            self.save_layout_now();
        }
        let mut changed = self.pump_terminals();
        changed |= self.model.pump_scm();
        changed |= self.model.pump_search();
        changed |= self.model.pump_index();
        changed |= self.pump_highlight();
        changed |= self.model.pump_problems();
        changed |= self.model.pump_marketplace();
        changed |= self.model.pump_discord();

        if let Some(tab) = self.model.active()
            && let TabContent::Text(e) = &tab.content
        {
            let v = e.version();
            let last = self
                .last_synced_versions
                .get(&tab.path)
                .copied()
                .unwrap_or(0);
            if v != last {
                self.last_synced_versions.insert(tab.path.clone(), v);
                let uri = format!("file:///{}", tab.path.to_string_lossy().replace('\\', "/"));
                self.model
                    .doc_sync_events
                    .push(crate::model::DocSyncEvent::Change {
                        uri,
                        version: v,
                        text: e.text(),
                    });
            }
        }
        if let Some(tab) = self.model.active()
            && let Some(lang) = Language::from_path(&tab.path)
            && matches!(lang, Language::Rust | Language::Go)
            && self.model.folder_open
            && let Some(server) = self.lsp.ensure_server(lang, &self.model.root)
            && !server.opened.contains(&tab.path)
            && let TabContent::Text(e) = &tab.content
        {
            server.did_open(&tab.path, lang, e.version(), &e.text());
        }
        let sync_events: Vec<crate::model::DocSyncEvent> =
            std::mem::take(&mut self.model.doc_sync_events);
        for ev in &sync_events {
            if let Some(panel) = self.panel.as_ref() {
                match ev {
                    crate::model::DocSyncEvent::Open {
                        uri,
                        language_id,
                        version,
                        text,
                    } => {
                        panel.doc_open(uri.clone(), language_id.clone(), *version, text.clone());
                    }
                    crate::model::DocSyncEvent::Change { uri, version, text } => {
                        panel.doc_change(uri.clone(), *version, text.clone());
                    }
                    crate::model::DocSyncEvent::Save { uri } => {
                        panel.doc_save(uri.clone());
                    }
                    crate::model::DocSyncEvent::Close { uri } => {
                        panel.doc_close(uri.clone());
                    }
                    crate::model::DocSyncEvent::Reset => {}
                }
            }
            match ev {
                crate::model::DocSyncEvent::Open {
                    uri, version, text, ..
                } => {
                    if let Some(path) = crate::lsp::uri_to_path(uri)
                        && let Some(lang) = Language::from_path(&path)
                        && self.model.folder_open
                        && let Some(server) = self.lsp.ensure_server(lang, &self.model.root)
                    {
                        server.did_open(&path, lang, *version, text);
                    }
                }
                crate::model::DocSyncEvent::Change { uri, version, text } => {
                    if let Some(path) = crate::lsp::uri_to_path(uri)
                        && let Some(lang) = Language::from_path(&path)
                        && let Some(server) = self.lsp.get_mut(lang)
                    {
                        server.did_change(&path, *version, text);
                    }
                }
                crate::model::DocSyncEvent::Save { uri } => {
                    if let Some(path) = crate::lsp::uri_to_path(uri)
                        && let Some(lang) = Language::from_path(&path)
                        && let Some(server) = self.lsp.get_mut(lang)
                    {
                        server.did_save(&path);
                    }
                }
                crate::model::DocSyncEvent::Close { uri } => {
                    if let Some(path) = crate::lsp::uri_to_path(uri)
                        && let Some(lang) = Language::from_path(&path)
                    {
                        // Never start a server just to close a document.
                        if let Some(server) = self.lsp.get_mut(lang) {
                            server.did_close(&path);
                        }
                    }
                }
                crate::model::DocSyncEvent::Reset => {
                    self.lsp.shutdown_all();
                    self.last_synced_versions.clear();
                }
            }
        }
        for ev in self.lsp.pump() {
            match ev {
                crate::lsp::LspEvent::Diagnostics { path, items } => {
                    let title = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string());
                    if self
                        .model
                        .problems
                        .update_lsp_diagnostics(path, title, items)
                    {
                        changed = true;
                    }
                }
            }
        }
        let events = match self.panel.as_mut() {
            Some(p) => p.pump(),
            None => Vec::new(),
        };
        for ev in events {
            match ev {
                UiEvent::StatusItem { id, visible, text } => {
                    changed = true;
                    if visible {
                        self.status_items.insert(id, text);
                    } else {
                        self.status_items.remove(&id);
                    }
                }
                UiEvent::OpenExternal { id, url } => {
                    if self.external_prompt.is_none() && Self::valid_external_url(&url) {
                        self.external_prompt = Some((id, url));
                        changed = true;
                    } else if let Some(p) = self.panel.as_ref() {
                        p.resolve_external(id, false);
                    }
                }
                UiEvent::TerminalCreate {
                    id,
                    name,
                    shell_path,
                    shell_args,
                    cwd,
                } => {
                    self.spawn_terminal_ext(Some(id), Some(name), shell_path, shell_args, cwd);
                    changed = true;
                }
                UiEvent::TerminalSendText {
                    id,
                    text,
                    add_new_line,
                } => {
                    if let Some(idx) = self.find_terminal_by_ext_id(&id)
                        && let Some(term) = self.terminals.get(idx)
                    {
                        let payload = if add_new_line {
                            format!("{text}\r\n")
                        } else {
                            text
                        };
                        term.term.send_text(&payload);
                        changed = true;
                    }
                }
                UiEvent::TerminalShow { id, preserve_focus } => {
                    if let Some(idx) = self.find_terminal_by_ext_id(&id) {
                        self.model.panel_tab = crate::model::PanelTab::Terminal;
                        self.model.layout.panel = true;
                        self.active_terminal = idx;
                        if !preserve_focus {
                            self.model.terminal_focused = true;
                        }
                        changed = true;
                    }
                }
                UiEvent::TerminalDispose { id } => {
                    if let Some(idx) = self.find_terminal_by_ext_id(&id) {
                        self.close_terminal(idx);
                        changed = true;
                    }
                }
                _ => {}
            }
        }
        changed
    }

    pub(crate) fn valid_external_url(value: &str) -> bool {
        if value
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
        {
            return false;
        }
        let Ok(parsed) = url::Url::parse(value) else {
            return false;
        };
        matches!(parsed.scheme(), "http" | "https")
            && parsed.has_host()
            && parsed.username().is_empty()
            && parsed.password().is_none()
    }

    fn resolve_external_prompt(&mut self, approved: bool, cx: &mut Context<Self>) {
        let Some((id, url)) = self.external_prompt.take() else {
            return;
        };
        let approved = approved && Self::valid_external_url(&url);
        if approved {
            cx.open_url(&url);
        }
        if let Some(panel) = &self.panel {
            panel.resolve_external(id, approved);
        }
        cx.notify();
    }

    fn reveal_cursor(&self) {
        if let Some(TabContent::Text(e)) = self.model.active().map(|t| &t.content) {
            let viewport = f32::from(self.scroll.bounds().size.height);
            if viewport <= 0.0 {
                return;
            }
            let lines = e.buffer().len_lines().min(EDITOR_MAX_LINES);
            let content = 2.0 * EDITOR_PAD_Y + lines as f32 * LINE_HEIGHT;
            let top = EDITOR_PAD_Y + e.line_col().0 as f32 * LINE_HEIGHT;
            let bottom = top + LINE_HEIGHT;
            let offset = self.scroll.offset();
            let mut scrolled = -f32::from(offset.y);
            if top - EDITOR_PAD_Y < scrolled {
                scrolled = top - EDITOR_PAD_Y;
            } else if bottom + EDITOR_PAD_Y > scrolled + viewport {
                scrolled = bottom + EDITOR_PAD_Y - viewport;
            }
            scrolled = scrolled.clamp(0.0, (content - viewport).max(0.0));
            self.scroll.set_offset(point(offset.x, px(-scrolled)));
        }
    }

    fn finish_quit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.quitting = true;
        self.kill_all_terminals();
        self.lsp.shutdown_all();
        window.remove_window();
        cx.quit();
    }

    fn target(&self) -> Target {
        if self.model.palette.is_some() {
            return Target::Palette;
        }
        match self.model.find_focus() {
            FindFocus::Query => return Target::FindQuery,
            FindFocus::Replace => return Target::FindReplace,
            FindFocus::Editor => {}
        }
        if self.model.search.focused
            && self.model.activity == Activity::Search
            && self.model.layout.sidebar
        {
            return Target::Search;
        }
        if self.model.scm.focused
            && self.model.activity == Activity::SourceControl
            && self.model.layout.sidebar
        {
            return Target::Commit;
        }
        if self.model.terminal_focused
            && self.model.layout.panel
            && self.model.panel_tab == PanelTab::Terminal
            && !self.terminals.is_empty()
        {
            return Target::Terminal;
        }
        if self.model.ext_query_focused
            && self.model.activity == Activity::Extensions
            && self.model.layout.sidebar
        {
            return Target::ExtSearch;
        }
        Target::Editor
    }

    fn field(&self) -> Option<&TextField> {
        match self.target() {
            Target::Palette => self.model.palette.as_ref().map(|p| &p.field),
            Target::FindQuery => self.model.find.as_ref().map(|f| &f.query),
            Target::FindReplace => self.model.find.as_ref().map(|f| &f.replace),
            Target::Search => Some(&self.model.search.query),
            Target::Commit => Some(&self.model.scm.message),
            Target::ExtSearch => Some(&self.model.ext_query),
            Target::Editor | Target::Terminal => None,
        }
    }

    fn field_mut(&mut self) -> Option<&mut TextField> {
        match self.target() {
            Target::Palette => self.model.palette.as_mut().map(|p| &mut p.field),
            Target::FindQuery => self.model.find.as_mut().map(|f| &mut f.query),
            Target::FindReplace => self.model.find.as_mut().map(|f| &mut f.replace),
            Target::Search => Some(&mut self.model.search.query),
            Target::Commit => Some(&mut self.model.scm.message),
            Target::ExtSearch => Some(&mut self.model.ext_query),
            Target::Editor | Target::Terminal => None,
        }
    }

    fn field_pad(&self) -> f32 {
        if self.target() == Target::Palette {
            PALETTE_INPUT_PAD
        } else {
            FIELD_PAD
        }
    }

    fn on_field_edited(&mut self) {
        match self.target() {
            Target::Palette => {
                if let Some(p) = self.model.palette.as_mut() {
                    p.refilter();
                }
            }
            Target::FindQuery => self.model.find_select_nearest(),
            Target::ExtSearch => self.model.on_ext_search_edited(),
            _ => {}
        }
    }

    fn reclaim_focus(&self) {
        if let Some(panel) = self.panel.as_ref() {
            panel.focus_parent();
        }
    }

    fn focus_target(&mut self, target: Target, window: &mut Window) {
        self.reclaim_focus();
        match target {
            Target::FindQuery | Target::FindReplace => {
                if let Some(f) = self.model.find.as_mut() {
                    f.focus = if target == Target::FindQuery {
                        FindFocus::Query
                    } else {
                        FindFocus::Replace
                    };
                }
                self.model.search.focused = false;
            }
            Target::Search => {
                self.model.focus_editor_inputs();
                self.model.search.focused = true;
            }
            Target::Commit => {
                self.model.focus_editor_inputs();
                self.model.scm.focused = true;
            }
            Target::Terminal => {
                self.model.focus_editor_inputs();
                self.model.terminal_focused = true;
            }
            Target::ExtSearch => {
                self.model.focus_editor_inputs();
                self.model.ext_query_focused = true;
            }
            Target::Editor | Target::Palette => self.model.focus_editor_inputs(),
        }
        self.sync_focus(window);
    }

    pub fn editor_scroll_metrics(&self) -> (f32, f32, f32) {
        (
            f32::from(self.scroll.bounds().size.height),
            f32::from(self.scroll.max_offset().height),
            f32::from(self.scroll.offset().y),
        )
    }

    pub fn run_task(&mut self, index: usize, window: &mut Window) {
        self.model.run_task(index);
        self.process_terminal_request();
        self.focus_target(Target::Terminal, window);
    }

    pub fn focus_commit_message(&mut self, window: &mut Window) {
        self.focus_target(Target::Commit, window);
    }

    pub fn focus_editor(&mut self, window: &mut Window) {
        self.model.focus_editor_inputs();
        window.focus(&self.focus);
    }

    fn sync_focus(&self, window: &mut Window) {
        match self.target() {
            Target::Palette => window.focus(&self.palette_focus),
            Target::FindQuery => window.focus(&self.find_query_focus),
            Target::FindReplace => window.focus(&self.find_replace_focus),
            Target::Search => window.focus(&self.search_focus),
            Target::Commit => window.focus(&self.commit_focus),
            Target::Terminal => window.focus(&self.terminal_focus),
            Target::ExtSearch => window.focus(&self.ext_search_focus),
            Target::Editor if self.model.layout.panel && self.model.layout.panel_maximized => {
                window.focus(&self.root_focus)
            }
            Target::Editor => window.focus(&self.focus),
        }
    }

    fn drain_requests(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.model.clipboard_request.take() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
        if let Some(req) = self.model.pick_request.take() {
            self.start_picker(req, window, cx);
        }
        if self.model.panel_request.is_some() {
            self.activate_panel(window);
        }
        if std::mem::take(&mut self.model.quit_request) && self.model.request_quit() {
            self.finish_quit(window, cx);
        }
    }

    fn apply_command(&mut self, command: Command, window: &mut Window, cx: &mut Context<Self>) {
        match command {
            Command::Copy | Command::Cut | Command::Paste => {
                self.clipboard_command(command, cx);
                self.focus_editor(window);
                self.reveal_cursor();
                return cx.notify();
            }
            _ => {}
        }
        self.model.run_command(command);
        self.process_terminal_request();
        self.drain_requests(window, cx);
        self.sync_focus(window);
        self.reveal_cursor();
        cx.notify();
    }

    fn clipboard_command(&mut self, command: Command, cx: &mut Context<Self>) {
        let Some(e) = self.model.active_editor_mut() else {
            return;
        };
        match command {
            Command::Copy if e.has_selection() => {
                cx.write_to_clipboard(ClipboardItem::new_string(e.selected_text()));
            }
            Command::Cut if e.has_selection() => {
                cx.write_to_clipboard(ClipboardItem::new_string(e.cut()));
            }
            Command::Paste => {
                if let Some(text) = cx.read_from_clipboard().and_then(|c| c.text()) {
                    e.insert(&text);
                }
            }
            _ => {}
        }
    }

    fn on_editor_right_down(
        &mut self,
        ev: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_editor(window);
        if self.model.prompt.is_some() {
            return;
        }
        if let Some(pos) = self.position_at(ev.position, window)
            && let Some(e) = self.model.active_editor_mut()
            && !e.selection().contains(&pos)
        {
            e.set_cursor(pos, false);
        }
        if self.model.active_editor_mut().is_some() {
            self.model
                .open_editor_menu(f32::from(ev.position.x), f32::from(ev.position.y));
        }
        cx.notify();
    }

    fn accept_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(p) = self.model.palette.as_ref()
            && let Some(crate::palette::PaletteAction::Command(
                c @ (Command::OpenFile | Command::OpenFolder),
            )) = p.selected_action()
        {
            self.model.close_palette();
            self.apply_command(c, window, cx);
            return;
        }
        self.model.accept_palette();
        self.process_terminal_request();
        self.drain_requests(window, cx);
        self.sync_focus(window);
        self.reveal_cursor();
        cx.notify();
    }

    fn on_palette_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let m = ev.keystroke.modifiers;
        let key = ev.keystroke.key.as_str();
        match key {
            "escape" => {
                self.model.close_palette();
                self.sync_focus(window);
            }
            "enter" => return self.accept_palette_and_stop(window, cx),
            _ => {
                let Some(p) = self.model.palette.as_mut() else {
                    return;
                };
                match key {
                    "up" => p.move_selection(-1),
                    "down" | "tab" => p.move_selection(1),
                    "pageup" => p.move_selection(-8),
                    "pagedown" => p.move_selection(8),
                    "left" => p.field.left(),
                    "right" => p.field.right(),
                    "home" => p.field.home(),
                    "end" => p.field.end(),
                    "backspace" => {
                        p.field.backspace();
                        p.refilter();
                    }
                    "delete" => {
                        p.field.delete();
                        p.refilter();
                    }
                    _ if m.control && !m.alt => {
                        if let Some(c @ (Command::CommandPalette | Command::QuickOpen)) =
                            Command::from_ctrl_key(key, m.shift)
                        {
                            return self.run_and_stop(c, window, cx);
                        }
                    }
                    _ => return,
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn field_shortcut(key: &str, shift: bool) -> Option<Command> {
        match Command::from_ctrl_key(key, shift) {
            Some(
                c @ (Command::CommandPalette
                | Command::QuickOpen
                | Command::Find
                | Command::Replace
                | Command::FindInFiles
                | Command::Save),
            ) => Some(c),
            _ => None,
        }
    }

    fn on_find_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let m = ev.keystroke.modifiers;
        let key = ev.keystroke.key.as_str();
        let target = self.target();
        if m.alt && !m.control {
            match key {
                "c" => self.model.find.as_mut().map(|f| f.toggle_case()),
                "w" => self.model.find.as_mut().map(|f| f.toggle_whole_word()),
                "r" => self.model.find.as_mut().map(|f| f.toggle_regex()),
                _ => return,
            };
            self.model.find_select_nearest();
            return self.after_field_key(window, cx);
        }
        if m.control && !m.alt {
            return match Self::field_shortcut(key, m.shift) {
                Some(c) => self.run_and_stop(c, window, cx),
                None => cx.stop_propagation(),
            };
        }
        match key {
            "escape" => {
                self.model.close_find();
            }
            "enter" if target == Target::FindReplace => {
                if m.shift {
                    self.model.find_replace_all();
                } else {
                    self.model.find_replace_current();
                }
            }
            "enter" | "f3" | "down" | "up" => {
                if (key == "enter" || key == "f3") && m.shift || key == "up" {
                    self.model.find_prev();
                } else {
                    self.model.find_next();
                }
            }
            "tab" => {
                if let Some(f) = self.model.find.as_mut()
                    && f.show_replace
                {
                    f.focus = if f.focus == FindFocus::Query {
                        FindFocus::Replace
                    } else {
                        FindFocus::Query
                    };
                }
            }
            "left" | "right" | "home" | "end" | "backspace" | "delete" => {
                if let Some(f) = self.field_mut() {
                    match key {
                        "left" => f.left(),
                        "right" => f.right(),
                        "home" => f.home(),
                        "end" => f.end(),
                        "backspace" => f.backspace(),
                        _ => f.delete(),
                    }
                }
                self.on_field_edited();
            }
            _ => return,
        }
        self.after_field_key(window, cx);
    }

    fn on_search_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let m = ev.keystroke.modifiers;
        let key = ev.keystroke.key.as_str();
        if m.alt && !m.control {
            match key {
                "c" => self.model.search.opts.case_sensitive ^= true,
                "w" => self.model.search.opts.whole_word ^= true,
                "r" => self.model.search.opts.regex ^= true,
                _ => return,
            }
            return self.after_field_key(window, cx);
        }
        if m.control && !m.alt {
            return match Self::field_shortcut(key, m.shift) {
                Some(c) => self.run_and_stop(c, window, cx),
                None => cx.stop_propagation(),
            };
        }
        match key {
            "escape" => self.model.search.focused = false,
            "enter" => self.model.run_search(),
            "left" | "right" | "home" | "end" | "backspace" | "delete" => {
                if let Some(f) = self.field_mut() {
                    match key {
                        "left" => f.left(),
                        "right" => f.right(),
                        "home" => f.home(),
                        "end" => f.end(),
                        "backspace" => f.backspace(),
                        _ => f.delete(),
                    }
                }
            }
            _ => return,
        }
        self.after_field_key(window, cx);
    }

    fn on_commit_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let m = ev.keystroke.modifiers;
        let key = ev.keystroke.key.as_str();
        if m.alt && !m.control {
            return;
        }
        if m.control && !m.alt {
            if key == "enter" {
                self.run_and_stop(Command::GitCommit, window, cx);
                return;
            }
            return match Self::field_shortcut(key, m.shift) {
                Some(c) => self.run_and_stop(c, window, cx),
                None if Command::from_ctrl_key(key, m.shift)
                    == Some(Command::ShowSourceControl) =>
                {
                    self.run_and_stop(Command::ShowSourceControl, window, cx)
                }
                None => cx.stop_propagation(),
            };
        }
        match key {
            "escape" => self.model.scm.focused = false,
            "left" | "right" | "home" | "end" | "backspace" | "delete" => {
                if let Some(f) = self.field_mut() {
                    match key {
                        "left" => f.left(),
                        "right" => f.right(),
                        "home" => f.home(),
                        "end" => f.end(),
                        "backspace" => f.backspace(),
                        _ => f.delete(),
                    }
                }
            }
            "enter" => {}
            _ => return,
        }
        self.after_field_key(window, cx);
    }

    fn on_ext_search_key(
        &mut self,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let m = ev.keystroke.modifiers;
        let key = ev.keystroke.key.as_str();
        if m.control && !m.alt {
            return match Self::field_shortcut(key, m.shift) {
                Some(c) => self.run_and_stop(c, window, cx),
                None => cx.stop_propagation(),
            };
        }
        match key {
            "escape" => self.model.ext_query_focused = false,
            "enter" => {
                let q = self.model.ext_query.text().to_owned();
                self.model.search_marketplace(&q);
            }
            "left" | "right" | "home" | "end" | "backspace" | "delete" => {
                if let Some(f) = self.field_mut() {
                    match key {
                        "left" => f.left(),
                        "right" => f.right(),
                        "home" => f.home(),
                        "end" => f.end(),
                        "backspace" => f.backspace(),
                        _ => f.delete(),
                    }
                }
            }
            _ => return,
        }
        self.after_field_key(window, cx);
    }

    fn after_field_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.on_field_edited();
        self.model.refresh_find();
        self.sync_focus(window);
        self.reveal_cursor();
        cx.stop_propagation();
        cx.notify();
    }

    fn accept_palette_and_stop(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.accept_palette(window, cx);
        cx.stop_propagation();
    }

    fn run_and_stop(&mut self, command: Command, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_command(command, window, cx);
        cx.stop_propagation();
    }

    fn on_key_down(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let k = &ev.keystroke;
        let m = k.modifiers;
        let key = k.key.as_str();

        // An external URL request is never approved by a keyboard shortcut.
        if self.external_prompt.is_some() {
            if key == "escape" {
                self.resolve_external_prompt(false, cx);
            }
            cx.stop_propagation();
            return;
        }

        if (self.model.menu_open.is_some() || self.model.context_menu.is_some()) && key == "escape"
        {
            self.model.close_menus();
            cx.stop_propagation();
            return cx.notify();
        }

        match self.target() {
            Target::Palette => return self.on_palette_key(ev, window, cx),
            Target::FindQuery | Target::FindReplace => return self.on_find_key(ev, window, cx),
            Target::Search => return self.on_search_key(ev, window, cx),
            Target::Commit => return self.on_commit_key(ev, window, cx),
            Target::Terminal => return self.on_terminal_key(ev, window, cx),
            Target::ExtSearch => return self.on_ext_search_key(ev, window, cx),
            Target::Editor => {}
        }

        if self.model.prompt.is_some() {
            match key {
                "escape" => self.model.resolve_cancel(),
                "enter" => {
                    if self.model.resolve_save() {
                        return self.finish_quit(window, cx);
                    }
                }
                _ => return,
            }
            return cx.notify();
        }

        if key == "f2" && !m.control && !m.alt {
            return self.run_and_stop(Command::RenameItem, window, cx);
        }
        if key == "f3" {
            let command = if m.shift {
                Command::FindPrev
            } else {
                Command::FindNext
            };
            return self.run_and_stop(command, window, cx);
        }
        if key == "escape" && self.model.find.is_some() {
            self.model.close_find();
            cx.stop_propagation();
            return cx.notify();
        }

        let ctrl = m.control && !m.alt;
        if ctrl {
            if let Some(command) = Command::from_ctrl_key(key, m.shift) {
                return self.run_and_stop(command, window, cx);
            }
            let Some(e) = self.model.active_editor_mut() else {
                return;
            };
            match (key, m.shift) {
                ("c", _) => {
                    if e.has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(e.selected_text()));
                    }
                }
                ("x", _) => {
                    if e.has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(e.cut()));
                    }
                }
                ("v", _) => {
                    if let Some(text) = cx.read_from_clipboard().and_then(|c| c.text()) {
                        e.insert(&text);
                    }
                }
                ("home", s) => e.move_doc_start(s),
                ("end", s) => e.move_doc_end(s),
                ("left", s) => e.move_word_left(s),
                ("right", s) => e.move_word_right(s),
                ("backspace", _) => e.delete_word_left(),
                ("delete", _) => e.delete_word_right(),
                ("enter", s) => e.insert_line(!s, model::INDENT_UNIT),
                _ => return,
            }
            self.reveal_cursor();
            cx.stop_propagation();
            return cx.notify();
        }

        if m.alt && !m.control {
            let command = match (key, m.shift) {
                ("up", false) => Some(Command::MoveLineUp),
                ("down", false) => Some(Command::MoveLineDown),
                ("f", true) => Some(Command::FormatDocument),
                ("up", true) => Some(Command::DuplicateLineUp),
                ("down", true) => Some(Command::DuplicateLineDown),
                _ => None,
            };
            if let Some(c) = command {
                return self.run_and_stop(c, window, cx);
            }
        }

        let html = self
            .model
            .active()
            .is_some_and(|t| emmet::applies_to(&t.path));
        let Some(e) = self.model.active_editor_mut() else {
            return;
        };
        let s = m.shift;
        if key == "tab" && !s && html && emmet::expand_at_cursor(e, model::INDENT_UNIT) {
            self.reveal_cursor();
            cx.stop_propagation();
            return cx.notify();
        }
        match key {
            "left" => e.move_left(s),
            "right" => e.move_right(s),
            "up" => e.move_up(s),
            "down" => e.move_down(s),
            "home" => e.move_line_start(s),
            "end" => e.move_line_end(s),
            "pageup" => (0..20).for_each(|_| e.move_up(s)),
            "pagedown" => (0..20).for_each(|_| e.move_down(s)),
            "backspace" => e.backspace(),
            "delete" => e.delete_forward(),
            "enter" => e.smart_newline(model::INDENT_UNIT),
            "tab" if s => e.outdent_lines(model::INDENT_UNIT),
            "tab" if e.has_multiline_selection() => e.indent_lines(model::INDENT_UNIT),
            "tab" => e.insert(model::INDENT_UNIT),
            _ => return,
        }
        self.reveal_cursor();
        cx.stop_propagation();
        cx.notify();
    }

    fn position_at(&self, p: Point<Pixels>, window: &Window) -> Option<usize> {
        let Some(TabContent::Text(e)) = self.model.active().map(|t| &t.content) else {
            return None;
        };
        let bounds = self.scroll.bounds();
        let offset = self.scroll.offset();
        let y = f32::from(p.y - bounds.origin.y - offset.y) - EDITOR_PAD_Y;
        let shown = e.buffer().len_lines().min(EDITOR_MAX_LINES);
        let line = ((y / LINE_HEIGHT).floor().max(0.0) as usize).min(shown.saturating_sub(1));
        let text = e.buffer().line_text(line);
        let x = (p.x - bounds.origin.x - px(GUTTER_WIDTH)).max(px(0.));
        Some(e.buffer().line_start(line) + col_at_x(window, &text, x))
    }

    fn on_editor_mouse_down(
        &mut self,
        ev: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_editor(window);
        if self.model.prompt.is_some() {
            return;
        }
        if let Some(pos) = self.position_at(ev.position, window)
            && let Some(e) = self.model.active_editor_mut()
        {
            match ev.click_count {
                2 => e.select_word_at(pos),
                n if n >= 3 => e.select_line_at(pos),
                _ => e.set_cursor(pos, ev.modifiers.shift),
            }
            self.dragging = ev.click_count < 2;
        }
        cx.notify();
    }

    fn on_editor_mouse_move(
        &mut self,
        ev: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.dragging || ev.pressed_button != Some(MouseButton::Left) {
            return;
        }
        if let Some(pos) = self.position_at(ev.position, window)
            && let Some(e) = self.model.active_editor_mut()
        {
            e.set_cursor(pos, true);
            cx.notify();
        }
    }

    fn on_editor_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.dragging = false;
    }

    fn on_root_mouse_move(
        &mut self,
        ev: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(handle) = self.scroll_drag.clone() {
            if ev.pressed_button == Some(MouseButton::Left) {
                views::scroll_to_pointer(&handle, ev.position.y);
                cx.notify();
            } else {
                self.scroll_drag = None;
            }
            return;
        }
        let Some(drag) = self.drag else {
            return;
        };
        if ev.pressed_button != Some(MouseButton::Left) {
            self.drag = None;
            return;
        }
        let vp = window.viewport_size();
        let (vw, vh) = (f32::from(vp.width), f32::from(vp.height));
        let (x, y) = (f32::from(ev.position.x), f32::from(ev.position.y));
        let l = &mut self.model.layout;
        let status = if l.status_bar { STATUS_BAR_HEIGHT } else { 0.0 };
        let activity = if l.activity_bar {
            ACTIVITY_BAR_WIDTH
        } else {
            0.0
        };
        let sidebar_w = if l.sidebar {
            l.effective_sidebar(vw) + SPLITTER
        } else {
            0.0
        };
        let (left_edge, right_edge) = match l.sidebar_side {
            Side::Left => (activity + sidebar_w, 0.0),
            Side::Right => (0.0, activity + sidebar_w),
        };
        let column_h = vh - status - TAB_BAR_HEIGHT - MENU_BAR_HEIGHT;
        match drag {
            Drag::Sidebar => match l.sidebar_side {
                Side::Left => l.set_sidebar_width(x - activity, vw),
                Side::Right => l.set_sidebar_width(vw - x - activity, vw),
            },
            Drag::Panel => {
                let height = if l.panel_pos == PanelPos::Top {
                    y - MENU_BAR_HEIGHT
                } else {
                    vh - status - y
                };
                l.set_panel_height(height, column_h);
            }
            Drag::PanelSide => {
                let width = if l.panel_pos == PanelPos::Left {
                    x - left_edge
                } else {
                    vw - right_edge - x
                };
                l.set_panel_width(width, vw);
            }
        }
        cx.notify();
    }

    fn on_root_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.scroll_drag.take().is_some() {
            cx.notify();
            return;
        }
        if self.drag.take().is_some() {
            cx.notify();
        }
    }

    fn splitter(
        &self,
        id: &'static str,
        drag: Drag,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let vertical = drag != Drag::Panel;
        let active = self.drag == Some(drag);
        div()
            .id(id)
            .group(id)
            .relative()
            .flex_none()
            .when(vertical, |d| d.w(px(SPLITTER)).h_full())
            .when(!vertical, |d| d.h(px(SPLITTER)).w_full())
            .child(
                div()
                    .size_full()
                    .bg(if active {
                        rgb(theme::ACCENT)
                    } else {
                        rgb(theme::BORDER)
                    })
                    .group_hover(id, |s| s.bg(rgb(theme::ACCENT))),
            )
            .child(
                div()
                    .id(SharedString::from(format!("{id}-grab")))
                    .absolute()
                    .when(vertical, |d| {
                        d.top_0()
                            .bottom_0()
                            .left(px(-SPLITTER_GRAB))
                            .w(px(SPLITTER + SPLITTER_GRAB))
                            .cursor_col_resize()
                    })
                    .when(!vertical, |d| {
                        d.left_0()
                            .right_0()
                            .top(px(-SPLITTER_GRAB))
                            .h(px(SPLITTER + SPLITTER_GRAB))
                            .cursor_row_resize()
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                            if ev.click_count >= 2 {
                                let d = Layout::default();
                                match drag {
                                    Drag::Sidebar => {
                                        this.model.layout.sidebar_width = d.sidebar_width
                                    }
                                    Drag::Panel => this.model.layout.panel_height = d.panel_height,
                                    Drag::PanelSide => {
                                        this.model.layout.panel_width = d.panel_width
                                    }
                                }
                                this.drag = None;
                            } else {
                                this.drag = Some(drag);
                            }
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    ),
            )
    }

    fn activity_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(48.))
            .h_full()
            .bg(rgb(theme::ACTIVITY_BAR))
            .when(self.model.layout.sidebar_side == Side::Left, |d| {
                d.border_r_1()
            })
            .when(self.model.layout.sidebar_side == Side::Right, |d| {
                d.border_l_1()
            })
            .border_color(rgb(theme::BORDER))
            .items_center()
            .pt_2()
            .gap_1()
            .children(Activity::ALL.into_iter().enumerate().map(|(i, a)| {
                let selected = a == self.model.activity && self.model.layout.sidebar;
                let badge = (a == Activity::SourceControl && self.model.scm.change_count() > 0)
                    .then(|| self.model.scm.change_count());
                div()
                    .id(("activity", i))
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(40.))
                    .h(px(40.))
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::HOVER)))
                    .when(selected, |d| d.bg(rgb(theme::SELECTED)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.model.select_activity(a);
                        if this.model.activity == a && this.model.layout.sidebar {
                            match a {
                                Activity::Extensions => {
                                    this.focus_target(Target::ExtSearch, window)
                                }
                                Activity::Search => this.focus_target(Target::Search, window),
                                Activity::SourceControl => {
                                    this.focus_target(Target::Commit, window)
                                }
                                _ => {}
                            }
                        }
                        cx.notify();
                    }))
                    .child(assets::icon(
                        activity_icon(a),
                        22.,
                        if selected { 0xffffff } else { theme::ICON },
                    ))
                    .children(badge.map(|n| {
                        div()
                            .absolute()
                            .right(px(1.))
                            .bottom(px(3.))
                            .min_w(px(16.))
                            .h(px(16.))
                            .px_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(rgb(theme::ACCENT))
                            .text_color(rgb(0x111111))
                            .text_size(px(10.))
                            .child(SharedString::from(if n > 99 {
                                "99+".to_owned()
                            } else {
                                n.to_string()
                            }))
                    }))
            }))
            .child(div().flex_1())
            .child(
                div()
                    .id("activity-settings")
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(40.))
                    .h(px(40.))
                    .mb_2()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::HOVER)))
                    .when(self.model.settings_open, |d| d.bg(rgb(theme::SELECTED)))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.apply_command(Command::OpenSettings, window, cx);
                    }))
                    .child(assets::icon(
                        "settings",
                        22.,
                        if self.model.settings_open {
                            0xffffff
                        } else {
                            theme::ICON
                        },
                    )),
            )
    }

    fn explorer_entries(&self) -> Rc<Vec<model::Entry>> {
        let sig = self.model.explorer_signature();
        if let Some((at, s, list)) = &*self.explorer_cache.borrow()
            && *s == sig
            && at.elapsed() < EXPLORER_CACHE_TTL
        {
            return list.clone();
        }
        let list = Rc::new(self.model.visible_entries());
        *self.explorer_cache.borrow_mut() = Some((std::time::Instant::now(), sig, list.clone()));
        list
    }

    fn sidebar(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let body: AnyElement = if self.model.activity == Activity::Explorer
            && !self.model.folder_open
        {
            self.no_folder_view(cx)
        } else if self.model.activity == Activity::Explorer {
            let active_path = self.model.active().map(|t| t.path.clone());
            let entries = self.explorer_entries();
            let total = entries.len();
            let viewport = f32::from(self.scrolls.explorer.bounds().size.height);
            let viewport = if viewport > 0.0 { viewport } else { 1600.0 };
            let scrolled = (-f32::from(self.scrolls.explorer.offset().y)).max(0.0);
            let first = ((scrolled / EXPLORER_ROW_H) as usize)
                .saturating_sub(OVERSCAN_ROWS)
                .min(total);
            let last = (((scrolled + viewport) / EXPLORER_ROW_H).ceil() as usize + OVERSCAN_ROWS)
                .min(total)
                .max(first);
            let list = div()
                .id("explorer-list")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.scrolls.explorer)
                .child(div().h(px(first as f32 * EXPLORER_ROW_H)).flex_none())
                .children(
                    entries[first..last]
                        .iter()
                        .cloned()
                        .enumerate()
                        .map(|(k, e)| {
                            let i = first + k;
                            let selected = (!e.is_dir && active_path.as_ref() == Some(&e.path))
                                || self.model.explorer_sel.as_deref() == Some(e.path.as_path());
                            let chevron = match (e.is_dir, e.expanded) {
                                (true, true) => Some("chevron-down"),
                                (true, false) => Some("chevron-right"),
                                (false, _) => None,
                            };
                            let path = e.path.clone();
                            let is_dir = e.is_dir;
                            let decor = self.model.scm.decor_for(&e.path, e.is_dir);
                            let name_color = decor.map_or(theme::TEXT, |d| tone_color(d.tone));
                            div()
                                .id(("entry", i))
                                .flex()
                                .items_center()
                                .justify_between()
                                .pl(px(8. + 12. * e.depth as f32))
                                .pr_2()
                                .h(px(EXPLORER_ROW_H))
                                .text_size(px(13.))
                                .text_color(rgb(name_color))
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Right,
                                    cx.listener({
                                        let path = path.clone();
                                        move |this, ev: &MouseDownEvent, _w, cx| {
                                            this.model.open_context_menu(
                                                &path,
                                                is_dir,
                                                f32::from(ev.position.x),
                                                f32::from(ev.position.y),
                                            );
                                            cx.notify();
                                        }
                                    }),
                                )
                                .when(selected, |d| d.bg(rgb(theme::SELECTED)))
                                .hover(|s| s.bg(rgb(theme::HOVER)))
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    this.model.select_entry(&path);
                                    if is_dir {
                                        this.model.toggle_dir(&path);
                                    } else {
                                        this.model.open_file(&path);
                                        this.focus_editor(window);
                                    }
                                    cx.notify();
                                }))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .min_w_0()
                                        .child(div().flex_none().w(px(14.)).children(
                                            chevron.map(|c| assets::icon(c, 14., theme::TEXT_DIM)),
                                        ))
                                        .child(if is_dir {
                                            assets::icon("folder", 15., 0x7f9fbf).into_any_element()
                                        } else {
                                            Self::file_icon(&e.name, 15.)
                                        })
                                        .child(
                                            div()
                                                .whitespace_nowrap()
                                                .overflow_hidden()
                                                .child(SharedString::from(e.name.clone())),
                                        ),
                                )
                                .children(decor.map(|d| {
                                    div()
                                        .flex_none()
                                        .pl_2()
                                        .text_size(px(11.))
                                        .child(SharedString::from(d.letter.to_string()))
                                }))
                        }),
                )
                .child(
                    div()
                        .h(px((total - last) as f32 * EXPLORER_ROW_H))
                        .flex_none(),
                )
                .into_any_element();
            let list = self.scroll_box("explorer-list", list, &self.scrolls.explorer, cx);
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .child(self.explorer_toolbar(cx))
                .child(list)
                .child(self.outline_section(cx))
                .into_any_element()
        } else if self.model.activity == Activity::Search {
            self.search_panel(window, cx)
        } else if self.model.activity == Activity::SourceControl {
            self.scm_panel(window, cx)
        } else if self.model.activity == Activity::Run {
            self.run_panel(cx)
        } else {
            self.extensions_panel(window, cx)
        };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(self.model.layout.effective_sidebar(f32::from(
                window.viewport_size().width,
            ))))
            .h_full()
            .bg(rgb(theme::SIDEBAR))
            .when(self.model.layout.sidebar_side == Side::Left, |d| {
                d.border_r_1()
            })
            .when(self.model.layout.sidebar_side == Side::Right, |d| {
                d.border_l_1()
            })
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .flex_none()
                    .px_3()
                    .py_2()
                    .text_size(px(11.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(theme::TEXT_DIM))
                    .child(self.model.activity.title().to_uppercase()),
            )
            .child(body)
    }

    fn tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .flex()
            .flex_none()
            .h(px(35.))
            .bg(rgb(
                if self.model.tabs.is_empty()
                    && self.model.ext_detail.is_none()
                    && !self.model.settings_open
                {
                    theme::EDITOR
                } else {
                    theme::TAB_BAR
                },
            ))
            .when(
                !self.model.tabs.is_empty()
                    || self.model.ext_detail.is_some()
                    || self.model.settings_open,
                |d| d.border_b_1().border_color(rgb(theme::BORDER)),
            )
            .children(self.model.settings_open.then(|| {
                div()
                    .id("tab-settings")
                    .flex()
                    .items_center()
                    .gap_2()
                    .pl_3()
                    .pr_1()
                    .text_size(px(13.))
                    .border_r_1()
                    .border_color(rgb(theme::BORDER))
                    .bg(rgb(theme::EDITOR))
                    .text_color(rgb(theme::TEXT))
                    .child(assets::icon("settings", 14., theme::TEXT_DIM))
                    .child("Settings")
                    .child(
                        div()
                            .id("tab-settings-close")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(18.))
                            .h(px(18.))
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(theme::SELECTED)))
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                this.model.close_settings();
                                cx.notify();
                            }))
                            .child(assets::icon("close", 12., theme::TEXT_DIM)),
                    )
            }))
            .children(self.model.ext_detail.as_ref().map(|d| {
                div()
                    .id("tab-ext-detail")
                    .flex()
                    .items_center()
                    .gap_2()
                    .pl_3()
                    .pr_1()
                    .text_size(px(13.))
                    .border_r_1()
                    .border_color(rgb(theme::BORDER))
                    .bg(rgb(theme::EDITOR))
                    .text_color(rgb(theme::TEXT))
                    .child(assets::icon("extensions", 14., theme::TEXT_DIM))
                    .child(SharedString::from(format!("Extension: {}", d.display_name)))
                    .child(
                        div()
                            .id("tab-ext-detail-close")
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(18.))
                            .h(px(18.))
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(theme::SELECTED)))
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                this.model.close_ext_detail();
                                cx.notify();
                            }))
                            .child(assets::icon("close", 12., theme::TEXT_DIM)),
                    )
            }))
            .children(self.model.tabs.iter().enumerate().map(|(i, t)| {
                let active = self.model.active_tab == Some(i)
                    && self.model.ext_detail.is_none()
                    && !self.model.settings_open;
                let dirty = t.is_dirty();
                div()
                    .id(("tab", i))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                            this.model.open_tab_menu(
                                i,
                                f32::from(ev.position.x),
                                f32::from(ev.position.y),
                            );
                            cx.notify();
                        }),
                    )
                    .relative()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pl_3()
                    .pr_1()
                    .text_size(px(13.))
                    .border_r_1()
                    .border_color(rgb(theme::BORDER))
                    .cursor_pointer()
                    .when(active, |d| {
                        d.bg(rgb(theme::EDITOR)).text_color(rgb(0xffffff)).child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .right_0()
                                .h(px(2.))
                                .bg(rgb(theme::ACCENT)),
                        )
                    })
                    .when(!active, |d| {
                        d.text_color(rgb(theme::TEXT_DIM))
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.model.activate_tab(i);
                        this.focus_editor(window);
                        cx.notify();
                    }))
                    .child(match &t.content {
                        TabContent::Webview => self.extension_icon(14.),
                        _ => Self::file_icon(&t.title, 14.),
                    })
                    .child(SharedString::from(t.title.clone()))
                    .child(
                        div()
                            .id(("tab-close", i))
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(18.))
                            .h(px(18.))
                            .rounded_sm()
                            .hover(|s| s.bg(rgb(theme::SELECTED)))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                cx.stop_propagation();
                                this.model.close_tab(i);
                                cx.notify();
                            }))
                            .child(if dirty {
                                div()
                                    .w(px(8.))
                                    .h(px(8.))
                                    .rounded_full()
                                    .bg(rgb(theme::TEXT))
                                    .into_any_element()
                            } else {
                                assets::icon("close", 12., theme::TEXT_DIM).into_any_element()
                            }),
                    )
            }))
    }

    fn prompt_modal_overlay(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if let Some(prompt) = self.model.prompt.clone() {
            let (title, text, primary, secondary, destructive) = match &prompt {
                Prompt::CloseDirty(i) => (
                    "Save Changes",
                    format!(
                        "Do you want to save the changes made to {}? Your changes will be lost if you don't save them.",
                        self.model
                            .tabs
                            .get(*i)
                            .map_or("this file", |t| t.title.as_str())
                    ),
                    "Save",
                    Some("Don't Save"),
                    false,
                ),
                Prompt::SaveConflict {
                    reason, then_close, ..
                } => (
                    "File Save Conflict",
                    format!("{reason}\n\nOverwrite it with your version?"),
                    "Overwrite",
                    Some(if *then_close {
                        "Discard Mine"
                    } else {
                        "Reload from Disk"
                    }),
                    true,
                ),
                Prompt::QuitDirty => (
                    "Unsaved Files",
                    format!(
                        "{} file(s) have unsaved changes. Do you want to save before closing?",
                        self.model.dirty_count()
                    ),
                    "Save All & Quit",
                    Some("Discard & Quit"),
                    false,
                ),
                Prompt::DeleteItem(p) => (
                    "Delete Item",
                    format!(
                        "Are you sure you want to move '{}' to the Recycle Bin?",
                        p.file_name()
                            .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
                    ),
                    "Move to Recycle Bin",
                    None,
                    true,
                ),
                Prompt::DiscardChanges { rel, untracked } => (
                    if *untracked {
                        "Delete Untracked File"
                    } else {
                        "Discard Changes"
                    },
                    if *untracked {
                        format!(
                            "Are you sure you want to move the untracked file '{rel}' to the Recycle Bin?"
                        )
                    } else {
                        format!(
                            "Are you sure you want to discard all unstaged changes to '{rel}'? This action cannot be undone."
                        )
                    },
                    if *untracked {
                        "Move to Recycle Bin"
                    } else {
                        "Discard Changes"
                    },
                    None,
                    true,
                ),
                Prompt::Push {
                    target,
                    ahead,
                    publish,
                } => (
                    "Push",
                    if *publish {
                        "Publish this branch to the remote and set it as the upstream?".to_owned()
                    } else {
                        format!(
                            "Push {ahead} commit{} to {target}?",
                            if *ahead == 1 { "" } else { "s" }
                        )
                    },
                    "Push",
                    None,
                    false,
                ),
                Prompt::RemoveExtension(p) => (
                    "Uninstall Extension",
                    format!(
                        "Remove the extension {}? All of its files will be permanently deleted.",
                        p.file_name()
                            .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
                    ),
                    "Remove",
                    None,
                    true,
                ),
            };

            let primary_btn = div()
                .id("prompt-primary")
                .flex()
                .items_center()
                .h(px(30.))
                .px_4()
                .rounded_md()
                .cursor_pointer()
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .when(destructive, |d| {
                    d.bg(rgb(0x9a2b2b))
                        .text_color(rgb(0xffffff))
                        .hover(|s| s.bg(rgb(0xba3636)))
                })
                .when(!destructive, |d| {
                    d.bg(rgb(theme::ACCENT))
                        .text_color(rgb(0x1e1e1e))
                        .hover(|s| s.bg(rgb(0xffffff)))
                })
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    if this.model.resolve_save() {
                        return this.finish_quit(window, cx);
                    }
                    cx.notify();
                }))
                .child(primary);

            // Only prompts with a third outcome (like "Don't Save") get a second button; Cancel is always there.
            let secondary_btn = secondary.map(|label| {
                div()
                    .id("prompt-secondary")
                    .flex()
                    .items_center()
                    .h(px(30.))
                    .px_3()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(rgb(0x333333))
                    .text_color(rgb(0xd0d0d0))
                    .text_size(px(12.))
                    .hover(|s| s.bg(rgb(0x404040)).text_color(rgb(0xffffff)))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        if this.model.resolve_discard() {
                            return this.finish_quit(window, cx);
                        }
                        cx.notify();
                    }))
                    .child(label)
            });

            let cancel_btn = div()
                .id("prompt-cancel")
                .flex()
                .items_center()
                .h(px(30.))
                .px_3()
                .rounded_md()
                .cursor_pointer()
                .bg(rgb(0x282828))
                .text_color(rgb(theme::TEXT_DIM))
                .text_size(px(12.))
                .hover(|s| s.bg(rgb(0x363636)).text_color(rgb(0xffffff)))
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    this.model.resolve_cancel();
                    cx.notify();
                }))
                .child("Cancel");

            return Some(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .id("modal-backdrop")
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .bg(rgba(0x000000a0))
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                this.model.resolve_cancel();
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_col()
                            .w(px(460.))
                            .max_w(px(520.))
                            .rounded_lg()
                            .bg(rgb(theme::MENU_BG))
                            .border_1()
                            .border_color(rgb(theme::BORDER))
                            .shadow_2xl()
                            .p_5()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .children(
                                        destructive.then(|| assets::icon("warning", 16., 0xe5a50a)),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(0xffffff))
                                            .child(title),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(rgb(0xc0c0c0))
                                    .child(SharedString::from(text)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_end()
                                    .gap_2()
                                    .pt_3()
                                    .child(primary_btn)
                                    .children(secondary_btn)
                                    .child(cancel_btn),
                            ),
                    )
                    .into_any_element(),
            );
        }

        if let Some((_, url)) = self.external_prompt.as_ref() {
            let url = url.clone();
            let primary_btn = div()
                .id("external-open")
                .flex()
                .items_center()
                .h(px(30.))
                .px_4()
                .rounded_md()
                .cursor_pointer()
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .bg(rgb(theme::ACCENT))
                .text_color(rgb(0x1e1e1e))
                .hover(|s| s.bg(rgb(0xffffff)))
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    this.resolve_external_prompt(true, cx);
                }))
                .child("Open");

            let cancel_btn = div()
                .id("external-cancel")
                .flex()
                .items_center()
                .h(px(30.))
                .px_3()
                .rounded_md()
                .cursor_pointer()
                .bg(rgb(0x282828))
                .text_color(rgb(theme::TEXT_DIM))
                .text_size(px(12.))
                .hover(|s| s.bg(rgb(0x363636)).text_color(rgb(0xffffff)))
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    this.resolve_external_prompt(false, cx);
                }))
                .child("Cancel");

            return Some(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .id("modal-backdrop")
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full()
                            .bg(rgba(0x000000a0))
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                this.resolve_external_prompt(false, cx);
                            })),
                    )
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_col()
                            .w(px(460.))
                            .max_w(px(520.))
                            .rounded_lg()
                            .bg(rgb(theme::MENU_BG))
                            .border_1()
                            .border_color(rgb(theme::BORDER))
                            .shadow_2xl()
                            .p_5()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().text_size(px(16.)).child("🌐"))
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(0xffffff))
                                            .child("Open External Website"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(rgb(0xc0c0c0))
                                    .child(SharedString::from(format!(
                                        "An extension wants to open an external link in your browser:\n\n{url}"
                                    ))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_end()
                                    .gap_2()
                                    .pt_3()
                                    .child(primary_btn)
                                    .child(cancel_btn),
                            ),
                    )
                    .into_any_element(),
            );
        }

        None
    }

    fn editor(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let base = div()
            .id("editor")
            .size_full()
            .overflow_y_scroll()
            .overflow_x_hidden()
            .track_scroll(&self.scroll)
            .track_focus(&self.focus)
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_editor_mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_editor_right_down))
            .on_mouse_move(cx.listener(Self::on_editor_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_editor_mouse_up))
            .bg(rgb(theme::EDITOR))
            .py(px(EDITOR_PAD_Y))
            .font_family(FONT_FAMILY)
            .text_size(px(FONT_SIZE))
            .text_color(rgb(theme::TEXT));
        let has_text = matches!(
            self.model.active().map(|t| &t.content),
            Some(TabContent::Text(_))
        );
        let inner = match self.model.active().map(|t| &t.content) {
            None => base
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(self.welcome(cx)),
            Some(TabContent::Unavailable(reason)) => base
                .px_4()
                .text_color(rgb(theme::TEXT_DIM))
                .child(SharedString::from(reason.clone())),
            Some(TabContent::Webview) => {
                let slot = self.webview_bounds.clone();
                let entity = cx.entity();
                base.child(
                    canvas(
                        |_, _, _| (),
                        move |bounds, (), _window, cx| {
                            if slot.get() != Some(bounds) {
                                slot.set(Some(bounds));
                                let entity = entity.clone();
                                cx.defer(move |cx| entity.update(cx, |_, cx| cx.notify()));
                            }
                        },
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                )
            }
            Some(TabContent::Diff(view)) => {
                let pad = EDITOR_PAD_Y;
                let scrolled = (-f32::from(self.scroll.offset().y) - pad).max(0.0);
                let viewport = f32::from(self.scroll.bounds().size.height);
                let viewport = if viewport > 0.0 { viewport } else { 1600.0 };
                let margin = OVERSCAN_ROWS as f32 * LINE_HEIGHT;
                base.flex().flex_col().children(Self::diff_rows(
                    view,
                    scrolled - margin,
                    scrolled + viewport + margin,
                ))
            }
            Some(TabContent::Text(editor)) => {
                let active_tab = self.model.active();
                let hl = active_tab.and_then(|t| self.highlights_for_tab(editor, t));
                let active_problems: Vec<syntax::Diagnostic> = active_tab
                    .and_then(|t| {
                        self.model
                            .problems
                            .files
                            .iter()
                            .find(|f| f.path == t.path)
                            .map(|f| f.items.clone())
                    })
                    .unwrap_or_default();
                let find_matches: &[Range<usize>] =
                    self.model.find.as_ref().map_or(&[], |f| f.matches());
                let find_current = self.model.find.as_ref().and_then(|f| f.current);
                let buf = editor.buffer();
                let total = buf.len_lines();
                let shown = total.min(EDITOR_MAX_LINES);
                let viewport = f32::from(self.scroll.bounds().size.height);
                let viewport = if viewport > 0.0 { viewport } else { 1600.0 };
                let scrolled = (-f32::from(self.scroll.offset().y) - EDITOR_PAD_Y).max(0.0);
                let first = ((scrolled / LINE_HEIGHT) as usize)
                    .saturating_sub(OVERSCAN_ROWS)
                    .min(shown);
                let last = (((scrolled + viewport) / LINE_HEIGHT).ceil() as usize + OVERSCAN_ROWS)
                    .min(shown)
                    .max(first);
                let sel = editor.selection();
                let (cursor_line, cursor_col) = editor.line_col();
                let focused = self.focus.is_focused(window);
                let rows: Vec<AnyElement> = (first..last)
                    .map(|n| {
                        let text = buf.line_text(n);
                        let (ls, le) = (buf.line_start(n), buf.line_end(n));
                        let line_num = n + 1;
                        let has_error = active_problems.iter().any(|d| d.line == n);
                        let is_active_line = focused && n == cursor_line;
                        let mut row = div()
                            .relative()
                            .flex()
                            .flex_none()
                            .h(px(LINE_HEIGHT))
                            .items_center()
                            .when(is_active_line, |d| d.bg(rgba(0xffffff0a)))
                            .child(
                                div()
                                    .w(px(GUTTER_WIDTH))
                                    .flex_none()
                                    .pr_3()
                                    .flex()
                                    .justify_end()
                                    .text_color(rgb(if has_error {
                                        0xe5534b
                                    } else if is_active_line {
                                        theme::LINE_NUMBER_ACTIVE
                                    } else {
                                        theme::LINE_NUMBER
                                    }))
                                    .child(SharedString::from(line_num.to_string())),
                            )
                            .child(div().whitespace_nowrap().child(line_element(
                                &text,
                                hl.as_ref().and_then(|h| h.get(n)).map(|v| v.as_slice()),
                            )));
                        let first_match = find_matches.partition_point(|m| m.end <= ls);
                        for (k, m) in find_matches.iter().enumerate().skip(first_match) {
                            if m.start > le {
                                break;
                            }
                            let x1 = x_of_col(window, &text, m.start.max(ls) - ls);
                            let x2 = x_of_col(window, &text, m.end.min(le) - ls);
                            if x2 > x1 {
                                let color = if find_current == Some(k) {
                                    theme::FIND_CURRENT
                                } else {
                                    theme::FIND_MATCH
                                };
                                row = row.child(
                                    div()
                                        .absolute()
                                        .top_0()
                                        .left(px(GUTTER_WIDTH) + x1)
                                        .w(x2 - x1)
                                        .h(px(LINE_HEIGHT))
                                        .bg(rgba(color)),
                                );
                            }
                        }
                        for d in active_problems.iter().filter(|d| d.line == n) {
                            let col = d.col.min(text.chars().count());
                            let len = d.len.max(1);
                            let x1 = x_of_col(window, &text, col);
                            let x2 = x_of_col(window, &text, (col + len).min(text.chars().count()))
                                .max(x1 + px(8.));
                            row = row.child(
                                div()
                                    .absolute()
                                    .top(px(LINE_HEIGHT - 2.))
                                    .left(px(GUTTER_WIDTH) + x1)
                                    .w(x2 - x1)
                                    .h(px(2.))
                                    .bg(rgb(0xe5534b)),
                            );
                        }
                        if editor.has_selection() && sel.end >= ls && sel.start <= le {
                            let from = sel.start.max(ls) - ls;
                            let to = sel.end.min(le) - ls;
                            let eol_selected = sel.end > le && sel.start <= le && n + 1 < total;
                            let x1 = x_of_col(window, &text, from);
                            let x2 = x_of_col(window, &text, to)
                                + px(if eol_selected { 8. } else { 0. });
                            if x2 > x1 {
                                row = row.child(
                                    div()
                                        .absolute()
                                        .top_0()
                                        .left(px(GUTTER_WIDTH) + x1)
                                        .w(x2 - x1)
                                        .h(px(LINE_HEIGHT))
                                        .bg(rgba(theme::SELECTION)),
                                );
                            }
                        }
                        if let Some(marked) = editor.marked_range()
                            && marked.end > marked.start
                            && marked.end >= ls
                            && marked.start <= le
                        {
                            let x1 = x_of_col(window, &text, marked.start.max(ls) - ls);
                            let x2 = x_of_col(window, &text, marked.end.min(le) - ls);
                            row = row.child(
                                div()
                                    .absolute()
                                    .top(px(LINE_HEIGHT - 3.))
                                    .left(px(GUTTER_WIDTH) + x1)
                                    .w(x2 - x1)
                                    .h(px(1.))
                                    .bg(rgb(theme::TEXT)),
                            );
                        }
                        if focused && n == cursor_line {
                            row = row.child(
                                div()
                                    .absolute()
                                    .top(px(2.))
                                    .left(px(GUTTER_WIDTH) + x_of_col(window, &text, cursor_col))
                                    .w(px(2.))
                                    .h(px(LINE_HEIGHT - 4.))
                                    .bg(rgb(theme::TEXT)),
                            );
                        }
                        row.into_any_element()
                    })
                    .collect();
                let note =
                    (total > shown).then(|| {
                        div().px_4().py_2().text_color(rgb(theme::TEXT_DIM)).child(
                            SharedString::from(format!(
                                "… {} more lines not shown (viewer limit {EDITOR_MAX_LINES})",
                                total - shown
                            )),
                        )
                    });
                base.child(div().h(px(first as f32 * LINE_HEIGHT)).flex_none())
                    .children(rows)
                    .child(div().h(px((shown - last) as f32 * LINE_HEIGHT)).flex_none())
                    .children(note)
            }
        };
        let focus = self.focus.clone();
        let entity = cx.entity();
        let input = has_text.then(|| {
            canvas(
                |_, _, _| (),
                move |bounds, (), window, cx| {
                    window.handle_input(&focus, ElementInputHandler::new(bounds, entity), cx);
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        });
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .child(inner)
            .children(input)
            .child(self.scrollbar("editor", &self.scroll, cx))
    }

    fn bottom_panel(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let vh = f32::from(window.viewport_size().height);
        let status = if self.model.layout.status_bar {
            STATUS_BAR_HEIGHT
        } else {
            0.0
        };
        let layout = &self.model.layout;
        let panel_h = layout.effective_panel(vh - status - TAB_BAR_HEIGHT - MENU_BAR_HEIGHT);
        let panel_w = layout.effective_panel_width(f32::from(window.viewport_size().width));
        let (maximized, pos) = (layout.panel_maximized, layout.panel_pos);
        let body: AnyElement = self.panel_body(window, cx);
        div()
            .flex()
            .flex_col()
            .when(maximized, |d| d.flex_1().min_h_0().min_w_0())
            .when(!maximized && !pos.is_side(), |d| {
                d.flex_none().h(px(panel_h))
            })
            .when(!maximized && pos.is_side(), |d| {
                d.flex_none().h_full().w(px(panel_w))
            })
            .bg(rgb(theme::PANEL))
            .when(pos == PanelPos::Bottom, |d| d.border_t_1())
            .when(pos == PanelPos::Top, |d| d.border_b_1())
            .when(pos == PanelPos::Left, |d| d.border_r_1())
            .when(pos == PanelPos::Right, |d| d.border_l_1())
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .gap_1()
                    .px_2()
                    .py_1()
                    .children(PanelTab::ALL.into_iter().enumerate().map(|(i, t)| {
                        let selected = t == self.model.panel_tab;
                        div()
                            .id(("panel-tab", i))
                            .px_2()
                            .py(px(2.))
                            .rounded_sm()
                            .text_size(px(11.))
                            .cursor_pointer()
                            .text_color(rgb(if selected {
                                theme::TEXT
                            } else {
                                theme::TEXT_DIM
                            }))
                            .when(selected, |d| {
                                d.bg(rgb(theme::SELECTED))
                                    .border_b_1()
                                    .border_color(rgb(theme::ACCENT))
                            })
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                if t == PanelTab::Terminal {
                                    this.open_terminal_tab(window);
                                } else {
                                    this.model.panel_tab = t;
                                    this.model.terminal_focused = false;
                                    this.sync_focus(window);
                                }
                                cx.notify();
                            }))
                            .child(t.title())
                    }))
                    .child(div().flex_1())
                    .when(self.model.panel_tab == PanelTab::Terminal, |d| {
                        d.child(
                            Self::panel_button("panel-new-term", "plus").on_click(cx.listener(
                                |this, _: &ClickEvent, window, cx| {
                                    this.apply_command(Command::NewTerminal, window, cx);
                                },
                            )),
                        )
                        .child(
                            Self::panel_button("panel-kill-term", "trash").on_click(cx.listener(
                                |this, _: &ClickEvent, window, cx| {
                                    this.apply_command(Command::KillTerminal, window, cx);
                                },
                            )),
                        )
                    })
                    .child(
                        Self::panel_button(
                            "panel-maximize",
                            if maximized { "restore" } else { "maximize" },
                        )
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::MaximizePanel, window, cx);
                            },
                        )),
                    )
                    .child(
                        Self::panel_button("panel-hide", "close").on_click(cx.listener(
                            |this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::TogglePanel, window, cx);
                            },
                        )),
                    ),
            )
            .child(body)
    }

    fn file_icon(name: &str, size: f32) -> AnyElement {
        let color = assets::file_color(name);
        match assets::file_badge(name) {
            Some(badge) => div()
                .flex_none()
                .w(px(size + 4.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(if badge.len() > 2 { 8. } else { 10. }))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(color))
                .child(badge)
                .into_any_element(),
            None => assets::icon("file", size, color).into_any_element(),
        }
    }

    fn extension_icon(&self, size: f32) -> AnyElement {
        let icon = self.model.panel_icon.clone().or_else(|| {
            self.model
                .panel_extension
                .as_deref()
                .and_then(|dir| self.model.extensions.iter().find(|e| e.dir == dir))
                .and_then(|e| e.icon.clone())
        });
        match icon {
            Some(path) => gpui::img(path)
                .flex_none()
                .size(px(size))
                .into_any_element(),
            None => assets::icon("extensions", size, theme::ICON).into_any_element(),
        }
    }

    fn panel_button(id: &'static str, icon_name: &'static str) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .w(px(28.))
            .h(px(24.))
            .rounded_sm()
            .cursor_pointer()
            .hover(|s| s.bg(rgb(theme::HOVER)))
            .child(assets::icon(icon_name, 17., theme::ICON))
    }

    fn layout_button(
        id: &'static str,
        icon_name: &'static str,
        on: bool,
        command: Command,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .w(px(26.))
            .h(px(20.))
            .rounded_sm()
            .cursor_pointer()
            .when(on, |d| d.bg(rgb(theme::SELECTED)))
            .hover(|s| s.bg(rgb(theme::HOVER_STRONG)))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.apply_command(command, window, cx);
            }))
            .child(assets::icon(
                if on {
                    match icon_name {
                        "layout-sidebar" => "layout-sidebar-on",
                        "layout-panel" => "layout-panel-on",
                        other => other,
                    }
                } else {
                    icon_name
                },
                17.,
                if on { 0xe6e6e6 } else { theme::ICON },
            ))
    }

    fn status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let branch = self.model.branch_label();
        let problems = self.model.problems.total();
        let (position, eol, lang) = match self.model.active() {
            Some(t) => {
                let (pos, eol) = match &t.content {
                    TabContent::Text(e) => {
                        let (l, c) = e.line_col();
                        (
                            Some(format!("Ln {}, Col {}", l + 1, c + 1)),
                            Some(if e.eol().as_str() == "\r\n" {
                                "CRLF"
                            } else {
                                "LF"
                            }),
                        )
                    }
                    TabContent::Unavailable(_) | TabContent::Diff(_) | TabContent::Webview => {
                        (None, None)
                    }
                };
                let lang =
                    (!matches!(t.content, TabContent::Webview)).then(|| model::tab_language(t));
                (pos, eol, lang)
            }
            None => (None, None, None),
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .h(px(STATUS_BAR_HEIGHT))
            .px_3()
            .bg(rgb(theme::STATUS_BAR))
            .border_t_1()
            .border_color(rgb(theme::BORDER))
            .text_size(px(12.))
            .text_color(rgb(theme::TEXT))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(
                        div()
                            .id("status-branch")
                            .px_2()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::SwitchBranch, window, cx);
                            }))
                            .child(SharedString::from(branch)),
                    )
                    .child(
                        div()
                            .id("status-problems")
                            .px_2()
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_color(rgb(if problems > 0 {
                                0xe5534b
                            } else {
                                theme::TEXT_DIM
                            }))
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::ShowProblems, window, cx);
                            }))
                            .child(assets::icon(
                                "error",
                                13.,
                                if problems > 0 {
                                    0xe5534b
                                } else {
                                    theme::TEXT_DIM
                                },
                            ))
                            .child(SharedString::from(problems.to_string())),
                    )
                    .child(
                        div()
                            .id("status-commands")
                            .px_2()
                            .cursor_pointer()
                            .text_color(rgb(theme::TEXT_DIM))
                            .hover(|s| s.bg(rgb(theme::HOVER)).text_color(rgb(theme::TEXT)))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::CommandPalette, window, cx);
                            }))
                            .child("Commands (Ctrl+Shift+P)"),
                    )
                    .children(self.model.message.clone().map(|m| {
                        div()
                            .text_color(rgb(theme::TEXT_DIM))
                            .child(SharedString::from(m))
                    })),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .children(self.status_items.iter().map(|(id, t)| {
                        div()
                            .id(SharedString::from(format!("status-item-{id}")))
                            .px_2()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::OpenExtensionPanel, window, cx);
                            }))
                            .child(SharedString::from(t.clone()))
                    }))
                    .children(position.map(|p| {
                        div()
                            .id("status-position")
                            .px_2()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::GoToLine, window, cx);
                            }))
                            .child(SharedString::from(p))
                    }))
                    .children(lang.as_ref().map(|_| "UTF-8"))
                    .children(eol)
                    .children(lang.map(|l| {
                        div()
                            .id("status-language")
                            .px_2()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::ChangeLanguageMode, window, cx);
                            }))
                            .child(SharedString::from(l))
                    }))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(Self::layout_button(
                                "toggle-sidebar",
                                "layout-sidebar",
                                self.model.layout.sidebar,
                                Command::ToggleSidebar,
                                cx,
                            ))
                            .child(Self::layout_button(
                                "toggle-panel",
                                "layout-panel",
                                self.model.layout.panel,
                                Command::TogglePanel,
                                cx,
                            )),
                    ),
            )
    }
}

impl Workbench {
    #[doc(hidden)]
    pub fn palette_top_for_tests(&self) -> usize {
        self.palette_top.get()
    }

    pub fn active_highlights(&self) -> Option<Rc<Vec<Vec<LineSpan>>>> {
        let tab = self.model.active()?;
        match &tab.content {
            TabContent::Text(e) => self.highlights_for_tab(e, tab),
            TabContent::Unavailable(_) | TabContent::Diff(_) | TabContent::Webview => None,
        }
    }

    fn highlights_for_tab(&self, editor: &Editor, tab: &Tab) -> Option<Spans> {
        let lang = tab
            .language_override
            .as_deref()
            .and_then(Language::from_id)
            .or_else(|| Language::from_path(&tab.path))?;
        let version = editor.version();
        self.poll_highlight_job();
        let mut st = self.hl.borrow_mut();
        if let Some((p, v, spans)) = &st.exact
            && p == &tab.path
            && *v == version
        {
            return spans.clone();
        }
        if let Some((p, v, spans)) = st
            .ready
            .take_if(|(p, v, _)| p == &tab.path && *v == version)
        {
            let spans = spans.map(Rc::new);
            if let Some(s) = &spans {
                st.last_good = Some((p.clone(), s.clone()));
            }
            st.exact = Some((p, v, spans.clone()));
            return spans;
        }
        let text = editor.text();
        if text.split('\n').count() != editor.buffer().len_lines() {
            st.exact = Some((tab.path.clone(), version, None));
            return None;
        }
        if text.len() <= HIGHLIGHT_SYNC_BYTES {
            let spans = self
                .highlighter
                .lock()
                .ok()
                .and_then(|mut h| h.highlight(lang, &text))
                .map(Rc::new);
            if let Some(s) = &spans {
                st.last_good = Some((tab.path.clone(), s.clone()));
            }
            st.exact = Some((tab.path.clone(), version, spans.clone()));
            return spans;
        }
        if st.job.is_none() {
            let (tx, rx) = std::sync::mpsc::channel();
            let highlighter = self.highlighter.clone();
            let path = tab.path.clone();
            std::thread::spawn(move || {
                let spans = highlighter
                    .lock()
                    .ok()
                    .and_then(|mut h| h.highlight(lang, &text));
                let _ = tx.send((path, version, spans));
            });
            st.job = Some(HlJob { rx });
        }
        let stale = st
            .last_good
            .as_ref()
            .filter(|(p, _)| p == &tab.path)
            .map(|(_, spans)| spans.clone())?;
        if let Some((p, v, spans)) = &st.provisional
            && p == &tab.path
            && *v == version
        {
            return spans.clone();
        }
        let new_lines = editor.buffer().len_lines();
        let adjusted = if stale.len() == new_lines {
            stale
        } else {
            Rc::new(adjust_spans(&stale, new_lines, editor.line_col().0))
        };
        st.provisional = Some((tab.path.clone(), version, Some(adjusted.clone())));
        Some(adjusted)
    }

    fn poll_highlight_job(&self) -> bool {
        let mut st = self.hl.borrow_mut();
        let Some(job) = &st.job else {
            return false;
        };
        match job.rx.try_recv() {
            Ok(result) => {
                st.job = None;
                if let (path, _, Some(spans)) = &result {
                    st.last_good = Some((path.clone(), Rc::new(spans.clone())));
                }
                st.ready = Some(result);
                true
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                st.job = None;
                true
            }
        }
    }

    pub fn pump_highlight(&mut self) -> bool {
        self.poll_highlight_job()
    }

    #[allow(clippy::too_many_arguments)]
    fn field_widget(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
        id: &'static str,
        field: &TextField,
        focused: bool,
        placeholder: &'static str,
        handle: &FocusHandle,
        target: Target,
    ) -> impl IntoElement + use<> {
        let text = field.text().to_owned();
        let caret_x = x_of_col(window, &text, field.cursor());
        let underline = field.marked_range().filter(|r| r.end > r.start).map(|r| {
            (
                x_of_col(window, &text, r.start),
                x_of_col(window, &text, r.end),
            )
        });
        let focus = handle.clone();
        let entity = cx.entity();
        let input_canvas = canvas(
            |_, _, _| (),
            move |bounds, (), window, cx| {
                window.handle_input(&focus, ElementInputHandler::new(bounds, entity), cx);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        div()
            .id(id)
            .track_focus(handle)
            .relative()
            .flex_1()
            .min_w_0()
            .h(px(28.))
            .px(px(FIELD_PAD))
            .flex()
            .items_center()
            .bg(rgb(theme::EDITOR))
            .border_1()
            .border_color(rgb(if focused { 0x8c8c8c } else { theme::BORDER }))
            .rounded_md()
            .font_family(FONT_FAMILY)
            .text_size(px(FONT_SIZE))
            .text_color(rgb(theme::TEXT))
            .cursor_text()
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    this.focus_target(target, window);
                    cx.notify();
                }),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.focus_target(target, window);
                cx.notify();
            }))
            .child(if text.is_empty() {
                div().text_color(rgb(theme::TEXT_DIM)).child(placeholder)
            } else {
                div().whitespace_nowrap().child(SharedString::from(text))
            })
            .children(focused.then(|| {
                div()
                    .absolute()
                    .top(px(4.))
                    .left(px(FIELD_PAD) + caret_x)
                    .w(px(2.))
                    .h(px(18.))
                    .bg(rgb(theme::TEXT))
            }))
            .children(underline.map(|(x1, x2)| {
                div()
                    .absolute()
                    .top(px(22.))
                    .left(px(FIELD_PAD) + x1)
                    .w(x2 - x1)
                    .h(px(1.))
                    .bg(rgb(theme::TEXT))
            }))
            .child(input_canvas)
    }

    fn toggle_button(id: &'static str, label: &'static str, on: bool) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .px_2()
            .py(px(2.))
            .rounded_sm()
            .cursor_pointer()
            .text_size(px(12.))
            .when(on, |d| d.bg(rgb(theme::SELECTED)).text_color(rgb(0xffffff)))
            .when(!on, |d| {
                d.text_color(rgb(theme::TEXT_DIM))
                    .hover(|s| s.bg(rgb(theme::HOVER)))
            })
            .child(label)
    }

    fn find_bar(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let f = self.model.find.as_ref()?;
        let status = f.status();
        let error = f.error.is_some();
        let query = self.field_widget(
            window,
            cx,
            "find-query",
            &f.query,
            f.focus == FindFocus::Query,
            "Find",
            &self.find_query_focus,
            Target::FindQuery,
        );
        let (case, word, regex) = (f.opts.case_sensitive, f.opts.whole_word, f.opts.regex);
        let row1 = div()
            .flex()
            .items_center()
            .gap_1()
            .child(query)
            .child(
                Self::toggle_button("find-case", "Aa", case).on_click(cx.listener(
                    |this, _: &ClickEvent, _w, cx| {
                        if let Some(f) = this.model.find.as_mut() {
                            f.toggle_case();
                        }
                        this.model.find_select_nearest();
                        cx.notify();
                    },
                )),
            )
            .child(
                Self::toggle_button("find-word", "W", word).on_click(cx.listener(
                    |this, _: &ClickEvent, _w, cx| {
                        if let Some(f) = this.model.find.as_mut() {
                            f.toggle_whole_word();
                        }
                        this.model.find_select_nearest();
                        cx.notify();
                    },
                )),
            )
            .child(
                Self::toggle_button("find-regex", ".*", regex).on_click(cx.listener(
                    |this, _: &ClickEvent, _w, cx| {
                        if let Some(f) = this.model.find.as_mut() {
                            f.toggle_regex();
                        }
                        this.model.find_select_nearest();
                        cx.notify();
                    },
                )),
            )
            .child(
                div()
                    .flex_none()
                    .min_w(px(70.))
                    .px_2()
                    .text_size(px(12.))
                    .text_color(rgb(if error { theme::WARN } else { theme::TEXT_DIM }))
                    .child(SharedString::from(status)),
            )
            .child(
                Self::toggle_button("find-prev", "↑", false).on_click(cx.listener(
                    |this, _: &ClickEvent, _w, cx| {
                        this.model.find_prev();
                        this.reveal_cursor();
                        cx.notify();
                    },
                )),
            )
            .child(
                Self::toggle_button("find-next", "↓", false).on_click(cx.listener(
                    |this, _: &ClickEvent, _w, cx| {
                        this.model.find_next();
                        this.reveal_cursor();
                        cx.notify();
                    },
                )),
            )
            .child(
                Self::toggle_button("find-close", "×", false).on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| {
                        this.model.close_find();
                        this.sync_focus(window);
                        cx.notify();
                    },
                )),
            );
        let row2 = f.show_replace.then(|| {
            let replace = self.field_widget(
                window,
                cx,
                "find-replace",
                &f.replace,
                f.focus == FindFocus::Replace,
                "Replace",
                &self.find_replace_focus,
                Target::FindReplace,
            );
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(replace)
                .child(
                    Self::toggle_button("replace-one", "Replace", false).on_click(cx.listener(
                        |this, _: &ClickEvent, _w, cx| {
                            this.model.find_replace_current();
                            this.reveal_cursor();
                            cx.notify();
                        },
                    )),
                )
                .child(
                    Self::toggle_button("replace-all", "All", false).on_click(cx.listener(
                        |this, _: &ClickEvent, _w, cx| {
                            this.model.find_replace_all();
                            cx.notify();
                        },
                    )),
                )
        });
        Some(
            div()
                .flex()
                .flex_col()
                .flex_none()
                .gap_1()
                .px_2()
                .py(px(4.))
                .bg(rgb(theme::SIDEBAR))
                .border_b_1()
                .border_color(rgb(theme::BORDER))
                .child(row1)
                .children(row2),
        )
    }

    fn search_panel(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let s = &self.model.search;
        let field = self.field_widget(
            window,
            cx,
            "search-query",
            &s.query,
            s.focused,
            "Search (Enter)",
            &self.search_focus,
            Target::Search,
        );
        let summary = match &s.results {
            None if s.running => "Searching...".to_owned(),
            None => "Type and press Enter to search the workspace.".to_owned(),
            Some(r) if r.error.is_some() => r.error.clone().unwrap_or_default(),
            Some(r) if r.total_hits == 0 => {
                format!("No results ({} files searched).", r.files_scanned)
            }
            Some(r) => format!(
                "{} result{} in {} file{}{}",
                r.total_hits,
                if r.total_hits == 1 { "" } else { "s" },
                r.files.len(),
                if r.files.len() == 1 { "" } else { "s" },
                if r.truncated {
                    " (limit reached, refine the query)"
                } else {
                    ""
                },
            ),
        };
        let summary = if s.running && s.results.is_some() {
            "Searching...".to_owned()
        } else {
            summary
        };
        // Up to 2000 hits can come back; only the rows near the viewport become elements.
        let scrolled = (-f32::from(self.scrolls.search.offset().y)).max(0.0);
        let viewport = f32::from(self.scrolls.search.bounds().size.height);
        let viewport = if viewport > 0.0 { viewport } else { 1600.0 };
        let margin = OVERSCAN_ROWS as f32 * SCM_ROW_H;
        let mut w = RowWindow::new(scrolled - margin, scrolled + viewport + margin);
        let mut hit_id = 0usize;
        if let Some(r) = &s.results {
            for (fi, file) in r.files.iter().enumerate() {
                let collapsed = s.collapsed.contains(&file.path);
                let path = file.path.clone();
                w.push(SCM_ROW_H, || {
                    div()
                        .id(("search-file", fi))
                        .flex()
                        .flex_none()
                        .items_center()
                        .h(px(SCM_ROW_H))
                        .gap_1()
                        .px_2()
                        .text_size(px(12.))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(theme::HOVER)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.model.search.toggle_file(&path);
                            cx.notify();
                        }))
                        .child(if collapsed { "▸" } else { "▾" })
                        .child(SharedString::from(file.rel.clone()))
                        .child(
                            div()
                                .text_color(rgb(theme::TEXT_DIM))
                                .child(SharedString::from(format!("({})", file.hits.len()))),
                        )
                        .into_any_element()
                });
                if collapsed {
                    continue;
                }
                for hit in &file.hits {
                    let (path, line, col, len) = (file.path.clone(), hit.line, hit.col, hit.len);
                    let bytes = char_range_to_bytes(&hit.preview, hit.preview_range.clone());
                    let highlight = HighlightStyle {
                        background_color: Some(rgba(theme::FIND_CURRENT).into()),
                        ..Default::default()
                    };
                    let id = hit_id;
                    w.push(SCM_ROW_H, || {
                        div()
                            .id(("search-hit", id))
                            .flex()
                            .flex_none()
                            .items_center()
                            .h(px(SCM_ROW_H))
                            .gap_2()
                            .pl_5()
                            .pr_2()
                            .text_size(px(12.))
                            .overflow_hidden()
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.model.open_at(&path, line, col, len);
                                this.focus_editor(window);
                                this.reveal_cursor();
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .flex_none()
                                    .text_color(rgb(theme::LINE_NUMBER))
                                    .child(SharedString::from((hit.line + 1).to_string())),
                            )
                            .child(
                                div().whitespace_nowrap().child(
                                    StyledText::new(SharedString::from(hit.preview.clone()))
                                        .with_highlights([(bytes, highlight)]),
                                ),
                            )
                            .into_any_element()
                    });
                    hit_id += 1;
                }
            }
        }
        let rows = w.finish();
        let opts = s.opts;
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_1()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_none()
                    .gap_1()
                    .px_2()
                    .child(field)
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(
                                Self::toggle_button("search-case", "Aa", opts.case_sensitive)
                                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.model.search.opts.case_sensitive ^= true;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Self::toggle_button("search-word", "W", opts.whole_word).on_click(
                                    cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.model.search.opts.whole_word ^= true;
                                        cx.notify();
                                    }),
                                ),
                            )
                            .child(
                                Self::toggle_button("search-regex", ".*", opts.regex).on_click(
                                    cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.model.search.opts.regex ^= true;
                                        cx.notify();
                                    }),
                                ),
                            )
                            .child(Self::toggle_button("search-run", "Search", false).on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| {
                                    this.model.run_search();
                                    cx.notify();
                                }),
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(theme::TEXT_DIM))
                            .child(SharedString::from(summary)),
                    ),
            )
            .child(self.scroll_list("search-results", &self.scrolls.search, rows, None, cx))
            .into_any_element()
    }

    fn scm_button(
        id: &'static str,
        label: impl Into<SharedString>,
        enabled: bool,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .h(px(28.))
            .px_3()
            .rounded_md()
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .when(enabled, |d| {
                d.cursor_pointer()
                    .bg(rgb(theme::ACCENT))
                    .text_color(rgb(0x1e1e1e))
                    .hover(|s| s.bg(rgb(0xffffff)))
            })
            .when(!enabled, |d| {
                d.cursor_default()
                    .border_1()
                    .border_color(rgb(0x2f2f2f))
                    .bg(rgb(0x202020))
                    .text_color(rgb(0x5e5e5e))
                    .font_weight(FontWeight::NORMAL)
            })
            .child(label.into())
    }

    fn scm_panel(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let s = &self.model.scm;
        let Some(status) = &s.status else {
            let text = match (&s.error, s.is_enabled()) {
                (Some(e), _) => e.clone(),
                (None, true) => "Reading the repository status...".to_owned(),
                (None, false) => "Open a folder to use source control.".to_owned(),
            };
            return div()
                .flex()
                .flex_col()
                .items_start()
                .gap_2()
                .px_3()
                .text_size(px(12.))
                .text_color(rgb(theme::TEXT_DIM))
                .child(SharedString::from(text))
                .child(
                    Self::scm_button("scm-refresh", "Refresh", false).on_click(cx.listener(
                        |this, _: &ClickEvent, _w, cx| {
                            this.model.scm.refresh();
                            cx.notify();
                        },
                    )),
                )
                .into_any_element();
        };
        let field = self.field_widget(
            window,
            cx,
            "commit-message",
            &s.message,
            s.focused,
            "Message (Ctrl+Enter to commit)",
            &self.commit_focus,
            Target::Commit,
        );
        let staged = s.staged_count();
        let branch_line = match (&status.branch, &status.upstream) {
            (Some(b), Some(u)) => format!("{b} → {u}   ↑{} ↓{}", status.ahead, status.behind),
            (Some(b), None) => format!("{b} (no upstream)"),
            (None, _) => "HEAD is detached".to_owned(),
        };
        let mut body: Vec<AnyElement> = Vec::new();
        let mut rid = 0usize;
        let mut y = 0f32;
        let scrolled = (-f32::from(self.scrolls.scm.offset().y)).max(0.0);
        let viewport = f32::from(self.scrolls.scm.bounds().size.height);
        let viewport = if viewport > 0.0 { viewport } else { 1600.0 };
        for kind in [
            scm::GroupKind::Conflicts,
            scm::GroupKind::Staged,
            scm::GroupKind::Changes,
        ] {
            let total = s.group_len(kind);
            if total == 0 {
                continue;
            }
            let collapsed = s.collapsed.contains(&kind);
            let bulk: Option<(&'static str, &'static str)> = match kind {
                scm::GroupKind::Staged => Some(("scm-unstage-all", "minimize")),
                scm::GroupKind::Changes => Some(("scm-stage-all", "plus")),
                scm::GroupKind::Conflicts => None,
            };
            body.push(
                div()
                    .id(("scm-group", kind as usize))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .h(px(24.))
                    .text_size(px(11.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(theme::ICON))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::HOVER)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.model.scm.toggle_group(kind);
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(assets::icon(
                                if collapsed {
                                    "chevron-right"
                                } else {
                                    "chevron-down"
                                },
                                14.,
                                theme::TEXT_DIM,
                            ))
                            .child(SharedString::from(kind.title()))
                            .child(
                                div()
                                    .ml_1()
                                    .px(px(6.))
                                    .rounded_full()
                                    .bg(rgb(0x2f2f2f))
                                    .font_weight(FontWeight::NORMAL)
                                    .text_color(rgb(theme::TEXT_DIM))
                                    .child(SharedString::from(total.to_string())),
                            ),
                    )
                    .children(bulk.map(|(id, icon_name)| {
                        div()
                            .id(id)
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(20.))
                            .h(px(20.))
                            .rounded_sm()
                            .hover(|s| s.bg(rgb(theme::SELECTED)))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                cx.stop_propagation();
                                if kind == scm::GroupKind::Staged {
                                    this.model.scm.unstage_all();
                                } else {
                                    this.model.scm.stage_all();
                                }
                                cx.notify();
                            }))
                            .child(assets::icon(icon_name, 14., theme::ICON))
                    }))
                    .into_any_element(),
            );
            y += SCM_HEADER_H;
            if collapsed {
                continue;
            }
            // Only the rows near the viewport become elements, so thousands of changes cost the same as a few.
            let first = (((scrolled - y) / SCM_ROW_H).max(0.0) as usize)
                .saturating_sub(OVERSCAN_ROWS)
                .min(total);
            let last = ((((scrolled + viewport - y) / SCM_ROW_H).max(0.0)).ceil() as usize
                + OVERSCAN_ROWS)
                .min(total)
                .max(first);
            let rows = s.group_window(kind, first, last);
            let base = rid;
            rid += total;
            y += total as f32 * SCM_ROW_H;
            if first > 0 {
                body.push(
                    div()
                        .h(px(first as f32 * SCM_ROW_H))
                        .flex_none()
                        .into_any_element(),
                );
            }
            for (k, row) in rows.iter().enumerate() {
                let rid = base + first + k;
                let (name, dir) = row.name_and_dir();
                let (name, dir) = (name.to_owned(), dir.to_owned());
                let (rel_open, rel_act) = (row.rel.clone(), row.rel.clone());
                let staged_group = kind == scm::GroupKind::Staged;
                let color = tone_color(row.tone);
                let act = if staged_group { "−" } else { "+" };
                body.push(
                    div()
                        .id(("scm-row", rid))
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_between()
                        .h(px(SCM_ROW_H))
                        .pl_4()
                        .pr_2()
                        .text_size(px(13.))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(theme::HOVER)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.model.open_diff(&rel_open, staged_group);
                            this.focus_editor(window);
                            cx.notify();
                        }))
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .min_w_0()
                                .overflow_hidden()
                                .child(
                                    div()
                                        .flex_none()
                                        .text_color(rgb(color))
                                        .child(SharedString::from(name)),
                                )
                                .child(
                                    div()
                                        .whitespace_nowrap()
                                        .text_size(px(11.))
                                        .text_color(rgb(theme::TEXT_DIM))
                                        .child(SharedString::from(dir)),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_none()
                                .items_center()
                                .gap_2()
                                .children((kind == scm::GroupKind::Changes).then(|| {
                                    let discard_rel = row.rel.clone();
                                    let untracked = row.untracked;
                                    div()
                                        .id(("scm-discard", rid))
                                        .px_2()
                                        .rounded_sm()
                                        .text_size(px(14.))
                                        .text_color(rgb(theme::TEXT))
                                        .hover(|s| s.bg(rgb(theme::SELECTED)))
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, _w, cx| {
                                                cx.stop_propagation();
                                                this.model.confirm_discard(&discard_rel, untracked);
                                                cx.notify();
                                            },
                                        ))
                                        .child("\u{21b6}")
                                }))
                                .child(
                                    div()
                                        .id(("scm-act", rid))
                                        .px_2()
                                        .rounded_sm()
                                        .text_size(px(14.))
                                        .text_color(rgb(theme::TEXT))
                                        .hover(|s| s.bg(rgb(theme::SELECTED)))
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, _w, cx| {
                                                cx.stop_propagation();
                                                if staged_group {
                                                    this.model.scm.unstage(&rel_act);
                                                } else {
                                                    this.model.scm.stage(&rel_act);
                                                }
                                                cx.notify();
                                            },
                                        ))
                                        .child(act),
                                )
                                .child(
                                    div()
                                        .w(px(12.))
                                        .text_size(px(11.))
                                        .text_color(rgb(color))
                                        .child(SharedString::from(row.letter.to_string())),
                                ),
                        )
                        .into_any_element(),
                );
            }
            if last < total {
                body.push(
                    div()
                        .h(px((total - last) as f32 * SCM_ROW_H))
                        .flex_none()
                        .into_any_element(),
                );
            }
        }
        if body.is_empty() {
            body.push(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("No changes.")
                    .into_any_element(),
            );
        }
        if status.truncated {
            body.push(
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(11.))
                    .text_color(rgb(theme::WARN))
                    .child(SharedString::from(format!(
                        "Only the first {} changes are shown.",
                        git::MAX_STATUS_ENTRIES
                    )))
                    .into_any_element(),
            );
        }
        self.scm_history_rows(&mut body, y, scrolled, viewport, cx);
        let notice = s.notice.as_ref().map(|(is_err, text)| {
            let shown: Vec<&str> = text.lines().take(12).collect();
            div()
                .flex_none()
                .px_2()
                .py_1()
                .text_size(px(11.))
                .text_color(rgb(if *is_err { 0xe5534b } else { 0x81b88b }))
                .child(SharedString::from(shown.join("\n")))
        });
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .gap_1()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_none()
                    .gap_2()
                    .px_3()
                    .pb_1()
                    .child(field)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Self::scm_button(
                                    "scm-commit",
                                    "Commit",
                                    staged > 0 || s.committable_changes() > 0,
                                )
                                .flex_1()
                                .on_click(cx.listener(
                                    |this, _: &ClickEvent, window, cx| {
                                        this.apply_command(Command::GitCommit, window, cx);
                                    },
                                )),
                            )
                            .child(
                                div()
                                    .id("scm-refresh")
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .w(px(28.))
                                    .h(px(28.))
                                    .rounded_md()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(theme::HOVER)))
                                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.model.scm.refresh();
                                        cx.notify();
                                    }))
                                    .child(assets::icon("refresh", 16., theme::ICON)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_size(px(11.))
                            .text_color(rgb(theme::TEXT_DIM))
                            .child(assets::icon("git", 13., theme::TEXT_DIM))
                            .child(SharedString::from(branch_line)),
                    )
                    .child(self.scm_sync_row(cx)),
            )
            .children(notice)
            .child(self.scroll_list("scm-list", &self.scrolls.scm, body, None, cx))
            .into_any_element()
    }

    fn scm_sync_row(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let s = &self.model.scm;
        let (ahead, behind) = s.status.as_ref().map_or((0, 0), |st| (st.ahead, st.behind));
        let busy = s.activity.is_some();
        let label = |base: &str, n: u32, arrow: &str| {
            if n > 0 {
                format!("{base} {arrow}{n}")
            } else {
                base.to_owned()
            }
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        Self::scm_button(
                            "scm-pull",
                            label("Pull", behind, "\u{2193}"),
                            s.can_pull() && !busy,
                        )
                        .flex_1()
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::GitPull, window, cx);
                            },
                        )),
                    )
                    .child(
                        Self::scm_button(
                            "scm-push",
                            label("Push", ahead, "\u{2191}"),
                            s.can_push() && !busy,
                        )
                        .flex_1()
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::GitPush, window, cx);
                            },
                        )),
                    )
                    .child(
                        Self::scm_button("scm-fetch", "Fetch", !busy)
                            .flex_1()
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::GitFetch, window, cx);
                            })),
                    ),
            )
            .children(s.activity.map(|text| {
                div()
                    .text_size(px(11.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child(text)
            }))
    }

    fn scm_history_rows(
        &self,
        body: &mut Vec<AnyElement>,
        y: f32,
        scrolled: f32,
        viewport: f32,
        cx: &mut Context<Self>,
    ) {
        let s = &self.model.scm;
        if s.history.is_empty() && s.history_error.is_none() {
            return;
        }
        let open = s.history_open;
        body.push(
            div()
                .id("scm-history-header")
                .flex()
                .flex_none()
                .items_center()
                .gap_1()
                .px_2()
                .h(px(SCM_HEADER_H))
                .mt_1()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(theme::ICON))
                .cursor_pointer()
                .hover(|s| s.bg(rgb(theme::HOVER)))
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    this.model.scm.history_open = !this.model.scm.history_open;
                    cx.notify();
                }))
                .child(assets::icon(
                    if open {
                        "chevron-down"
                    } else {
                        "chevron-right"
                    },
                    14.,
                    theme::TEXT_DIM,
                ))
                .child("COMMITS")
                .into_any_element(),
        );
        if !open {
            return;
        }
        if let Some(e) = &s.history_error {
            body.push(
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(11.))
                    .text_color(rgb(0xe5534b))
                    .child(SharedString::from(e.clone()))
                    .into_any_element(),
            );
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        // Commits and their files all have a fixed height, so only the rows near the viewport become elements.
        let margin = OVERSCAN_ROWS as f32 * SCM_ROW_H;
        let mut w = RowWindow::new(scrolled - margin, scrolled + viewport + margin);
        w.y = y + HISTORY_HEADER_H;
        let note = |text: SharedString, color: u32| {
            div()
                .flex()
                .flex_none()
                .items_center()
                .h(px(SCM_ROW_H))
                .pl_6()
                .text_size(px(11.))
                .text_color(rgb(color))
                .child(text)
                .into_any_element()
        };
        for (i, c) in s.history.iter().enumerate() {
            let expanded = s.expanded_commits.contains(&c.hash);
            w.push(COMMIT_ROW_H, || {
                let hash = c.hash.clone();
                let hover_hash = c.hash.clone();
                let hover_entity = cx.entity();
                let refs: Vec<AnyElement> = c
                    .refs
                    .iter()
                    .take(3)
                    .map(|r| {
                        let head = r.starts_with("HEAD");
                        let name = r.strip_prefix("HEAD -> ").unwrap_or(r).to_owned();
                        div()
                            .flex_none()
                            .px_1()
                            .rounded_sm()
                            .text_size(px(10.))
                            .bg(rgb(if head { 0x264f78 } else { 0x2f2f2f }))
                            .text_color(rgb(if head { 0xffffff } else { theme::TEXT_DIM }))
                            .child(SharedString::from(name))
                            .into_any_element()
                    })
                    .collect();
                div()
                    .id(("scm-commit-row", i))
                    .flex()
                    .flex_none()
                    .flex_col()
                    .justify_center()
                    .h(px(COMMIT_ROW_H))
                    .gap(px(1.))
                    .px_2()
                    .cursor_pointer()
                    .when(expanded, |d| d.bg(rgb(theme::HOVER)))
                    .hover(|s| s.bg(rgb(theme::HOVER)))
                    .on_hover(move |hovered, window, cx| {
                        let at = window.mouse_position();
                        hover_entity.update(cx, |wb, cx| {
                            if *hovered {
                                wb.hover_commit(&hover_hash, at, cx);
                            } else {
                                wb.leave_commit(&hover_hash, cx);
                            }
                        });
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.model.scm.toggle_commit(&hash);
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .min_w_0()
                            .child(assets::icon(
                                if expanded {
                                    "chevron-down"
                                } else {
                                    "chevron-right"
                                },
                                12.,
                                theme::TEXT_DIM,
                            ))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .text_size(px(13.))
                                    .child(SharedString::from(c.subject.clone())),
                            )
                            .children(refs),
                    )
                    .child(
                        div()
                            .pl(px(16.))
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_size(px(11.))
                            .text_color(rgb(theme::TEXT_DIM))
                            .child(SharedString::from(format!(
                                "{} \u{b7} {} \u{b7} {}",
                                c.short(),
                                c.author,
                                scm::relative_time(now, c.time)
                            ))),
                    )
                    .into_any_element()
            });
            if !expanded {
                continue;
            }
            match s.commit_files.get(&c.hash) {
                None | Some(scm::CommitFiles::Loading) => {
                    w.push(SCM_ROW_H, || {
                        note("Loading files...".into(), theme::TEXT_DIM)
                    });
                }
                Some(scm::CommitFiles::Failed(e)) => {
                    w.push(SCM_ROW_H, || note(SharedString::from(e.clone()), 0xe5534b));
                }
                Some(scm::CommitFiles::Ready(files)) => {
                    for (j, file) in files.iter().enumerate() {
                        w.push(SCM_ROW_H, || {
                            let name = file
                                .path
                                .rsplit('/')
                                .next()
                                .unwrap_or(&file.path)
                                .to_owned();
                            let dir = file
                                .path
                                .rfind('/')
                                .map_or(String::new(), |k| file.path[..k].to_owned());
                            let color = match file.letter {
                                'A' | 'C' | 'R' => 0x81b88b,
                                'D' => 0xc74e39,
                                _ => 0xe2c08d,
                            };
                            let letter = file.letter.to_string();
                            let (hash, file) = (c.hash.clone(), file.clone());
                            div()
                                .id(("scm-commit-file", i * 100_000 + j))
                                .flex()
                                .flex_none()
                                .items_center()
                                .justify_between()
                                .h(px(SCM_ROW_H))
                                .pl_6()
                                .pr_2()
                                .text_size(px(12.))
                                .cursor_pointer()
                                .hover(|s| s.bg(rgb(theme::HOVER)))
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    this.model.open_commit_diff(&hash, &file);
                                    this.focus_editor(window);
                                    cx.notify();
                                }))
                                .child(
                                    div()
                                        .flex()
                                        .gap_2()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .child(
                                            div()
                                                .flex_none()
                                                .text_color(rgb(color))
                                                .child(SharedString::from(name)),
                                        )
                                        .child(
                                            div()
                                                .whitespace_nowrap()
                                                .text_size(px(11.))
                                                .text_color(rgb(theme::TEXT_DIM))
                                                .child(SharedString::from(dir)),
                                        ),
                                )
                                .child(
                                    div()
                                        .w(px(12.))
                                        .text_size(px(11.))
                                        .text_color(rgb(color))
                                        .child(SharedString::from(letter)),
                                )
                                .into_any_element()
                        });
                    }
                }
            }
        }
        // Reaching the end of the loaded commits asks for the next page, so there is no button to hunt for.
        let near_end = w.y < scrolled + viewport + 600.0;
        if s.history_more {
            w.push(SCM_ROW_H, || {
                note(
                    if s.history_loading() {
                        "Loading more commits...".into()
                    } else {
                        "Scroll for more commits".into()
                    },
                    theme::TEXT_DIM,
                )
            });
            if near_end && !s.history_loading() {
                let entity = cx.entity();
                cx.defer(move |cx| {
                    entity.update(cx, |wb, cx| {
                        wb.model.scm.load_more_history();
                        cx.notify();
                    });
                });
            }
        }
        body.extend(w.finish());
    }

    fn diff_rows(view: &scm::DiffView, lo: f32, hi: f32) -> Vec<AnyElement> {
        let note = |text: String, color: u32| {
            div()
                .flex_none()
                .flex()
                .items_center()
                .h(px(DIFF_NOTE_H))
                .px_4()
                .text_size(px(12.))
                .font_family("Segoe UI")
                .text_color(rgb(color))
                .child(SharedString::from(text))
                .into_any_element()
        };
        let d = match &view.state {
            scm::DiffState::Loading => {
                return vec![note("Loading diff...".into(), theme::TEXT_DIM)];
            }
            scm::DiffState::Failed(e) => {
                return vec![note(format!("Could not read the diff: {e}"), 0xe5534b)];
            }
            scm::DiffState::Ready(d) => d,
        };
        // Only the lines near the viewport become elements; every other row is a known height, so one spacer stands in for them.
        let mut w = RowWindow::new(lo, hi);
        w.push(DIFF_NOTE_H, || {
            note(
                format!(
                    "{}  +{} −{}{}",
                    view.req.key.rel,
                    d.added(),
                    d.removed(),
                    if d.notes.is_empty() {
                        String::new()
                    } else {
                        format!("  ({})", d.notes.join(", "))
                    }
                ),
                theme::TEXT,
            )
        });
        if d.binary {
            w.push(DIFF_NOTE_H, || {
                note(
                    "Binary file: there is no text diff.".into(),
                    theme::TEXT_DIM,
                )
            });
        } else if d.hunks.is_empty() {
            w.push(DIFF_NOTE_H, || note("No changes.".into(), theme::TEXT_DIM));
        }
        let mut shown = 0usize;
        'hunks: for h in &d.hunks {
            w.push(LINE_HEIGHT, || {
                div()
                    .flex_none()
                    .h(px(LINE_HEIGHT))
                    .flex()
                    .items_center()
                    .px_4()
                    .bg(rgb(0x1f2a38))
                    .text_size(px(12.))
                    .text_color(rgb(0x7aa2d6))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .child(SharedString::from(h.header.clone()))
                    .into_any_element()
            });
            for l in &h.lines {
                if shown >= MAX_RENDERED_LINES {
                    w.push(DIFF_NOTE_H, || {
                        note(
                            format!("Showing the first {MAX_RENDERED_LINES} lines of the diff."),
                            theme::WARN,
                        )
                    });
                    break 'hunks;
                }
                shown += 1;
                let (bg, marker, mcolor) = match l.kind {
                    git::LineKind::Add => (Some(0x1d3326), "+", 0x81b88b),
                    git::LineKind::Del => (Some(0x3a1f22), "−", 0xc74e39),
                    git::LineKind::Context => (None, " ", theme::TEXT_DIM),
                    git::LineKind::NoNewline => (None, " ", theme::TEXT_DIM),
                };
                let num = |n: Option<u32>| {
                    div()
                        .w(px(44.))
                        .flex_none()
                        .pr_2()
                        .flex()
                        .justify_end()
                        .text_color(rgb(theme::LINE_NUMBER))
                        .child(SharedString::from(
                            n.map_or(String::new(), |n| n.to_string()),
                        ))
                };
                w.push(LINE_HEIGHT, || {
                    div()
                        .flex()
                        .flex_none()
                        .h(px(LINE_HEIGHT))
                        .items_center()
                        .when_some(bg, |d, c| d.bg(rgb(c)))
                        .child(num(l.old))
                        .child(num(l.new))
                        .child(
                            div()
                                .w(px(16.))
                                .flex_none()
                                .text_color(rgb(mcolor))
                                .child(marker),
                        )
                        .child(
                            div()
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .when(l.kind == git::LineKind::NoNewline, |d| {
                                    d.text_color(rgb(theme::TEXT_DIM))
                                })
                                .child(SharedString::from(display_text(&l.text))),
                        )
                        .into_any_element()
                });
            }
        }
        if d.truncated {
            w.push(DIFF_NOTE_H, || {
                note(
                    "The diff is larger than the display limit and was cut.".into(),
                    theme::WARN,
                )
            });
        }
        w.finish()
    }

    fn palette_overlay(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let p = self.model.palette.as_ref()?;
        let vw = f32::from(window.viewport_size().width);
        let width = 560.0_f32.min(vw - 40.0).max(200.0);
        let left = ((vw - width) / 2.0).max(0.0);
        let query = p.field.text().to_owned();
        let caret_x = x_of_col(window, &query, p.field.cursor());
        let underline = p.field.marked_range().filter(|r| r.end > r.start).map(|r| {
            (
                x_of_col(window, &query, r.start),
                x_of_col(window, &query, r.end),
            )
        });
        let focus = self.palette_focus.clone();
        let entity = cx.entity();
        let input_canvas = canvas(
            |_, _, _| (),
            move |bounds, (), window, cx| {
                window.handle_input(&focus, ElementInputHandler::new(bounds, entity), cx);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        let count = p.result_count();
        let max_top = count.saturating_sub(PALETTE_ROWS);
        {
            let mut follow = self.palette_follow.borrow_mut();
            if follow.0 != p.selected || follow.1 != query {
                *follow = (p.selected, query.clone());
                let top = self.palette_top.get();
                if p.selected < top {
                    self.palette_top.set(p.selected);
                } else if p.selected >= top + PALETTE_ROWS {
                    self.palette_top.set(p.selected + 1 - PALETTE_ROWS);
                }
            }
        }
        let start = self.palette_top.get().min(max_top);
        self.palette_top.set(start);
        let is_lang_mode = matches!(p.mode, palette::PaletteMode::LanguageMode);
        let mut rows: Vec<AnyElement> = Vec::new();
        for (i, item) in p.results().enumerate().skip(start).take(PALETTE_ROWS) {
            let row = div()
                .id(("palette-item", i))
                .flex()
                .items_center()
                .justify_between()
                .gap_4()
                .px_3()
                .py(px(4.))
                .text_size(px(13.))
                .cursor_pointer()
                .when(i == p.selected, |d| {
                    d.bg(rgb(theme::SELECTED)).text_color(rgb(0xffffff))
                })
                .hover(|s| s.bg(rgb(theme::HOVER)))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    if let Some(p) = this.model.palette.as_mut() {
                        p.select(i);
                    }
                    this.accept_palette(window, cx);
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .min_w_0()
                        .children(item.badge.map(|b| {
                            div()
                                .flex_none()
                                .w(px(26.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(b.color))
                                .child(b.glyph)
                        }))
                        .child(
                            div()
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .child(SharedString::from(item.label.clone())),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(px(11.))
                        .text_color(rgb(theme::TEXT_DIM))
                        .child(SharedString::from(item.detail.clone().unwrap_or_default())),
                )
                .into_any_element();
            rows.push(row);

            if is_lang_mode && i == 0 && query.is_empty() {
                rows.push(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_3()
                        .py(px(3.))
                        .border_b_1()
                        .border_color(rgb(theme::BORDER))
                        .text_size(px(11.))
                        .child("")
                        .child(
                            div()
                                .text_color(rgb(0x3794ff))
                                .child("languages (identifier)"),
                        )
                        .into_any_element(),
                );
            }
        }
        let is_input = matches!(p.mode, palette::PaletteMode::Input(_));
        let error = p.error.clone().map(|e| {
            div()
                .px_3()
                .py_2()
                .text_size(px(12.))
                .text_color(rgb(0xe5534b))
                .child(SharedString::from(e))
        });
        let empty = (p.result_count() == 0 && !is_input).then(|| {
            div()
                .px_3()
                .py_2()
                .text_size(px(13.))
                .text_color(rgb(theme::TEXT_DIM))
                .child(if p.loading {
                    "Indexing files..."
                } else {
                    "No matching results"
                })
        });

        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(
                    div()
                        .id("palette-backdrop")
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.model.close_palette();
                            this.sync_focus(window);
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(48.))
                        .left(px(left))
                        .w(px(width))
                        .track_focus(&self.palette_focus)
                        .flex()
                        .flex_col()
                        .bg(rgb(theme::MENU_BG))
                        .border_1()
                        .border_color(rgb(theme::BORDER))
                        .rounded_md()
                        .shadow_xl()
                        .text_color(rgb(theme::TEXT))
                        .child(
                            div()
                                .relative()
                                .flex_none()
                                .h(px(34.))
                                .px(px(PALETTE_INPUT_PAD))
                                .flex()
                                .items_center()
                                .border_b_1()
                                .border_color(rgb(theme::BORDER))
                                .font_family(FONT_FAMILY)
                                .text_size(px(FONT_SIZE))
                                .child(if query.is_empty() {
                                    div()
                                        .text_color(rgb(theme::TEXT_DIM))
                                        .child(p.placeholder())
                                } else {
                                    div()
                                        .whitespace_nowrap()
                                        .child(SharedString::from(query.clone()))
                                })
                                .child(
                                    div()
                                        .absolute()
                                        .top(px(7.))
                                        .left(px(PALETTE_INPUT_PAD) + caret_x)
                                        .w(px(2.))
                                        .h(px(20.))
                                        .bg(rgb(theme::TEXT)),
                                )
                                .children(underline.map(|(x1, x2)| {
                                    div()
                                        .absolute()
                                        .top(px(26.))
                                        .left(px(PALETTE_INPUT_PAD) + x1)
                                        .w(x2 - x1)
                                        .h(px(1.))
                                        .bg(rgb(theme::TEXT))
                                }))
                                .child(input_canvas),
                        )
                        .child(
                            div()
                                .id("palette-rows")
                                .relative()
                                .on_scroll_wheel(cx.listener(
                                    move |this, ev: &ScrollWheelEvent, _w, cx| {
                                        let lines = match ev.delta {
                                            ScrollDelta::Lines(l) => l.y.round() as i32,
                                            ScrollDelta::Pixels(d) => {
                                                (f32::from(d.y) / 20.0).round() as i32
                                            }
                                        };
                                        let top = this.palette_top.get() as i32 - lines;
                                        let top = top.clamp(0, max_top as i32) as usize;
                                        if top != this.palette_top.get() {
                                            this.palette_top.set(top);
                                            cx.notify();
                                        }
                                        cx.stop_propagation();
                                    },
                                ))
                                .children(rows)
                                .children((count > PALETTE_ROWS).then(|| {
                                    let frac = |n: usize| n as f32 / count as f32;
                                    div()
                                        .absolute()
                                        .right(px(2.))
                                        .top(relative(frac(start)))
                                        .h(relative(frac(PALETTE_ROWS).max(0.06)))
                                        .w(px(3.))
                                        .rounded_sm()
                                        .bg(rgb(theme::SCROLL_THUMB))
                                })),
                        )
                        .children(empty)
                        .children(error),
                ),
        )
    }

    fn editor_bounds_for(&self, chars: Range<usize>, window: &Window) -> Option<Bounds<Pixels>> {
        let TabContent::Text(e) = &self.model.active()?.content else {
            return None;
        };
        let buf = e.buffer();
        let line = buf.line_of(chars.start.min(buf.len_chars()));
        let text = buf.line_text(line);
        let col = chars.start.saturating_sub(buf.line_start(line));
        let end_col = if buf.line_of(chars.end.min(buf.len_chars())) == line {
            chars.end.saturating_sub(buf.line_start(line))
        } else {
            text.chars().count()
        };
        let area = self.scroll.bounds();
        let offset = self.scroll.offset();
        let x1 = x_of_col(window, &text, col);
        let x2 = x_of_col(window, &text, end_col.max(col));
        Some(Bounds {
            origin: point(
                area.origin.x + px(GUTTER_WIDTH) + x1,
                area.origin.y + offset.y + px(EDITOR_PAD_Y) + px(LINE_HEIGHT * line as f32),
            ),
            size: size((x2 - x1).max(px(2.)), px(LINE_HEIGHT)),
        })
    }
}

fn chars_in_utf16_prefix(text: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (i, c) in text.chars().enumerate() {
        if units >= utf16 {
            return i;
        }
        units += c.len_utf16();
    }
    text.chars().count()
}

impl EntityInputHandler for Workbench {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        if self.target() == Target::Terminal {
            return None;
        }
        if let Some(f) = self.field() {
            let r = f.utf16_to_char(range.start)..f.utf16_to_char(range.end);
            *adjusted = Some(f.char_to_utf16(r.start)..f.char_to_utf16(r.end));
            return Some(f.slice(r));
        }
        let TabContent::Text(e) = &self.model.active()?.content else {
            return None;
        };
        let r = e.utf16_range_to_chars(range);
        *adjusted = Some(e.chars_range_to_utf16(r.clone()));
        Some(e.buffer().slice(r))
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if self.target() == Target::Terminal {
            let n = self.term_marked_utf16().map_or(0, |r| r.end);
            return Some(UTF16Selection {
                range: n..n,
                reversed: false,
            });
        }
        if let Some(f) = self.field() {
            let c = f.char_to_utf16(f.cursor());
            return Some(UTF16Selection {
                range: c..c,
                reversed: false,
            });
        }
        let TabContent::Text(e) = &self.model.active()?.content else {
            return None;
        };
        let (range, reversed) = e.selection_utf16();
        Some(UTF16Selection { range, reversed })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        if self.target() == Target::Terminal {
            return self.term_marked_utf16();
        }
        if let Some(f) = self.field() {
            return f
                .marked_range()
                .map(|r| f.char_to_utf16(r.start)..f.char_to_utf16(r.end));
        }
        match &self.model.active()?.content {
            TabContent::Text(e) => e.marked_utf16(),
            TabContent::Unavailable(_) | TabContent::Diff(_) | TabContent::Webview => None,
        }
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.target() == Target::Terminal {
            self.term_unmark();
        } else if let Some(f) = self.field_mut() {
            f.end_composition();
        } else if let Some(e) = self.model.active_editor_mut() {
            e.end_composition();
        }
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.target() == Target::Terminal {
            self.term_commit(text);
            return cx.notify();
        }
        if self.target() == Target::Editor && self.model.prompt.is_some() {
            return; // a confirmation is pending: the editor must not change underneath it
        }
        if let Some(f) = self.field_mut() {
            let chars = range.map(|r| f.utf16_to_char(r.start)..f.utf16_to_char(r.end));
            if chars.is_some() || f.marked_range().is_some() {
                f.commit(chars, text);
            } else {
                f.insert(text);
            }
            self.on_field_edited();
            self.reveal_cursor();
        } else if let Some(e) = self.model.active_editor_mut() {
            if e.is_composing() || range.is_some() {
                let chars = range.map(|r| e.utf16_range_to_chars(r));
                e.commit_text(chars, text);
            } else {
                e.insert(text);
            }
            self.reveal_cursor();
        }
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.target() == Target::Terminal {
            self.term_compose(new_text);
            return cx.notify();
        }
        if self.target() == Target::Editor && self.model.prompt.is_some() {
            return;
        }
        let selected = new_selected_range.map(|r| {
            chars_in_utf16_prefix(new_text, r.start)..chars_in_utf16_prefix(new_text, r.end)
        });
        if let Some(f) = self.field_mut() {
            let chars = range.map(|r| f.utf16_to_char(r.start)..f.utf16_to_char(r.end));
            f.compose(chars, new_text, selected);
            self.on_field_edited();
        } else if let Some(e) = self.model.active_editor_mut() {
            let chars = range.map(|r| e.utf16_range_to_chars(r));
            e.compose(chars, new_text, selected);
            self.reveal_cursor();
        }
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        if self.target() == Target::Terminal {
            return self.term_cursor_bounds();
        }
        if let Some(f) = self.field() {
            let pad = self.field_pad();
            let start = f.utf16_to_char(range_utf16.start);
            let end = f.utf16_to_char(range_utf16.end).max(start);
            let x1 = x_of_col(window, f.text(), start);
            let x2 = x_of_col(window, f.text(), end);
            return Some(Bounds {
                origin: point(
                    element_bounds.origin.x + px(pad) + x1,
                    element_bounds.origin.y,
                ),
                size: size((x2 - x1).max(px(2.)), element_bounds.size.height),
            });
        }
        let TabContent::Text(e) = &self.model.active()?.content else {
            return None;
        };
        let chars = e.utf16_range_to_chars(range_utf16);
        self.editor_bounds_for(chars, window)
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        if self.target() == Target::Terminal {
            return None;
        }
        let pos = self.position_at(point, window)?;
        match &self.model.active()?.content {
            TabContent::Text(e) => Some(e.chars_range_to_utf16(pos..pos).start),
            TabContent::Unavailable(_) | TabContent::Diff(_) | TabContent::Webview => None,
        }
    }
}

impl Render for Workbench {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let overlay_open = self.model.palette.is_some()
            || self.model.menu_open.is_some()
            || self.model.context_menu.is_some()
            || self.model.prompt.is_some()
            || self.external_prompt.is_some()
            || self.model.settings_open
            || self.model.ext_detail.is_some();
        let webview_wanted = !overlay_open
            && matches!(
                self.model.active().map(|t| &t.content),
                Some(TabContent::Webview)
            )
            && !(self.model.layout.panel && self.model.layout.panel_maximized);
        let webview_target = webview_wanted
            .then(|| self.webview_bounds.get())
            .flatten()
            .map(|b| {
                (
                    f32::from(b.origin.x) as i32,
                    f32::from(b.origin.y) as i32,
                    f32::from(b.size.width) as i32,
                    f32::from(b.size.height) as i32,
                )
            });
        // The native webview paints above everything GPUI draws, so the hover card needs a hole cut out of it.
        let hole = webview_target
            .zip(self.commit_card.and(self.card_bounds.get()))
            .map(|((wx, wy, _, _), b)| {
                (
                    f32::from(b.origin.x) as i32 - wx,
                    f32::from(b.origin.y) as i32 - wy,
                    f32::from(b.size.width).ceil() as i32,
                    f32::from(b.size.height).ceil() as i32,
                )
            });
        let scale = window.scale_factor();
        if let Some(panel) = self.panel.as_mut()
            && webview_target.is_some()
            && self.webview_applied == webview_target
        {
            panel.set_hole(hole, scale);
        }
        if let Some(panel) = self.panel.as_mut()
            && self.webview_applied != webview_target
        {
            match webview_target {
                Some((x, y, w, h)) => {
                    panel.set_area(x, y, w, h);
                    panel.set_hole(hole, scale);
                    panel.set_visible(true);
                }
                None => {
                    panel.set_visible(false);
                    panel.focus_parent();
                }
            }
            self.webview_applied = webview_target;
        }
        if window.focused(cx).is_none() {
            window.focus(&self.root_focus);
        }
        let layout = self.model.layout.clone();
        let sidebar = layout
            .sidebar
            .then(|| self.sidebar(window, cx).into_any_element());
        let activity_bar = layout
            .activity_bar
            .then(|| self.activity_bar(cx).into_any_element());
        let maximized = layout.panel && layout.panel_maximized;
        let split_sidebar = layout.sidebar.then(|| {
            self.splitter("split-sidebar", Drag::Sidebar, cx)
                .into_any_element()
        });
        let panel_drag = if layout.panel_pos.is_side() {
            Drag::PanelSide
        } else {
            Drag::Panel
        };
        let split_panel = (layout.panel && !maximized).then(|| {
            self.splitter("split-panel", panel_drag, cx)
                .into_any_element()
        });
        let tab_bar = (!maximized).then(|| self.tab_bar(cx).into_any_element());
        let editor: Option<AnyElement> = (!maximized).then(|| {
            if self.model.settings_open {
                self.settings_page(cx)
            } else {
                match &self.model.ext_detail {
                    Some(detail) => self.ext_detail_page(detail, cx),
                    None => self.editor(window, cx).into_any_element(),
                }
            }
        });
        let bottom_panel = layout
            .panel
            .then(|| self.bottom_panel(window, cx).into_any_element());
        self.model.refresh_find();
        let find_bar = if maximized {
            None
        } else {
            self.find_bar(window, cx).map(IntoElement::into_any_element)
        };
        let palette = self.palette_overlay(window, cx);
        let menu_bar = self.menu_bar(window, cx);
        let menus = self.menus_overlay(window, cx);
        let status_bar = layout.status_bar.then(|| self.status_bar(cx));
        let group = |a: Option<AnyElement>, b: Option<AnyElement>, c: Option<AnyElement>| {
            [a, b, c].into_iter().flatten().collect::<Vec<_>>()
        };
        let (sidebar_left, sidebar_right) = match layout.sidebar_side {
            Side::Left => (group(activity_bar, sidebar, split_sidebar), Vec::new()),
            Side::Right => (Vec::new(), group(split_sidebar, sidebar, activity_bar)),
        };
        let editor_column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .children(tab_bar)
            .children(find_bar)
            .children(editor);
        let center = match layout.panel_pos {
            PanelPos::Bottom => div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .child(editor_column)
                .children(split_panel)
                .children(bottom_panel),
            PanelPos::Top => div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .children(bottom_panel)
                .children(split_panel)
                .child(editor_column),
            PanelPos::Left => div()
                .flex()
                .flex_1()
                .min_w_0()
                .children(bottom_panel)
                .children(split_panel)
                .when(!maximized, |d| d.child(editor_column)),
            PanelPos::Right => div()
                .flex()
                .flex_1()
                .min_w_0()
                .when(!maximized, |d| d.child(editor_column))
                .children(split_panel)
                .children(bottom_panel),
        };
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(theme::EDITOR))
            .track_focus(&self.root_focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_move(cx.listener(Self::on_root_mouse_move))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _w, _cx| this.reclaim_focus()),
            )
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_root_mouse_up))
            .text_color(rgb(theme::TEXT))
            .child(menu_bar)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .children(sidebar_left)
                    .child(center)
                    .children(sidebar_right),
            )
            .children(status_bar)
            .children(menus)
            .children(self.commit_card_overlay(window, cx))
            .children(palette)
            .children(self.prompt_modal_overlay(cx))
    }
}

pub fn saved(outcome: &SaveOutcome) -> bool {
    matches!(outcome, SaveOutcome::Saved)
}
