use gpui::{
    AnyElement, ClickEvent, Context, FontWeight, IntoElement, SharedString, div, prelude::*, px,
    rgb,
};

use crate::palette::Command;
use crate::{Workbench, assets, theme};

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
            .w_full()
            .max_w(px(820.))
            .py(px(14.))
            .border_b_1()
            .border_color(rgb(0x363636))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(3.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0xe8e8e8))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(rgb(0xa8a8a8))
                            .child(hint.into()),
                    ),
            )
            .child(div().flex_none().child(control))
            .into_any_element()
    }

    fn discord_account_card(&self, cx: &mut Context<Self>) -> AnyElement {
        use crate::discord::DiscordStatus;
        const AVATARS: [u32; 6] = [0x5865f2, 0x3ba55d, 0xc9811a, 0xc2377f, 0xc03537, 0x2f9e6e];
        let status = self.model.discord.status();
        // While a refresh reconnects, the account stays on screen instead of looking logged out.
        let refreshing = matches!(status, DiscordStatus::Connecting)
            .then(|| self.model.discord.last_profile.clone())
            .flatten();
        let connected = status.connected() || refreshing.is_some();
        let (title, line) = match (&status, &refreshing) {
            (DiscordStatus::Connected(profile), _) => (
                profile.name.clone(),
                "Signed in on the Discord app on this computer".to_owned(),
            ),
            (_, Some(profile)) => (profile.name.clone(), "Refreshing...".to_owned()),
            (other, None) => ("Discord".to_owned(), other.describe()),
        };
        let hue = AVATARS[title.bytes().map(usize::from).sum::<usize>() % AVATARS.len()];
        let photo = connected
            .then(|| self.model.discord.avatar_path.clone())
            .flatten();
        let letter = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(46.))
            .rounded_full()
            .bg(rgb(if connected { hue } else { 0x3a3a3a }))
            .text_size(px(19.))
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(0xffffff))
            .child(if connected {
                div()
                    .child(SharedString::from(
                        title
                            .chars()
                            .next()
                            .map_or(String::from("?"), |c| c.to_uppercase().to_string()),
                    ))
                    .into_any_element()
            } else {
                assets::icon("person", 24., 0xb0b0b0).into_any_element()
            });
        let avatar: AnyElement = match photo {
            Some(path) => gpui::img(path)
                .flex_none()
                .size(px(46.))
                .rounded_full()
                .object_fit(gpui::ObjectFit::Cover)
                .into_any_element(),
            None => letter.into_any_element(),
        };
        let buttons = if connected {
            div()
                .flex()
                .gap_2()
                .child(
                    Self::scm_button("set-discord-refresh", "Refresh", true).on_click(cx.listener(
                        |this, _: &ClickEvent, window, cx| {
                            this.apply_command(Command::DiscordConnect, window, cx);
                        },
                    )),
                )
                .child(
                    Self::scm_button("set-discord-logout", "Log out", true).on_click(cx.listener(
                        |this, _: &ClickEvent, window, cx| {
                            this.apply_command(Command::DiscordLogout, window, cx);
                        },
                    )),
                )
        } else {
            div().flex().gap_2().child(
                Self::scm_button("set-discord-connect", "Connect", true).on_click(cx.listener(
                    |this, _: &ClickEvent, window, cx| {
                        this.apply_command(Command::DiscordConnect, window, cx);
                    },
                )),
            )
        };
        div()
            .flex()
            .items_center()
            .gap_4()
            .w_full()
            .max_w(px(820.))
            .py(px(14.))
            .border_b_1()
            .border_color(rgb(0x363636))
            .child(avatar)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(px(3.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xf0f0f0))
                            .child(SharedString::from(title)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_size(px(12.5))
                            .text_color(rgb(0xa8a8a8))
                            .child(
                                div()
                                    .flex_none()
                                    .size(px(8.))
                                    .rounded_full()
                                    .bg(rgb(if connected { 0x3ba55d } else { 0x6b6b6b })),
                            )
                            .child(SharedString::from(line)),
                    ),
            )
            .child(div().flex_none().child(buttons))
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
            .gap(px(2.))
            .p(px(3.))
            .rounded_full()
            .bg(rgb(0x2b2b2b))
            .children(
                options
                    .into_iter()
                    .enumerate()
                    .map(|(i, (label, on, command))| {
                        div()
                            .id((id, i))
                            .flex()
                            .items_center()
                            .h(px(26.))
                            .px_3()
                            .rounded_full()
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
                .w_full()
                .max_w(px(820.))
                .pt_8()
                .pb_2()
                .border_b_1()
                .border_color(rgb(0x4a4a4a))
                .text_size(px(12.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(0xf2f2f2))
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
                    .child(section("EDITOR"))
                    .child(Self::setting_row(
                        "Format on save",
                        "Format the file with rustfmt, gofmt, dart format or Prettier right before it is saved.",
                        self.switch(
                            "set-format-on-save",
                            self.model.format_on_save,
                            Command::ToggleFormatOnSave,
                            cx,
                        ),
                    ))
                    .when(crate::discord::AVAILABLE, |el| {
                        el.child(section("DISCORD"))
                    .child(Self::setting_row(
                        "Rich Presence",
                        "Show what you are working on in your Discord status.",
                        self.switch(
                            "set-discord",
                            d.settings.enabled,
                            Command::ToggleDiscord,
                            cx,
                        ),
                    ))
                    .child(self.discord_account_card(cx))
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
                    })
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
