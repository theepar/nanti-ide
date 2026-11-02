adong
use gpui::{
    AnyElement, ClickEvent, Context, FontWeight, IntoElement, SharedString, div, prelude::*, px,
    rgb,
};

use crate::palette::Command;
use crate::{Workbench, theme};

impl Workbench {
    pub(crate) fn setting_row(
        title: &'static str,
        hint: impl Into<SharedString>,
        control: AnyElement,
    ) -> AnyElement {
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
                            .child(hint.into()),
                    ),
            )
            .child(control)
            .into_any_element()
    }

    pub(crate) fn segmented(
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

    pub(crate) fn switch(
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
        let d = &self.model.discord;
        let discord_status = d.status().describe();
        let discord_app_hint = if d.settings.client_id.is_empty() {
            "Not set. Create a free application at discord.com/developers and paste its Application ID."
        } else {
            "Set. Names and images shown in Discord come from this application."
        };
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
                        "Terminal, problems and output. Hidden when nanti starts.",
                        self.switch("set-panel", l.panel, Command::TogglePanel, cx),
                    ))
                    .child(Self::setting_row(
                        "Status bar",
                        "The bar at the bottom of the window.",
                        self.switch("set-status", l.status_bar, Command::ToggleStatusBar, cx),
                    ))
                    .child(section("DISCORD"))
                    .child(Self::setting_row(
                        "Rich Presence",
                        "Show what you are working on in your Discord status. Off by default.",
                        self.switch(
                            "set-discord",
                            d.settings.enabled,
                            Command::ToggleDiscord,
                            cx,
                        ),
                    ))
                    .child(Self::setting_row(
                        "Account",
                        discord_status,
                        Self::flat_button("set-discord-reconnect", "Reconnect")
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::DiscordReconnect, window, cx);
                            }))
                            .into_any_element(),
                    ))
                    .child(Self::setting_row(
                        "Application ID",
                        discord_app_hint,
                        Self::flat_button("set-discord-id", "Set...")
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::DiscordSetApplicationId, window, cx);
                            }))
                            .into_any_element(),
                    ))
                    .child(Self::setting_row(
                        "Show the file name",
                        "The file you are editing and its language.",
                        self.switch(
                            "set-discord-file",
                            d.settings.show_file,
                            Command::ToggleDiscordFile,
                            cx,
                        ),
                    ))
                    .child(Self::setting_row(
                        "Show the workspace",
                        "The name of the folder you opened.",
                        self.switch(
                            "set-discord-workspace",
                            d.settings.show_workspace,
                            Command::ToggleDiscordWorkspace,
                            cx,
                        ),
                    ))
                    .child(Self::setting_row(
                        "Show elapsed time",
                        "How long this session has been open.",
                        self.switch(
                            "set-discord-time",
                            d.settings.show_time,
                            Command::ToggleDiscordTime,
                            cx,
                        ),
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
}
