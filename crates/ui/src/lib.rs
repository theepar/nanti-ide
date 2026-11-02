//! Workbench: activity bar | sidebar | editor (tabs + bottom panel) | extension panel, status bar below.
//! Dark theme only for now (PRD 6). State lives in [`model::Model`]; this file only renders it and
//! maps clicks to model calls. Things that are not implemented say so on screen instead of faking it.

use std::collections::BTreeMap;
use std::path::PathBuf;

use gpui::{
    AnyElement, ClickEvent, Context, IntoElement, Render, SharedString, Window, div, prelude::*,
    px, rgb,
};

pub mod model;
pub mod panel;
pub mod resource;

use model::{Activity, Model, PanelTab, TabContent, language_of};
use panel::{Panel, UiEvent};

const AI_PANEL_WIDTH: f32 = 320.0;
const AI_PANEL_HEADER: f32 = 30.0;
const STATUS_BAR_HEIGHT: f32 = 22.0;
/// The viewer renders at most this many lines per file until the editor is virtualized.
const MAX_RENDERED_LINES: usize = 2000;

pub mod theme {
    pub const ACTIVITY_BAR: u32 = 0x2b2b2b;
    pub const SIDEBAR: u32 = 0x1f1f1f;
    pub const EDITOR: u32 = 0x181818;
    pub const TAB_BAR: u32 = 0x1f1f1f;
    pub const PANEL: u32 = 0x181818;
    pub const STATUS_BAR: u32 = 0x1f1f1f;
    pub const BORDER: u32 = 0x2e2e2e;
    pub const TEXT: u32 = 0xcccccc;
    pub const TEXT_DIM: u32 = 0x858585;
    pub const ACCENT: u32 = 0x3b82f6;
    pub const LINE_NUMBER: u32 = 0x6e7681;
    pub const HOVER: u32 = 0x2a2d2e;
    pub const SELECTED: u32 = 0x37373d;
}

fn activity_glyph(a: Activity) -> &'static str {
    match a {
        Activity::Explorer => "▤",
        Activity::Search => "⌕",
        Activity::SourceControl => "⎇",
        Activity::Run => "▶",
        Activity::Extensions => "⧉",
    }
}

pub struct Workbench {
    model: Model,
    panel: Option<Panel>,
    /// Status bar items contributed by extensions (id -> text), only visible ones.
    status_items: BTreeMap<String, String>,
}

impl Workbench {
    pub fn new(root: PathBuf) -> Self {
        Self {
            model: Model::new(root),
            panel: None,
            status_items: BTreeMap::new(),
        }
    }

    /// Show an extension webview in the right-hand panel.
    pub fn attach_panel(&mut self, panel: Panel) {
        self.panel = Some(panel);
    }

    /// Apply pending extension-host events. Returns true if the UI needs a redraw.
    pub fn pump(&mut self) -> bool {
        let Some(panel) = self.panel.as_mut() else {
            return false;
        };
        let mut changed = false;
        for ev in panel.pump() {
            if let UiEvent::StatusItem { id, visible, text } = ev {
                changed = true;
                if visible {
                    self.status_items.insert(id, text);
                } else {
                    self.status_items.remove(&id);
                }
            }
        }
        changed
    }

