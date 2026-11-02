use gpui::{
    AnyElement, Bounds, ClickEvent, Context, FontWeight, IntoElement, MouseButton, MouseDownEvent,
    ObjectFit, Pixels, ScrollHandle, SharedString, Window, WindowControlArea, canvas, div, fill,
    img, point, prelude::*, px, rgb, size,
};

use crate::assets;
use crate::menu::{self, MenuItem, MenuKind};
use crate::model::PanelTab;
use crate::palette::Command;
use crate::{MENU_BAR_HEIGHT, Target, Workbench, theme};

fn short_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn parse_link(b: &[char], open: usize) -> Option<(String, usize)> {
    let mut depth = 0usize;
    let mut close = None;
    for (i, c) in b.iter().enumerate().skip(open) {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    if b.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = b[close + 1..].iter().position(|c| *c == ')')? + close + 2;
    let text: String = b[open + 1..close].iter().collect();
    Some((text, end))
}

fn strip_markup(s: &str) -> String {
    let b: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            '<' => {
                if let Some(j) = b[i..].iter().position(|c| *c == '>') {
                    i += j + 1;
                    continue;
                }
                out.push('<');
            }
            '!' if b.get(i + 1) == Some(&'[') => {
                if let Some((_, end)) = parse_link(&b, i + 1) {
                    i = end;
                    continue;
                }
                out.push('!');
            }
            '[' => {
                if let Some((text, end)) = parse_link(&b, i) {
                    out.push_str(&strip_markup(&text));
                    i = end;
                    continue;
                }
                out.push('[');
            }
            '*' | '`' => {}
            c => out.push(c),
        }
        i += 1;
    }
    out
}

fn readme_blocks(text: &str) -> Vec<AnyElement> {
    let mut out: Vec<AnyElement> = Vec::new();
    let (mut in_code, mut in_comment) = (false, false);
    for raw in text.lines().take(400) {
        let line = raw.trim_end();
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            out.push(
                div()
                    .px_2()
                    .bg(rgb(0x1e1e1e))
                    .font_family(crate::FONT_FAMILY)
                    .text_size(px(12.))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .child(SharedString::from(line.to_owned()))
                    .into_any_element(),
            );
            continue;
        }
        if in_comment {
            in_comment = !line.contains("-->");
            continue;
        }
        if line.contains("<!--") && !line.contains("-->") {
            in_comment = true;
            continue;
        }
        let clean = strip_markup(line);
        let t = clean.trim();
        if t.is_empty() {
            out.push(div().h(px(8.)).into_any_element());
            continue;
        }
        let level = t.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&level) && t[level..].starts_with(' ') {
            let size = match level {
                1 => 22.,
                2 => 18.,
                3 => 15.,
                _ => 13.,
            };
            out.push(
                div()
                    .pt_2()
                    .pb_1()
                    .text_size(px(size))
                    .font_weight(FontWeight::SEMIBOLD)
                    .when(level <= 2, |d| {
                        d.border_b_1().border_color(rgb(theme::BORDER))
                    })
                    .child(SharedString::from(t[level..].trim().to_owned()))
                    .into_any_element(),
            );
        } else if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            out.push(
                div()
                    .flex()
                    .gap_2()
                    .pl_3()
                    .text_size(px(13.))
                    .child("•")
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(SharedString::from(rest.to_owned())),
                    )
                    .into_any_element(),
            );
        } else {
            out.push(
                div()
                    .text_size(px(13.))
                    .text_color(rgb(0xcfcfcf))
                    .child(SharedString::from(t.to_owned()))
                    .into_any_element(),
            );
        }
    }
    out
}

const BRAND_W: f32 = 38.0;
const MENU_W: f32 = 260.0;

fn button_w(kind: MenuKind) -> f32 {
    kind.title().chars().count() as f32 * 7.2 + 22.0
}

fn menu_left(kind: MenuKind) -> f32 {
    BRAND_W
        + MenuKind::ALL
            .iter()
            .take_while(|k| **k != kind)
            .map(|k| button_w(*k))
            .sum::<f32>()
}

const SCROLLBAR_W: f32 = 12.0;
const SCROLLBAR_MIN_THUMB: f32 = 30.0;

fn thumb_geometry(handle: &ScrollHandle, track: Bounds<Pixels>) -> Option<(f32, f32)> {
    let view = f32::from(handle.bounds().size.height);
    let max = f32::from(handle.max_offset().height);
    if max <= 1.0 || view <= 1.0 {
        return None;
    }
    let track_h = f32::from(track.size.height);
    let thumb = (track_h * view / (view + max)).clamp(SCROLLBAR_MIN_THUMB.min(track_h), track_h);
    let frac = (-f32::from(handle.offset().y) / max).clamp(0.0, 1.0);
    Some((f32::from(track.origin.y) + (track_h - thumb) * frac, thumb))
}

pub(crate) fn scroll_to_pointer(handle: &ScrollHandle, y: Pixels) {
    let b = handle.bounds();
    let (view, max) = (
        f32::from(b.size.height),
        f32::from(handle.max_offset().height),
    );
    if max <= 1.0 || view <= 1.0 {
        return;
    }
    let thumb = (view * view / (view + max)).clamp(SCROLLBAR_MIN_THUMB.min(view), view);
    let span = (view - thumb).max(1.0);
    let frac = ((f32::from(y - b.origin.y) - thumb / 2.0) / span).clamp(0.0, 1.0);
    let off = handle.offset();
    handle.set_offset(point(off.x, px(-frac * max)));
}

