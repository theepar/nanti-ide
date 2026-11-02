//! Workbench layout: activity bar | sidebar | editor (tabs + bottom panel) | AI panel slot,
//! status bar below. Dark theme only for now (PRD 6).

use std::collections::BTreeMap;

use editor_core::Buffer;
use gpui::{
    AnyElement, Context, Hsla, IntoElement, Render, SharedString, Window, div, prelude::*, px, rgb,
};

pub mod panel;
pub mod resource;

use panel::{Panel, UiEvent};

const AI_PANEL_WIDTH: f32 = 320.0;
const AI_PANEL_HEADER: f32 = 30.0;
const STATUS_BAR_HEIGHT: f32 = 22.0;

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
}

const SAMPLE: &str = "fn main() {\n    println!(\"Code now. Not later.\");\n}\n";
const FILES: [&str; 6] = [
    "▾ nanti",
    "   ▸ crates",
    "   ▸ extension-host",
    "     Cargo.toml",
    "     README.md",
    "     main.rs",
];

pub struct Workbench {
    buffer: Buffer,
    tabs: Vec<&'static str>,
    active_tab: usize,
    panel: Option<Panel>,
    /// Status bar items contributed by extensions (id -> text), only visible ones.
    status_items: BTreeMap<String, String>,
}

impl Workbench {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::from_text(SAMPLE),
            tabs: vec!["main.rs", "Cargo.toml"],
            active_tab: 0,
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

    fn activity_bar(&self) -> impl IntoElement {
        let icons = ["▤", "⌕", "⎇", "▶", "⧉"];
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(48.))
            .h_full()
            .bg(rgb(theme::ACTIVITY_BAR))
            .items_center()
            .pt_2()
            .gap_3()
            .text_size(px(20.))
            .text_color(rgb(theme::TEXT_DIM))
            .children(icons.into_iter().enumerate().map(|(i, g)| {
                let c: Hsla = if i == 0 {
                    rgb(theme::TEXT).into()
                } else {
                    rgb(theme::TEXT_DIM).into()
                };
                div().text_color(c).child(g)
            }))
    }

    fn sidebar(&self) -> impl IntoElement {
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
                    .px_3()
                    .py_2()
                    .text_size(px(11.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("EXPLORER"),
            )
            .children(FILES.iter().map(|f| {
                div()
                    .px_3()
                    .py(px(2.))
                    .text_size(px(13.))
                    .text_color(rgb(theme::TEXT))
                    .child(SharedString::from(*f))
            }))
    }

    fn tab_bar(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_none()
            .h(px(35.))
            .bg(rgb(theme::TAB_BAR))
            .border_b_1()
            .border_color(rgb(theme::BORDER))
            .children(self.tabs.iter().enumerate().map(|(i, t)| {
                let active = i == self.active_tab;
                let mut tab = div()
                    .flex()
                    .items_center()
                    .px_3()
                    .text_size(px(13.))
                    .border_r_1()
                    .border_color(rgb(theme::BORDER));
                if active {
                    tab = tab
                        .bg(rgb(theme::EDITOR))
                        .text_color(rgb(theme::TEXT))
                        .border_t_2()
                        .border_color(rgb(theme::ACCENT));
                } else {
                    tab = tab.text_color(rgb(theme::TEXT_DIM));
                }
                tab.child(SharedString::from(*t))
            }))
    }

    fn editor(&self) -> impl IntoElement {
        let text = self.buffer.to_string();
        let lines: Vec<AnyElement> = text
            .lines()
            .enumerate()
            .map(|(n, l)| {
                div()
                    .flex()
                    .child(
                        div()
                            .w(px(48.))
                            .flex_none()
                            .pr_3()
                            .flex()
                            .justify_end()
                            .text_color(rgb(theme::LINE_NUMBER))
                            .child(SharedString::from((n + 1).to_string())),
                    )
                    .child(SharedString::from(l.to_string()))
                    .into_any_element()
            })
            .collect();
        div()
            .flex_1()
            .bg(rgb(theme::EDITOR))
            .py_2()
            .font_family("Consolas")
            .text_size(px(14.))
            .text_color(rgb(theme::TEXT))
            .children(lines)
    }

    fn bottom_panel(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_none()
            .h(px(180.))
            .bg(rgb(theme::PANEL))
            .border_t_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .px_3()
                    .py_1()
                    .text_size(px(11.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("PROBLEMS")
                    .child("OUTPUT")
                    .child("DEBUG CONSOLE")
                    .child(div().text_color(rgb(theme::TEXT)).child("TERMINAL")),
            )
            .child(
                div()
                    .px_3()
                    .font_family("Consolas")
                    .text_size(px(13.))
                    .text_color(rgb(theme::TEXT))
                    .child("PS D:\\Kodingan\\nanti-ide> _"),
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
                    .child("⎇ main")
                    .child("0 errors  0 warnings"),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .children(
                        self.status_items
                            .values()
                            .map(|t| div().child(SharedString::from(t.clone()))),
                    )
                    .child(SharedString::from(format!(
                        "{} lines",
                        self.buffer.to_string().lines().count()
                    )))
                    .child("UTF-8")
                    .child("Rust"),
            )
    }
}

impl Default for Workbench {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for Workbench {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(self.activity_bar())
                    .child(self.sidebar())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(self.tab_bar())
                            .child(self.editor())
                            .child(self.bottom_panel()),
                    )
                    .child(self.ai_panel()),
            )
            .child(self.status_bar())
    }
}
