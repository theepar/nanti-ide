use gpui::{AnyElement, ClickEvent, Context, IntoElement, SharedString, div, prelude::*, px, rgb};

use crate::palette::Command;
use crate::{Workbench, theme};

impl Workbench {
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
}