impl Workbench {
    pub(crate) fn scrollbar(
        &self,
        id: &str,
        handle: &ScrollHandle,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (down, paint) = (handle.clone(), handle.clone());
        div()
            .id(SharedString::from(format!("{id}-bar")))
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .w(px(SCROLLBAR_W))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                    scroll_to_pointer(&down, ev.position.y);
                    this.scroll_drag = Some(down.clone());
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .child(
                canvas(
                    |_, _, _| (),
                    move |track, (), window, _cx| {
                        let Some((top, height)) = thumb_geometry(&paint, track) else {
                            return;
                        };
                        window.paint_quad(fill(track, rgb(theme::SCROLL_TRACK)));
                        let thumb = Bounds {
                            origin: point(track.origin.x + px(2.), px(top)),
                            size: size(px(SCROLLBAR_W - 4.), px(height)),
                        };
                        window
                            .paint_quad(fill(thumb, rgb(theme::SCROLL_THUMB)).corner_radii(px(4.)));
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .into_any_element()
    }

    pub(crate) fn scroll_box(
        &self,
        id: &str,
        inner: AnyElement,
        handle: &ScrollHandle,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .child(inner)
            .child(self.scrollbar(id, handle, cx))
            .into_any_element()
    }

    pub(crate) fn scroll_list(
        &self,
        id: &'static str,
        handle: &ScrollHandle,
        rows: Vec<AnyElement>,
        max_h: Option<f32>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let list = div()
            .id(id)
            .overflow_y_scroll()
            .track_scroll(handle)
            .children(rows);
        match max_h {
            Some(h) => div()
                .relative()
                .child(list.max_h(px(h)))
                .child(self.scrollbar(id, handle, cx))
                .into_any_element(),
            None => self.scroll_box(id, list.size_full().into_any_element(), handle, cx),
        }
    }

    fn caption_button(
        id: &'static str,
        area: WindowControlArea,
        icon_name: &'static str,
        danger: bool,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .window_control_area(area)
            .occlude()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .w(px(46.))
            .h(px(MENU_BAR_HEIGHT))
            .hover(move |s| s.bg(rgb(if danger { 0xe81123 } else { theme::HOVER })))
            .child(assets::icon(icon_name, 14., 0xe6e6e6))
    }

    pub(crate) fn menu_bar(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let open = self.model.menu_open;
        div()
            .flex()
            .flex_none()
            .items_center()
            .h(px(MENU_BAR_HEIGHT))
            .bg(rgb(theme::ACTIVITY_BAR))
            .border_b_1()
            .border_color(rgb(theme::BORDER))
            .text_size(px(12.))
            .child(
                div()
                    .w(px(BRAND_W))
                    .flex_none()
                    .flex()
                    .items_center()
                    .h_full()
                    .pl_3()
                    .window_control_area(WindowControlArea::Drag)
                    .child(assets::logo_mark(14., 0xffffff)),
            )
            .children(MenuKind::ALL.into_iter().enumerate().map(|(i, k)| {
                div()
                    .id(("menu", i))
                    .flex_none()
                    .w(px(button_w(k)))
                    .h(px(MENU_BAR_HEIGHT - 6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .cursor_pointer()
                    .text_color(rgb(if open == Some(k) {
                        0xffffff
                    } else {
                        theme::TEXT
                    }))
                    .when(open == Some(k), |d| d.bg(rgb(theme::SELECTED)))
                    .hover(|s| s.bg(rgb(theme::HOVER)).text_color(rgb(0xffffff)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        if this.model.menu_open == Some(k) {
                            this.model.close_menus();
                        } else {
                            this.model.open_menu(k);
                        }
                        cx.notify();
                    }))
                    .on_hover(cx.listener(move |this, hovered: &bool, _w, cx| {
                        if *hovered
                            && this.model.menu_open.is_some()
                            && this.model.menu_open != Some(k)
                        {
                            this.model.open_menu(k);
                            cx.notify();
                        }
                    }))
                    .child(k.title())
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .window_control_area(WindowControlArea::Drag),
            )
            .child(Self::caption_button(
                "win-min",
                WindowControlArea::Min,
                "minimize",
                false,
            ))
            .child(Self::caption_button(
                "win-max",
                WindowControlArea::Max,
                if window.is_maximized() {
                    "restore"
                } else {
                    "maximize"
                },
                false,
            ))
            .child(Self::caption_button(
                "win-close",
                WindowControlArea::Close,
                "close",
                true,
            ))
            .into_any_element()
    }

    fn menu_rows(
        &self,
        items: Vec<MenuItem>,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        items
            .into_iter()
            .enumerate()
            .map(|(i, it)| match it {
                MenuItem::Separator => div()
                    .h(px(1.))
                    .my(px(4.))
                    .mx_2()
                    .bg(rgb(theme::BORDER))
                    .into_any_element(),
                MenuItem::Command(c) => div()
                    .id((id, i))
                    .flex()
                    .justify_between()
                    .items_center()
                    .gap_6()
                    .px_3()
                    .py(px(5.))
                    .mx_1()
                    .rounded_sm()
                    .text_size(px(12.))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::HOVER_STRONG)).text_color(rgb(0xffffff)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.model.close_menus();
                        this.apply_command(c, window, cx);
                    }))
                    .child(c.menu_label())
                    .child(
                        div()
                            .text_color(rgb(theme::TEXT_DIM))
                            .child(c.keybinding().unwrap_or("")),
                    )
                    .into_any_element(),
            })
            .collect()
    }

    pub(crate) fn menus_overlay(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let (left, top, rows) = if let Some(kind) = self.model.menu_open {
            (
                menu_left(kind),
                MENU_BAR_HEIGHT,
                self.menu_rows(menu::items(kind), "menu-item", cx),
            )
        } else if let Some(c) = &self.model.context_menu {
            let (vw, vh) = (
                f32::from(window.viewport_size().width),
                f32::from(window.viewport_size().height),
            );
            let items = menu::context_items(c.is_dir);
            let h = items.len() as f32 * 26.0;
            (
                c.x.min(vw - MENU_W - 4.0).max(0.0),
                c.y.min(vh - h - 8.0).max(0.0),
                self.menu_rows(items, "ctx-item", cx),
            )
        } else {
            return None;
        };
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(
                    div()
                        .id("menu-backdrop")
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                            this.model.close_menus();
                            cx.notify();
                        }))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(|this, _: &MouseDownEvent, _w, cx| {
                                this.model.close_menus();
                                cx.notify();
                            }),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(top))
                        .left(px(left))
                        .w(px(MENU_W))
                        .py_1()
                        .flex()
                        .flex_col()
                        .bg(rgb(theme::MENU_BG))
                        .border_1()
                        .border_color(rgb(theme::BORDER))
                        .rounded_md()
                        .shadow_xl()
                        .text_color(rgb(theme::TEXT))
                        .children(rows),
                )
                .into_any_element(),
        )
    }

    fn flat_button(
        id: impl Into<gpui::ElementId>,
        label: &'static str,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .px_2()
            .py(px(2.))
            .rounded_sm()
            .cursor_pointer()
            .text_size(px(11.))
            .text_color(rgb(theme::TEXT_DIM))
            .hover(|s| s.bg(rgb(theme::HOVER)).text_color(rgb(theme::TEXT)))
            .child(label)
    }

    fn icon_button(id: &'static str, icon_name: &'static str) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .w(px(26.))
            .h(px(26.))
            .rounded_sm()
            .cursor_pointer()
            .hover(|s| s.bg(rgb(theme::HOVER)))
            .child(assets::icon(icon_name, 17., theme::ICON))
    }

    pub(crate) fn explorer_toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap_1()
            .px_2()
            .pb_1()
            .child(
                Self::icon_button("ex-open", "folder-open").on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| {
                        this.apply_command(Command::OpenFolder, window, cx)
                    },
                )),
            )
            .child(
                Self::icon_button("ex-new-file", "file-plus").on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| {
                        this.apply_command(Command::NewFile, window, cx)
                    },
                )),
            )
            .child(
                Self::icon_button("ex-new-folder", "folder-plus").on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| {
                        this.apply_command(Command::NewFolder, window, cx)
                    },
                )),
            )
            .child(
                Self::icon_button("ex-refresh", "refresh").on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| {
                        this.apply_command(Command::RefreshExplorer, window, cx)
                    },
                )),
            )
            .child(
                Self::icon_button("ex-collapse", "collapse").on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| {
                        this.apply_command(Command::CollapseAll, window, cx)
                    },
                )),
            )
            .into_any_element()
    }

    pub(crate) fn outline_section(&self, cx: &mut Context<Self>) -> AnyElement {
        let symbols = self.model.outline();
        let open = self.outline_open;
        let rows: Vec<AnyElement> = if open {
            symbols
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let line = s.line;
                    div()
                        .id(("outline", i))
                        .flex()
                        .gap_2()
                        .pl(px(12. + 12. * s.depth as f32))
                        .pr_2()
                        .py(px(1.))
                        .text_size(px(12.))
                        .cursor_pointer()
                        .hover(|st| st.bg(rgb(theme::HOVER)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            let _ = this.model.go_to_line(line + 1, 1);
                            this.focus_editor(window);
                            this.reveal_cursor();
                            cx.notify();
                        }))
                        .child(
                            div()
                                .flex_none()
                                .w(px(46.))
                                .text_color(rgb(theme::TEXT_DIM))
                                .child(s.kind),
                        )
                        .child(
                            div()
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .child(SharedString::from(s.name.clone())),
                        )
                        .into_any_element()
                })
                .collect()
        } else {
            Vec::new()
        };
        let empty = (open && rows.is_empty()).then(|| {
            div()
                .px_3()
                .py_1()
                .text_size(px(11.))
                .text_color(rgb(theme::TEXT_DIM))
                .child("No symbols (open a Rust, JS/TS, Go or Markdown file).")
        });
        div()
            .flex()
            .flex_col()
            .flex_none()
            .border_t_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .id("outline-header")
                    .px_2()
                    .py(px(4.))
                    .text_size(px(11.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::HOVER)))
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.outline_open = !this.outline_open;
                        cx.notify();
                    }))
                    .child(SharedString::from(format!(
                        "{} OUTLINE{}",
                        if open { "▾" } else { "▸" },
                        if symbols.is_empty() {
                            String::new()
                        } else {
                            format!("  ({})", symbols.len())
                        }
                    ))),
            )
            .children(empty)
            .child(self.scroll_list("outline-list", &self.scrolls.outline, rows, Some(190.), cx))
            .into_any_element()
    }

    pub(crate) fn run_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let tasks = &self.model.tasks;
        let rows: Vec<AnyElement> = tasks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                div()
                    .id(("task", i))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_3()
                    .py(px(4.))
                    .text_size(px(13.))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::HOVER)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.run_task(i, window);
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(div().text_color(rgb(0x81b88b)).child("▶"))
                            .child(SharedString::from(t.label.clone())),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(theme::TEXT_DIM))
                            .child(t.source),
                    )
                    .into_any_element()
            })
            .collect();
        let empty = rows.is_empty().then(|| {
            div()
                .px_3()
                .py_2()
                .text_size(px(12.))
                .text_color(rgb(theme::TEXT_DIM))
                .child("No tasks found. Tasks come from Cargo.toml, package.json, go.mod or pubspec.yaml in the workspace folder.")
        });
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .flex()
                    .gap_1()
                    .px_2()
                    .pb_1()
                    .child(Self::flat_button("run-refresh", "Refresh").on_click(cx.listener(
                        |this, _: &ClickEvent, _w, cx| {
                            this.model.refresh_tasks();
                            cx.notify();
                        },
                    ))),
            )
            .children(empty)
            .child(self.scroll_list("task-list", &self.scrolls.tasks, rows, None, cx))
            .child(
                div()
                    .flex_none()
                    .px_3()
                    .py_2()
                    .text_size(px(11.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("Tasks run in the integrated terminal. Debugging (breakpoints, stepping) is not available."),
            )
            .into_any_element()
    }

    fn ext_icon(
        &self,
        id: &str,
        own: Option<&std::path::PathBuf>,
        label: &str,
        size: f32,
    ) -> AnyElement {
        let path = own
            .cloned()
            .or_else(|| self.model.ext_icons.get(id).cloned());
        match path {
            Some(p) => img(p)
                .size(px(size))
                .flex_none()
                .rounded_md()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),
            None => div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(size))
                .rounded_md()
                .bg(rgb(0x2d2d2d))
                .text_size(px(size * 0.45))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(theme::TEXT_DIM))
                .child(SharedString::from(
                    label
                        .chars()
                        .find(|c| c.is_alphanumeric())
                        .map_or_else(String::new, |c| c.to_uppercase().to_string()),
                ))
                .into_any_element(),
        }
    }

    fn setting_row(title: &'static str, hint: &'static str, control: AnyElement) -> AnyElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_6()
            .py_3()
            .border_b_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(div().text_size(px(13.)).child(title))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(theme::TEXT_DIM))
                            .child(hint),
                    ),
            )
            .child(control)
            .into_any_element()
    }

    fn segmented(
        &self,
        id: &'static str,
        options: Vec<(&'static str, bool, Command)>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .flex()
            .flex_none()
            .rounded_md()
            .border_1()
            .border_color(rgb(theme::BORDER))
            .overflow_hidden()
            .children(
                options
                    .into_iter()
                    .enumerate()
                    .map(|(i, (label, on, command))| {
                        div()
                            .id((id, i))
                            .flex()
                            .items_center()
                            .h(px(28.))
                            .px_3()
                            .text_size(px(12.))
                            .cursor_pointer()
                            .when(on, |d| {
                                d.bg(rgb(theme::ACCENT))
                                    .text_color(rgb(0x1e1e1e))
                                    .font_weight(FontWeight::MEDIUM)
                            })
                            .when(!on, |d| {
                                d.text_color(rgb(theme::TEXT))
                                    .hover(|s| s.bg(rgb(theme::HOVER)))
                            })
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.apply_command(command, window, cx);
                            }))
                            .child(label)
                    }),
            )
            .into_any_element()
    }

    fn switch(
        &self,
        id: &'static str,
        on: bool,
        command: Command,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id(id)
            .flex()
            .items_center()
            .flex_none()
            .w(px(40.))
            .h(px(22.))
            .p(px(2.))
            .rounded_full()
            .cursor_pointer()
            .bg(rgb(if on { theme::ACCENT } else { 0x3a3a3a }))
            .when(on, |d| d.justify_end())
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.apply_command(command, window, cx);
            }))
            .child(
                div()
                    .size(px(18.))
                    .rounded_full()
                    .bg(rgb(if on { 0x1e1e1e } else { 0xb0b0b0 })),
            )
            .into_any_element()
    }

    pub(crate) fn settings_page(&self, cx: &mut Context<Self>) -> AnyElement {
        use crate::layout::{PanelPos, Side};
        let l = &self.model.layout;
        let section = |text: &'static str| {
            div()
                .pt_4()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(theme::TEXT_DIM))
                .child(text)
        };
        let sidebar_side = self.segmented(
            "set-sidebar-side",
            vec![
                ("Left", l.sidebar_side == Side::Left, Command::SidebarLeft),
                (
                    "Right",
                    l.sidebar_side == Side::Right,
                    Command::SidebarRight,
                ),
            ],
            cx,
        );
        let panel_pos = self.segmented(
            "set-panel-pos",
            vec![
                (
                    "Bottom",
                    l.panel_pos == PanelPos::Bottom,
                    Command::PanelBottom,
                ),
                ("Top", l.panel_pos == PanelPos::Top, Command::PanelTop),
                ("Left", l.panel_pos == PanelPos::Left, Command::PanelLeft),
                ("Right", l.panel_pos == PanelPos::Right, Command::PanelRight),
            ],
            cx,
        );
        let saved_in = crate::layout::layout_path()
            .map_or_else(|| "memory only".to_owned(), |p| p.display().to_string());
        let page = div()
            .id("settings-page")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scrolls.settings)
            .px_8()
            .py_6()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .max_w(px(760.))
                    .child(
                        div()
                            .text_size(px(26.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Settings"),
                    )
                    .child(
                        div()
                            .pt_1()
                            .text_size(px(13.))
                            .text_color(rgb(theme::TEXT_DIM))
                            .child("Changes apply at once and are saved on this computer."),
                    )
                    .child(section("LAYOUT"))
                    .child(Self::setting_row(
                        "Sidebar position",
                        "Which side the sidebar and the activity bar sit on.",
                        sidebar_side,
                    ))
                    .child(Self::setting_row(
                        "Panel position",
                        "Where the terminal, problems and output panel docks.",
                        panel_pos,
                    ))
                    .child(section("VISIBILITY"))
                    .child(Self::setting_row(
                        "Activity bar",
                        "The icon strip next to the sidebar.",
                        self.switch(
                            "set-activity",
                            l.activity_bar,
                            Command::ToggleActivityBar,
                            cx,
                        ),
                    ))
                    .child(Self::setting_row(
                        "Sidebar",
                        "Explorer, search, source control, run and extensions.",
                        self.switch("set-sidebar", l.sidebar, Command::ToggleSidebar, cx),
                    ))
                    .child(Self::setting_row(
                        "Panel",
                        "Terminal, problems and output. Hidden when nanti. starts.",
                        self.switch("set-panel", l.panel, Command::TogglePanel, cx),
                    ))
                    .child(Self::setting_row(
                        "Extension panel",
                        "The side panel an extension can fill (appears when one does).",
                        self.switch("set-right", l.right, Command::ToggleRightPanel, cx),
                    ))
                    .child(Self::setting_row(
                        "Status bar",
                        "The bar at the bottom of the window.",
                        self.switch("set-status", l.status_bar, Command::ToggleStatusBar, cx),
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_4()
                            .pt_5()
                            .child(
                                div()
                                    .min_w_0()
                                    .text_size(px(11.))
                                    .text_color(rgb(theme::TEXT_DIM))
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .child(SharedString::from(format!("Saved in {saved_in}"))),
                            )
                            .child(Self::flat_button("set-reset", "Reset layout").on_click(
                                cx.listener(|this, _: &ClickEvent, window, cx| {
                                    this.apply_command(Command::ResetLayout, window, cx);
                                }),
                            )),
                    ),
            );
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .bg(rgb(theme::EDITOR))
            .child(self.scroll_box(
                "settings",
                page.into_any_element(),
                &self.scrolls.settings,
                cx,
            ))
            .into_any_element()
    }

    pub(crate) fn ext_detail_page(
        &self,
        d: &crate::extensions::ExtDetail,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use crate::extensions::Readme;
        let installed = self.model.extensions.iter().find(|e| e.id == d.id);
        let installed_root = self
            .model
            .config_dir
            .as_ref()
            .map(|c| crate::extensions::extensions_dir(c));
        let removable = installed
            .zip(installed_root.as_ref())
            .is_some_and(|(e, root)| e.dir.parent() == Some(root.as_path()));

        let action: AnyElement = if let Some(x) = installed {
            let dir = x.dir.clone();
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(0x81b88b))
                        .child("Installed"),
                )
                .children(removable.then(|| {
                    Self::flat_button("ext-detail-remove", "Uninstall").on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            this.model.confirm_remove_extension(&dir);
                            cx.notify();
                        },
                    ))
                }))
                .into_any_element()
        } else if let Some(url) = d.download_url.clone() {
            if self.model.installing.as_deref() == Some(url.as_str()) {
                div()
                    .text_size(px(12.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child(SharedString::from(self.model.install_label()))
                    .into_any_element()
            } else {
                div()
                    .id("ext-detail-install")
                    .flex()
                    .items_center()
                    .h(px(30.))
                    .px_4()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(rgb(theme::ACCENT))
                    .text_color(rgb(0x1e1e1e))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .hover(|s| s.bg(rgb(0xffffff)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.model.install_from_marketplace(&url);
                        cx.notify();
                    }))
                    .child("Install")
                    .into_any_element()
            }
        } else {
            div().into_any_element()
        };

        let fact = |label: &'static str, value: String| {
            div()
                .flex()
                .gap_3()
                .py(px(3.))
                .text_size(px(12.))
                .child(
                    div()
                        .flex_none()
                        .w(px(84.))
                        .text_color(rgb(theme::TEXT_DIM))
                        .child(label),
                )
                .child(div().flex_1().min_w_0().child(SharedString::from(value)))
        };
        let mut facts = div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(260.))
            .p_4()
            .rounded_lg()
            .bg(rgb(0x202020))
            .border_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .pb_2()
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Installation"),
            )
            .child(fact("Identifier", d.id.clone()))
            .child(fact("Version", d.version.clone()));
        if let Some(n) = d.downloads {
            facts = facts.child(fact("Downloads", short_count(n)));
        }
        if let Some(r) = d.rating.filter(|_| d.rating_count > 0) {
            facts = facts.child(fact("Rating", format!("{r:.1} / 5  ({})", d.rating_count)));
        }
        if let Some(t) = &d.updated {
            facts = facts.child(fact("Last updated", t.chars().take(10).collect()));
        }
        if let Some(l) = &d.license {
            let short: String = l.chars().take(60).collect();
            facts = facts.child(fact("License", short));
        }
        if !d.categories.is_empty() {
            facts = facts.child(fact("Categories", d.categories.join(", ")));
        }

        let readme: Vec<AnyElement> = match &d.readme {
            Readme::Ready(text) => readme_blocks(text),
            Readme::Loading => vec![
                div()
                    .text_size(px(13.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("Loading the description...")
                    .into_any_element(),
            ],
            Readme::Failed(e) => vec![
                div()
                    .text_size(px(13.))
                    .text_color(rgb(0xe5534b))
                    .child(SharedString::from(e.clone()))
                    .into_any_element(),
            ],
            Readme::None => vec![
                div()
                    .text_size(px(13.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("This extension has no description page.")
                    .into_any_element(),
            ],
        };

        let page = div()
            .id("ext-detail")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scrolls.ext_detail)
            .px_8()
            .py_6()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .flex()
                    .gap_5()
                    .items_center()
                    .child(self.ext_icon(
                        &d.id,
                        installed.and_then(|e| e.icon.as_ref()),
                        &d.display_name,
                        96.,
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(28.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(SharedString::from(d.display_name.clone())),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(rgb(theme::TEXT_DIM))
                                    .child(SharedString::from(d.publisher.clone())),
                            )
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .text_color(rgb(0xcfcfcf))
                                    .child(SharedString::from(d.description.clone())),
                            )
                            .child(div().pt_2().flex().child(action)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .border_b_1()
                    .border_color(rgb(theme::BORDER))
                    .child(
                        div()
                            .pb(px(6.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .border_b_2()
                            .border_color(rgb(theme::ACCENT))
                            .child("DETAILS"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_6()
                    .items_start()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .max_w(px(780.))
                            .gap(px(4.))
                            .children(readme),
                    )
                    .child(facts),
            );

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .bg(rgb(theme::EDITOR))
            .child(self.scroll_box(
                "ext-detail",
                page.into_any_element(),
                &self.scrolls.ext_detail,
                cx,
            ))
            .into_any_element()
    }

    pub(crate) fn extensions_panel(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let installed_root = self
            .model
            .config_dir
            .as_ref()
            .map(|c| crate::extensions::extensions_dir(c));

        let mut list_rows: Vec<AnyElement> = Vec::new();

        if self.model.marketplace_searching {
            list_rows.push(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child(if self.model.marketplace_recommended {
                        "Loading recommended extensions..."
                    } else {
                        "Searching Open VSX Marketplace..."
                    })
                    .into_any_element(),
            );
        } else if let Some(err) = &self.model.marketplace_message {
            list_rows.push(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(rgb(0xe5534b))
                    .child(SharedString::from(err.clone()))
                    .into_any_element(),
            );
        } else if !self.model.marketplace_results.is_empty() {
            list_rows.push(
                div()
                    .px_3()
                    .py(px(4.))
                    .bg(rgb(theme::PANEL))
                    .text_size(px(11.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(theme::TEXT_DIM))
                    .child(SharedString::from(format!(
                        "{} ({})",
                        if self.model.marketplace_recommended {
                            "RECOMMENDED"
                        } else {
                            "MARKETPLACE"
                        },
                        self.model.marketplace_results.len()
                    )))
                    .into_any_element(),
            );
            for (i, m) in self.model.marketplace_results.iter().enumerate() {
                let is_installed = self
                    .model
                    .extensions
                    .iter()
                    .any(|e| e.id == m.id || e.id == m.name);
                let download_url = m.download_url.clone();
                let stats = if m.downloads > 0 {
                    format!("{}  ·  {}", m.publisher, short_count(m.downloads))
                } else {
                    m.publisher.clone()
                };
                list_rows.push(
                    div()
                        .id(("mkt-ext", i))
                        .flex()
                        .gap_3()
                        .px_3()
                        .py(px(10.))
                        .cursor_pointer()
                        .border_b_1()
                        .border_color(rgb(theme::BORDER))
                        .hover(|s| s.bg(rgb(theme::HOVER)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.model.open_market_detail(i);
                            cx.notify();
                        }))
                        .child(self.ext_icon(&m.id, None, &m.display_name, 42.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .child(SharedString::from(m.display_name.clone())),
                                )
                                .children((!m.description.is_empty()).then(|| {
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(theme::TEXT_DIM))
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .child(SharedString::from(m.description.clone()))
                                }))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .gap_2()
                                        .pt(px(2.))
                                        .child(
                                            div()
                                                .min_w_0()
                                                .text_size(px(11.))
                                                .text_color(rgb(theme::TEXT_DIM))
                                                .whitespace_nowrap()
                                                .overflow_hidden()
                                                .child(SharedString::from(stats)),
                                        )
                                        .child(if is_installed {
                                            div()
                                                .flex_none()
                                                .text_size(px(11.))
                                                .text_color(rgb(0x81b88b))
                                                .child("Installed")
                                                .into_any_element()
                                        } else if self.model.installing.as_deref()
                                            == Some(download_url.as_str())
                                        {
                                            div()
                                                .flex_none()
                                                .text_size(px(11.))
                                                .text_color(rgb(theme::TEXT_DIM))
                                                .child(SharedString::from(
                                                    self.model.install_label(),
                                                ))
                                                .into_any_element()
                                        } else {
                                            Self::flat_button(("mkt-install-btn", i), "Install")
                                                .flex_none()
                                                .bg(rgb(0x2f2f2f))
                                                .text_color(rgb(theme::TEXT))
                                                .on_click(cx.listener(
                                                    move |this, _: &ClickEvent, _w, cx| {
                                                        cx.stop_propagation();
                                                        this.model.install_from_marketplace(
                                                            &download_url,
                                                        );
                                                        cx.notify();
                                                    },
                                                ))
                                                .into_any_element()
                                        }),
                                ),
                        )
                        .into_any_element(),
                );
            }
        } else if let Some(q) = &self.model.marketplace_query {
            list_rows.push(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child(SharedString::from(format!(
                        "No extensions found for \"{q}\" on Open VSX."
                    )))
                    .into_any_element(),
            );
        } else {
            list_rows.push(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("Type a name above and press Enter to search the Open VSX marketplace.")
                    .into_any_element(),
            );
        }

        list_rows.push(
            div()
                .px_3()
                .py(px(4.))
                .bg(rgb(theme::PANEL))
                .text_size(px(11.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(theme::TEXT_DIM))
                .child(SharedString::from(format!(
                    "INSTALLED ({})",
                    self.model.extensions.len()
                )))
                .into_any_element(),
        );

        if self.model.extensions.is_empty() {
            list_rows.push(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("No extensions installed. Search above or install a .vsix package.")
                    .into_any_element(),
            );
        } else {
            for (i, x) in self.model.extensions.iter().enumerate() {
                let active = self.model.panel_extension.as_deref() == Some(x.dir.as_path());
                let removable = installed_root.as_deref() == x.dir.parent();
                let (dir_use, dir_rm) = (x.dir.clone(), x.dir.clone());
                let view = x.view_type.clone();
                let mut actions = div().flex().gap_1().pt_1();
                if let Some(view) = view {
                    actions = actions.child(
                        Self::flat_button(("ext-use", i), "Use in right panel").on_click(
                            cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                cx.stop_propagation();
                                this.model.use_as_panel_extension(&dir_use, &view);
                                cx.notify();
                            }),
                        ),
                    );
                }
                if removable {
                    actions = actions.child(Self::flat_button(("ext-rm", i), "Remove").on_click(
                        cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            cx.stop_propagation();
                            this.model.confirm_remove_extension(&dir_rm);
                            cx.notify();
                        }),
                    ));
                }
                list_rows.push(
                    div()
                        .id(("ext", i))
                        .flex()
                        .gap_3()
                        .px_3()
                        .py_2()
                        .cursor_pointer()
                        .border_b_1()
                        .border_color(rgb(theme::BORDER))
                        .hover(|s| s.bg(rgb(theme::HOVER)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.model.open_installed_detail(i);
                            cx.notify();
                        }))
                        .child(self.ext_icon(&x.id, x.icon.as_ref(), &x.display_name, 40.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    div()
                                        .flex()
                                        .justify_between()
                                        .text_size(px(13.))
                                        .child(SharedString::from(x.display_name.clone()))
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(rgb(theme::TEXT_DIM))
                                                .child(SharedString::from(x.version.clone())),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(theme::TEXT_DIM))
                                        .child(SharedString::from(format!(
                                            "{}{}{}",
                                            x.id,
                                            if x.commands > 0 {
                                                format!("  ·  {} commands", x.commands)
                                            } else {
                                                String::new()
                                            },
                                            if x.engine.is_empty() {
                                                String::new()
                                            } else {
                                                format!("  ·  vscode {}", x.engine)
                                            }
                                        ))),
                                )
                                .children((!x.description.is_empty()).then(|| {
                                    div()
                                        .pt_1()
                                        .text_size(px(12.))
                                        .child(SharedString::from(x.description.clone()))
                                }))
                                .children(active.then(|| {
                                    div()
                                        .pt_1()
                                        .text_size(px(11.))
                                        .text_color(rgb(0x81b88b))
                                        .child("● running in the right panel")
                                }))
                                .child(actions),
                        )
                        .into_any_element(),
                );
            }
        }

        let folder = installed_root.as_ref().map_or_else(
            || "no configuration folder".to_owned(),
            |p| p.display().to_string(),
        );

        let search_field = self.field_widget(
            window,
            cx,
            "ext-search-query",
            &self.model.ext_query,
            self.model.ext_query_focused,
            "Search Marketplace (Enter)",
            &self.ext_search_focus,
            Target::ExtSearch,
        );

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(div().px_2().pb_2().child(search_field))
            .child(
                div()
                    .flex()
                    .gap_1()
                    .px_2()
                    .pb_1()
                    .child(
                        Self::flat_button("ext-install", "Install from VSIX...").on_click(
                            cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::InstallVsix, window, cx)
                            }),
                        ),
                    )
                    .child(
                        Self::flat_button("ext-refresh", "Refresh").on_click(cx.listener(
                            |this, _: &ClickEvent, _w, cx| {
                                this.model.refresh_extensions();
                                cx.notify();
                            },
                        )),
                    ),
            )
            .child(self.scroll_list("ext-list", &self.scrolls.ext, list_rows, None, cx))
            .child(
                div()
                    .flex_none()
                    .px_3()
                    .py_2()
                    .text_size(px(11.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child(SharedString::from(format!("Folder: {folder}"))),
            )
            .into_any_element()
    }

    pub(crate) fn problems_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let files = &self.model.problems.files;
        if files.is_empty() {
            return div()
                .px_3()
                .py_2()
                .text_size(px(13.))
                .text_color(rgb(theme::TEXT_DIM))
                .child("No syntax problems in the open files. (Syntax check only: no type errors or lints.)")
                .into_any_element();
        }
        let mut rows: Vec<AnyElement> = Vec::new();
        let mut id = 0usize;
        for f in files {
            rows.push(
                div()
                    .px_3()
                    .py(px(3.))
                    .text_size(px(12.))
                    .child(SharedString::from(format!(
                        "{}  ({})",
                        f.title,
                        f.items.len()
                    )))
                    .into_any_element(),
            );
            for d in &f.items {
                let (path, line, col, len) = (f.path.clone(), d.line, d.col, d.len);
                rows.push(
                    div()
                        .id(("problem", id))
                        .flex()
                        .gap_2()
                        .pl_6()
                        .pr_3()
                        .py(px(2.))
                        .text_size(px(12.))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(theme::HOVER)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.model.open_at(&path, line, col, len);
                            this.focus_editor(window);
                            this.reveal_cursor();
                            cx.notify();
                        }))
                        .child(div().text_color(rgb(0xe5534b)).child("✖"))
                        .child(SharedString::from(d.message.clone()))
                        .child(
                            div()
                                .text_color(rgb(theme::TEXT_DIM))
                                .child(SharedString::from(format!(
                                    "[Ln {}, Col {}]",
                                    d.line + 1,
                                    d.col + 1
                                ))),
                        )
                        .into_any_element(),
                );
                id += 1;
            }
        }
        self.scroll_list("problems-list", &self.scrolls.problems, rows, None, cx)
    }

    pub(crate) fn output_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let log = &self.model.output;
        let lines: Vec<AnyElement> = log
            .lines()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|l| {
                div()
                    .flex()
                    .gap_3()
                    .px_3()
                    .text_size(px(12.))
                    .font_family(crate::FONT_FAMILY)
                    .child(
                        div()
                            .flex_none()
                            .text_color(rgb(theme::LINE_NUMBER))
                            .child(SharedString::from(crate::output::OutputLog::stamp(l))),
                    )
                    .child(
                        div()
                            .flex_none()
                            .w(px(76.))
                            .text_color(rgb(theme::ACCENT))
                            .child(SharedString::from(l.channel.clone())),
                    )
                    .child(
                        div()
                            .whitespace_nowrap()
                            .child(SharedString::from(l.text.clone())),
                    )
                    .into_any_element()
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .pb_1()
                    .text_size(px(11.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child(SharedString::from(format!(
                        "{} line(s), newest first",
                        log.len()
                    )))
                    .child(
                        Self::flat_button("output-clear", "Clear").on_click(cx.listener(
                            |this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::ClearOutput, window, cx)
                            },
                        )),
                    ),
            )
            .child(self.scroll_list("output-list", &self.scrolls.output, lines, None, cx))
            .into_any_element()
    }

    pub(crate) fn panel_body(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        match self.model.panel_tab {
            PanelTab::Terminal => {
                let mut b = div().flex().flex_1().min_h_0();
                b = b.child(self.terminal_view(window, cx));
                if self.terminals.len() > 1 {
                    b = b.child(self.terminal_side_tabs(cx));
                }
                b.into_any_element()
            }
            PanelTab::Problems => self.problems_view(cx),
            PanelTab::Output => self.output_view(cx),
        }
    }

    pub(crate) fn no_folder_view(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .items_start()
            .gap_2()
            .px_3()
            .text_size(px(12.))
            .text_color(rgb(theme::TEXT_DIM))
            .child("You have not opened a folder yet.")
            .child(
                Self::scm_button("explorer-open-folder", "Open Folder", true).on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.apply_command(Command::OpenFolder, window, cx);
                    }),
                ),
            )
            .into_any_element()
    }

    pub(crate) fn welcome(&self, cx: &mut Context<Self>) -> AnyElement {
        let entries: [(&'static str, Command); 8] = [
            ("Open Folder", Command::OpenFolder),
            ("Go to File", Command::QuickOpen),
            ("New File", Command::NewFile),
            ("Command Palette", Command::CommandPalette),
            ("Find in Files", Command::FindInFiles),
            ("Source Control", Command::ShowSourceControl),
            ("Open Terminal", Command::ToggleTerminal),
            ("About nanti.", Command::About),
        ];
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .font_family("Segoe UI")
            .child(div().mb_3().child(assets::logo_mark(72., 0xffffff)))
            .child(
                div()
                    .text_size(px(32.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xffffff))
                    .child("nanti."),
            )
            .child(
                div()
                    .pb_4()
                    .text_size(px(14.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("Code now. Not later."),
            )
            .children(entries.into_iter().enumerate().map(|(i, (label, cmd))| {
                div()
                    .id(("welcome", i))
                    .flex()
                    .justify_between()
                    .gap_8()
                    .w(px(320.))
                    .px_3()
                    .py(px(6.))
                    .rounded_sm()
                    .cursor_pointer()
                    .text_size(px(13.))
                    .hover(|s| s.bg(rgb(theme::HOVER)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.apply_command(cmd, window, cx)
                    }))
                    .child(div().text_color(rgb(theme::ACCENT)).child(label))
                    .child(
                        div()
                            .text_color(rgb(theme::TEXT_DIM))
                            .child(cmd.keybinding().unwrap_or("")),
                    )
            }))
            .into_any_element()
    }
}