    fn activity_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(48.))
            .h_full()
            .bg(rgb(theme::ACTIVITY_BAR))
            .items_center()
            .pt_2()
            .gap_1()
            .children(Activity::ALL.into_iter().enumerate().map(|(i, a)| {
                let selected = a == self.model.activity && self.model.sidebar_visible;
                div()
                    .id(("activity", i))
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(40.))
                    .h(px(40.))
                    .rounded_md()
                    .cursor_pointer()
                    .text_size(px(20.))
                    .text_color(rgb(if selected {
                        theme::TEXT
                    } else {
                        theme::TEXT_DIM
                    }))
                    .hover(|s| s.bg(rgb(theme::HOVER)).text_color(rgb(theme::TEXT)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.model.select_activity(a);
                        cx.notify();
                    }))
                    .child(activity_glyph(a))
            }))
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let body: AnyElement = if self.model.activity == Activity::Explorer {
            let active_path = self.model.active().map(|t| t.path.clone());
            div()
                .id("explorer-list")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .children(
                    self.model
                        .visible_entries()
                        .into_iter()
                        .enumerate()
                        .map(|(i, e)| {
                            let selected = !e.is_dir && active_path.as_ref() == Some(&e.path);
                            let marker = match (e.is_dir, e.expanded) {
                                (true, true) => "▾ ",
                                (true, false) => "▸ ",
                                (false, _) => "  ",
                            };
                            let path = e.path.clone();
                            let is_dir = e.is_dir;
                            div()
                                .id(("entry", i))
                                .pl(px(8. + 12. * e.depth as f32))
                                .pr_2()
                                .py(px(2.))
                                .text_size(px(13.))
                                .text_color(rgb(theme::TEXT))
                                .cursor_pointer()
                                .when(selected, |d| d.bg(rgb(theme::SELECTED)))
                                .hover(|s| s.bg(rgb(theme::HOVER)))
                                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                    if is_dir {
                                        this.model.toggle_dir(&path);
                                    } else {
                                        this.model.open_file(&path);
                                    }
                                    cx.notify();
                                }))
                                .child(SharedString::from(format!("{marker}{}", e.name)))
                        }),
                )
                .into_any_element()
        } else {
            div()
                .px_3()
                .text_size(px(12.))
                .text_color(rgb(theme::TEXT_DIM))
                .child(SharedString::from(format!(
                    "{} is not implemented yet.",
                    self.model.activity.title().to_lowercase()
                )))
                .into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(260.))
            .h_full()
            .bg(rgb(theme::SIDEBAR))
            .border_r_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .flex_none()
                    .px_3()
                    .py_2()
                    .text_size(px(11.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child(self.model.activity.title()),
            )
            .child(body)
    }

    fn tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .flex()
            .flex_none()
            .h(px(35.))
            .bg(rgb(theme::TAB_BAR))
            .border_b_1()
            .border_color(rgb(theme::BORDER))
            .children(self.model.tabs.iter().enumerate().map(|(i, t)| {
                let active = self.model.active_tab == Some(i);
                div()
                    .id(("tab", i))
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
                        d.bg(rgb(theme::EDITOR))
                            .text_color(rgb(theme::TEXT))
                            .border_t_2()
                            .border_color(rgb(theme::ACCENT))
                    })
                    .when(!active, |d| {
                        d.text_color(rgb(theme::TEXT_DIM))
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.model.activate_tab(i);
                        cx.notify();
                    }))
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
                            .text_size(px(14.))
                            .hover(|s| s.bg(rgb(theme::SELECTED)))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                cx.stop_propagation();
                                this.model.close_tab(i);
                                cx.notify();
                            }))
                            .child("×"),
                    )
            }))
    }

    fn editor(&self) -> impl IntoElement {
        let base = div()
            .id("editor")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .bg(rgb(theme::EDITOR))
            .py_2()
            .font_family("Consolas")
            .text_size(px(14.))
            .text_color(rgb(theme::TEXT));
        match self.model.active().map(|t| &t.content) {
            None => base
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .font_family("Segoe UI")
                .child(div().text_size(px(22.)).child("nanti."))
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(theme::TEXT_DIM))
                        .child("Code now. Not later. Open a file from the Explorer."),
                ),
            Some(TabContent::Unavailable(reason)) => base
                .px_4()
                .text_color(rgb(theme::TEXT_DIM))
                .child(SharedString::from(reason.clone())),
            Some(TabContent::Text(buffer)) => {
                let text = buffer.to_string();
                let total = text.lines().count();
                let lines: Vec<AnyElement> = text
                    .lines()
                    .take(MAX_RENDERED_LINES)
                    .enumerate()
                    .map(|(n, l)| {
                        div()
                            .flex()
                            .child(
                                div()
                                    .w(px(56.))
                                    .flex_none()
                                    .pr_3()
                                    .flex()
                                    .justify_end()
                                    .text_color(rgb(theme::LINE_NUMBER))
                                    .child(SharedString::from((n + 1).to_string())),
                            )
                            .child(SharedString::from(l.replace('\t', "    ")))
                            .into_any_element()
                    })
                    .collect();
                let note =
                    (total > MAX_RENDERED_LINES).then(|| {
                        div().px_4().py_2().text_color(rgb(theme::TEXT_DIM)).child(
                            SharedString::from(format!(
                                "… {} more lines not shown (viewer limit {MAX_RENDERED_LINES})",
                                total - MAX_RENDERED_LINES
                            )),
                        )
                    });
                base.children(lines).children(note)
            }
        }
    }

    fn bottom_panel(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let (_, note) = match self.model.panel_tab {
            PanelTab::Problems => (
                0,
                "Problems: not implemented yet (needs LSP diagnostics, ED-10).",
            ),
            PanelTab::Output => (
                1,
                "Output: not implemented yet (extension output channels arrive from the host).",
            ),
            PanelTab::DebugConsole => (2, "Debug console: not implemented yet (needs DAP, DB-04)."),
            PanelTab::Terminal => (3, "Terminal: not implemented yet (TR-01)."),
        };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .h(px(180.))
            .bg(rgb(theme::PANEL))
            .border_t_1()
            .border_color(rgb(theme::BORDER))
            .child(div().flex().flex_none().gap_1().px_2().py_1().children(
                PanelTab::ALL.into_iter().enumerate().map(|(i, t)| {
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
                            d.border_b_1().border_color(rgb(theme::ACCENT))
                        })
                        .hover(|s| s.bg(rgb(theme::HOVER)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.model.panel_tab = t;
                            cx.notify();
                        }))
                        .child(t.title())
                }),
            ))
            .child(
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(13.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child(note),
            )
    }

    fn ai_panel(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(AI_PANEL_WIDTH))
            .h_full()
            .bg(rgb(theme::SIDEBAR))
            .border_l_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .h(px(AI_PANEL_HEADER))
                    .flex_none()
                    .px_3()
                    .flex()
                    .items_center()
                    .text_size(px(11.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("EXTENSION PANEL"),
            )
            .children(self.panel.is_none().then(|| {
                div()
                    .px_3()
                    .text_size(px(12.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("No extension view loaded (set NANTI_EXTENSION and NANTI_VIEW)")
            }))
    }

    fn status_bar(&self) -> impl IntoElement {
        let branch = self
            .model
            .git_branch()
            .map_or_else(|| "no repository".to_owned(), |b| format!("⎇ {b}"));
        let (lines, lang) = match self.model.active() {
            Some(t) => (
                match &t.content {
                    TabContent::Text(b) => Some(b.to_string().lines().count()),
                    TabContent::Unavailable(_) => None,
                },
                Some(language_of(&t.path)),
            ),
            None => (None, None),
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
            .child(div().flex().gap_4().child(SharedString::from(branch)))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .children(
                        self.status_items
                            .values()
                            .map(|t| div().child(SharedString::from(t.clone()))),
                    )
                    .children(lines.map(|n| SharedString::from(format!("{n} lines"))))
                    .children(lang.map(|_| "UTF-8"))
                    .children(lang)
                    .child(div().text_color(rgb(theme::TEXT_DIM)).child("Read-only")),
            )
    }
}

impl Default for Workbench {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

impl Render for Workbench {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The WebView2 child is a native surface on top of the GPUI window: keep it glued to the
        // right-hand panel area on every layout pass (covers window resize).
        if let Some(panel) = self.panel.as_mut() {
            let vp = window.viewport_size();
            let (vw, vh) = (f32::from(vp.width), f32::from(vp.height));
            panel.set_area(
                (vw - AI_PANEL_WIDTH + 1.0) as i32,
                AI_PANEL_HEADER as i32,
                (AI_PANEL_WIDTH - 1.0) as i32,
                (vh - STATUS_BAR_HEIGHT - AI_PANEL_HEADER) as i32,
            );
        }
        let sidebar = if self.model.sidebar_visible {
            Some(self.sidebar(cx))
        } else {
            None
        };
        let activity_bar = self.activity_bar(cx);
        let tab_bar = self.tab_bar(cx);
        let bottom_panel = self.bottom_panel(cx);
        div()
            .flex()
            .flex_col()
            .size_full()
            .text_color(rgb(theme::TEXT))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(activity_bar)
                    .children(sidebar)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(tab_bar)
                            .child(self.editor())
                            .child(bottom_panel),
                    )
                    .child(self.ai_panel()),
            )
            .child(self.status_bar())
    }
}
