use gpui::{
    AnyElement, ClickEvent, Context, FontWeight, IntoElement, MouseButton, MouseDownEvent, Window,
    WindowControlArea, div, prelude::*, px, rgb,
};

use crate::assets;
use crate::menu::{self, MenuItem, MenuKind};
use crate::palette::Command;
use crate::{MENU_BAR_HEIGHT, Workbench, theme};

pub(crate) const BRAND_W: f32 = 38.0;
pub(crate) const MENU_W: f32 = 260.0;

pub(crate) fn button_w(kind: MenuKind) -> f32 {
    kind.title().chars().count() as f32 * 7.2 + 22.0
}

pub(crate) fn menu_left(kind: MenuKind) -> f32 {
    BRAND_W
        + MenuKind::ALL
            .iter()
            .take_while(|k| **k != kind)
            .map(|k| button_w(*k))
            .sum::<f32>()
}

impl Workbench {
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
            ("About nanti", Command::About),
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
                    .child("nanti"),
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
