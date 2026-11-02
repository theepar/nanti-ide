//! Views added around the editor: menu bar and menus, explorer toolbar and outline, Run and
//! Extensions sidebars, Problems and Output panels, welcome screen. Every control calls a real
//! command or model operation; nothing here is decorative.

use gpui::{
    AnyElement, ClickEvent, Context, IntoElement, MouseButton, MouseDownEvent, SharedString,
    Window, div, prelude::*, px, rgb,
};

use crate::menu::{self, MenuItem, MenuKind};
use crate::model::PanelTab;
use crate::palette::Command;
use crate::{MENU_BAR_HEIGHT, Workbench, theme};

const BRAND_W: f32 = 58.0;
const MENU_W: f32 = 236.0;

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

impl Workbench {
    // ---- menu bar -------------------------------------------------------------------------------

    pub(crate) fn menu_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let root_name = self
            .model
            .root
            .file_name()
            .map_or_else(|| "nanti.".to_owned(), |n| n.to_string_lossy().into_owned());
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
                    .pl_3()
                    .text_size(px(13.))
                    .text_color(rgb(theme::ACCENT))
                    .child("nanti."),
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
                        theme::TEXT
                    } else {
                        theme::TEXT_DIM
                    }))
                    .when(open == Some(k), |d| d.bg(rgb(theme::SELECTED)))
                    .hover(|s| s.bg(rgb(theme::HOVER)).text_color(rgb(theme::TEXT)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        if this.model.menu_open == Some(k) {
                            this.model.close_menus();
                        } else {
                            this.model.open_menu(k);
                        }
                        cx.notify();
                    }))
                    // with a menu open, moving over another title switches to it (like desktop menus)
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
                    .flex()
                    .justify_center()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_color(rgb(theme::TEXT_DIM))
                    .child(SharedString::from(format!("{root_name} — nanti."))),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .pr_2()
                    .child(Self::layout_button(
                        "top-toggle-sidebar",
                        "▤",
                        self.model.layout.sidebar,
                        Command::ToggleSidebar,
                        cx,
                    ))
                    .child(Self::layout_button(
                        "top-toggle-panel",
                        "▭",
                        self.model.layout.panel,
                        Command::TogglePanel,
                        cx,
                    ))
                    .child(Self::layout_button(
                        "top-toggle-right",
                        "▯",
                        self.model.layout.right,
                        Command::ToggleRightPanel,
                        cx,
                    )),
            )
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
                    .gap_6()
                    .px_3()
                    .py(px(4.))
                    .text_size(px(12.))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(theme::ACCENT)).text_color(rgb(0xffffff)))
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

    /// Dropdown of the open menu or the explorer context menu, with a click-away backdrop.
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
                        .bg(rgb(theme::SIDEBAR))
                        .border_1()
                        .border_color(rgb(theme::BORDER))
                        .rounded_md()
                        .shadow_lg()
                        .text_color(rgb(theme::TEXT))
                        .children(rows),
                )
                .into_any_element(),
        )
    }

    // ---- explorer extras ---------------------------------------------------------------------------

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

    pub(crate) fn explorer_toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_none()
            .flex_wrap()
            .gap_1()
            .px_2()
            .pb_1()
            .child(
                Self::flat_button("ex-new-file", "+ File").on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| {
                        this.apply_command(Command::NewFile, window, cx)
                    },
                )),
            )
            .child(
                Self::flat_button("ex-new-folder", "+ Folder").on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| {
                        this.apply_command(Command::NewFolder, window, cx)
                    },
                )),
            )
            .child(
                Self::flat_button("ex-refresh", "Refresh").on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| {
                        this.apply_command(Command::RefreshExplorer, window, cx)
                    },
                )),
            )
            .child(
                Self::flat_button("ex-collapse", "Collapse").on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| {
                        this.apply_command(Command::CollapseAll, window, cx)
                    },
                )),
            )
            .into_any_element()
    }

    /// Collapsible symbol list of the active file under the explorer tree.
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
            .child(
                div()
                    .id("outline-list")
                    .max_h(px(190.))
                    .overflow_y_scroll()
                    .children(rows),
            )
            .into_any_element()
    }

    // ---- Run view ------------------------------------------------------------------------------------

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
            .child(div().id("task-list").flex_1().min_h_0().overflow_y_scroll().children(rows))
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

    // ---- Extensions view -----------------------------------------------------------------------------

    pub(crate) fn extensions_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let installed_root = self
            .model
            .config_dir
            .as_ref()
            .map(|c| crate::extensions::extensions_dir(c));
        let rows: Vec<AnyElement> = self
            .model
            .extensions
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let active = self.model.panel_extension.as_deref() == Some(x.dir.as_path());
                let removable = installed_root.as_deref() == x.dir.parent();
                let (dir_use, dir_rm) = (x.dir.clone(), x.dir.clone());
                let view = x.view_type.clone();
                let mut actions = div().flex().gap_1().pt_1();
                if let Some(view) = view {
                    actions = actions.child(
                        Self::flat_button(("ext-use", i), "Use in right panel").on_click(
                            cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.model.use_as_panel_extension(&dir_use, &view);
                                cx.notify();
                            }),
                        ),
                    );
                }
                if removable {
                    actions = actions.child(Self::flat_button(("ext-rm", i), "Remove").on_click(
                        cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.model.confirm_remove_extension(&dir_rm);
                            cx.notify();
                        }),
                    ));
                }
                div()
                    .id(("ext", i))
                    .flex()
                    .flex_col()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(theme::BORDER))
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
                    .child(actions)
                    .into_any_element()
            })
            .collect();
        let folder = installed_root.as_ref().map_or_else(
            || "no configuration folder".to_owned(),
            |p| p.display().to_string(),
        );
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
                    .child(Self::flat_button("ext-install", "Install from VSIX...").on_click(cx.listener(
                        |this, _: &ClickEvent, window, cx| this.apply_command(Command::InstallVsix, window, cx),
                    )))
                    .child(Self::flat_button("ext-refresh", "Refresh").on_click(cx.listener(
                        |this, _: &ClickEvent, _w, cx| {
                            this.model.refresh_extensions();
                            cx.notify();
                        },
                    ))),
            )
            .children(self.model.extensions.is_empty().then(|| {
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("No extensions installed. Install a .vsix package you own, or start with NANTI_EXTENSION.")
            }))
            .child(div().id("ext-list").flex_1().min_h_0().overflow_y_scroll().children(rows))
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

    // ---- bottom panel: Problems and Output -----------------------------------------------------------

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
        div()
            .id("problems-list")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .children(rows)
            .into_any_element()
    }

    pub(crate) fn output_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let log = &self.model.output;
        let lines: Vec<AnyElement> = log
            .lines()
            .collect::<Vec<_>>()
            .into_iter()
            .rev() // newest first: no scrolling to the bottom needed
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
            .child(
                div()
                    .id("output-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(lines),
            )
            .into_any_element()
    }

    pub(crate) fn panel_body(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        match self.model.panel_tab {
            PanelTab::Terminal => div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .child(self.terminal_strip(cx))
                .child(self.terminal_view(window, cx))
                .into_any_element(),
            PanelTab::Problems => self.problems_view(cx),
            PanelTab::Output => self.output_view(cx),
        }
    }

    // ---- welcome ----------------------------------------------------------------------------------------

    pub(crate) fn welcome(&self, cx: &mut Context<Self>) -> AnyElement {
        let entries: [(&'static str, Command); 7] = [
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
            .child(
                div()
                    .text_size(px(30.))
                    .text_color(rgb(theme::TEXT))
                    .child("nanti."),
            )
            .child(
                div()
                    .pb_4()
                    .text_size(px(13.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("Code now. Not later."),
            )
            .children(entries.into_iter().enumerate().map(|(i, (label, cmd))| {
                div()
                    .id(("welcome", i))
                    .flex()
                    .justify_between()
                    .gap_8()
                    .w(px(300.))
                    .px_3()
                    .py(px(5.))
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
