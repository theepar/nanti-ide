use gpui::{
    AnyElement, ClickEvent, Context, FontWeight, IntoElement, SharedString, div, prelude::*, px,
    rgb,
};

use crate::palette::Command;
use crate::{Workbench, assets, theme};

/// One entry of a dropdown: its label, whether it is the current one, and what picking it does.
pub(crate) type DropdownOption = (SharedString, bool, Box<dyn Fn(&mut Workbench) + 'static>);

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

    /// A dropdown: the closed box shows the current choice, the open list scrolls so any number of entries fits.
    pub(crate) fn dropdown(
        &self,
        id: &'static str,
        current: &str,
        options: Vec<DropdownOption>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let open = self.model.dropdown_open == Some(id);
        let list = open.then(|| {
            gpui::deferred(
                div()
                    .id((id, 10_000usize))
                    .absolute()
                    .top(px(34.))
                    .right_0()
                    .w_full()
                    .max_h(px(260.))
                    .overflow_y_scroll()
                    .p(px(4.))
                    .rounded_md()
                    .bg(rgb(theme::MENU_BG()))
                    .border_1()
                    .border_color(rgb(0x4a4a4a))
                    .shadow_lg()
                    .occlude()
                    .children(
                        options
                            .into_iter()
                            .enumerate()
                            .map(|(i, (label, on, pick))| {
                                div()
                                    .id((id, i))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px_3()
                                    .py(px(6.))
                                    .rounded_sm()
                                    .text_size(px(12.5))
                                    .cursor_pointer()
                                    .text_color(rgb(if on { 0xffffff } else { theme::TEXT() }))
                                    .when(on, |d| {
                                        d.font_weight(FontWeight::MEDIUM).bg(rgb(theme::SELECTED()))
                                    })
                                    .hover(|s| s.bg(rgb(theme::HOVER_STRONG())))
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                        pick(this);
                                        this.model.dropdown_open = None;
                                        cx.notify();
                                    }))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .child(label),
                                    )
                            }),
                    ),
            )
            .with_priority(1)
        });
        div()
            .relative()
            .w(px(230.))
            // Clicks inside the button or the list stay inside this box, so only outside clicks close it.
            .on_mouse_down_out(cx.listener(move |this, _: &gpui::MouseDownEvent, _w, cx| {
                if this.model.dropdown_open == Some(id) {
                    this.model.dropdown_open = None;
                    cx.notify();
                }
            }))
            .child(
                div()
                    .id(id)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .h(px(30.))
                    .px_3()
                    .rounded_md()
                    .bg(rgb(0x2b2b2b))
                    .border_1()
                    .border_color(rgb(if open { 0x6a6a6a } else { 0x3c3c3c }))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0x333333)))
                    .text_size(px(12.5))
                    .text_color(rgb(0xe8e8e8))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.model.dropdown_open = if this.model.dropdown_open == Some(id) {
                            None
                        } else {
                            Some(id)
                        };
                        cx.notify();
                    }))
                    .child(
                        div()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(SharedString::from(current.to_owned())),
                    )
                    .child(assets::icon("chevron-down", 14., theme::ICON())),
            )
            .children(list)
            .into_any_element()
    }

    fn prettier_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        use crate::format::PrettierSource;
        let source = self.model.prettier_source.clone();
        let pick = |s: PrettierSource| -> Box<dyn Fn(&mut Workbench) + 'static> {
            Box::new(move |wb: &mut Workbench| wb.model.set_prettier_source(s.clone()))
        };
        let mut options: Vec<DropdownOption> = vec![(
            "Automatic".into(),
            source == PrettierSource::Auto,
            pick(PrettierSource::Auto),
        )];
        let mut current = "Automatic".to_owned();
        for ext in self
            .model
            .extensions
            .iter()
            .filter(|e| e.id.eq_ignore_ascii_case("esbenp.prettier-vscode"))
        {
            let Some(folder) = ext
                .dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
            else {
                continue;
            };
            let choice = PrettierSource::Extension(folder);
            let label = format!("Extension {} v{}", ext.display_name, ext.version);
            if source == choice {
                current = label.clone();
            }
            options.push((label.into(), source == choice, pick(choice)));
        }
        if crate::format::prettier_on_path() {
            if source == PrettierSource::Path {
                current = "Prettier on PATH".to_owned();
            }
            options.push((
                "Prettier on PATH".into(),
                source == PrettierSource::Path,
                pick(PrettierSource::Path),
            ));
        }
        self.dropdown("set-prettier", &current, options, cx)
    }

    fn icon_theme_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let active = self.model.icon_choice.active.clone();
        let mut options: Vec<DropdownOption> = vec![(
            "Built-in".into(),
            active.is_none(),
            Box::new(|wb: &mut Workbench| wb.pick_icon_theme(None)),
        )];
        let mut current = "Built-in".to_owned();
        for t in &self.model.icon_themes {
            let on = active.as_deref() == Some(t.key.as_str());
            if on {
                current = t.label.clone();
            }
            let key = t.key.clone();
            options.push((
                t.label.clone().into(),
                on,
                Box::new(move |wb: &mut Workbench| wb.pick_icon_theme(Some(key.clone()))),
            ));
        }
        self.dropdown("set-icon-theme", &current, options, cx)
    }

    pub fn pick_icon_theme(&mut self, key: Option<String>) {
        self.model.set_icon_theme(key);
        self.model.message = Some("File icon theme changed".to_owned());
    }

    fn discord_account_card(&self, cx: &mut Context<Self>) -> AnyElement {
        use crate::discord::DiscordStatus;
        const AVATARS: [u32; 6] = [0x5865f2, 0x3ba55d, 0xc9811a, 0xc2377f, 0xc03537, 0x2f9e6e];
        let status = self.model.discord.status();
        // The account stays on screen while Rich Presence is off or reconnecting; only Log out forgets it.
        let account = match &status {
            DiscordStatus::Connected(p) => Some(p.clone()),
            _ => self.model.discord.settings.profile.clone(),
        };
        let connected = status.connected();
        let (title, line) = match (&status, &account) {
            (DiscordStatus::Connected(_), Some(p)) => (
                p.name.clone(),
                "Signed in on the Discord app on this computer".to_owned(),
            ),
            (DiscordStatus::Off, Some(p)) => (p.name.clone(), "Rich Presence is off".to_owned()),
            (other, Some(p)) => (p.name.clone(), other.describe()),
            (other, None) => ("Discord".to_owned(), other.describe()),
        };
        let has_account = account.is_some();
        let hue = AVATARS[title.bytes().map(usize::from).sum::<usize>() % AVATARS.len()];
        let photo = has_account
            .then(|| self.model.discord.avatar_path.clone())
            .flatten();
        let letter = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(46.))
            .rounded_full()
            .bg(rgb(if has_account { hue } else { 0x3a3a3a }))
            .text_size(px(19.))
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(0xffffff))
            .child(if has_account {
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
                            this.apply_command(Command::DiscordRefresh, window, cx);
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
            let connect = Self::scm_button("set-discord-connect", "Connect", true).on_click(
                cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.apply_command(Command::DiscordConnect, window, cx);
                }),
            );
            div()
                .flex()
                .gap_2()
                .child(connect)
                .children(has_account.then(|| {
                    Self::scm_button("set-discord-logout", "Log out", true).on_click(cx.listener(
                        |this, _: &ClickEvent, window, cx| {
                            this.apply_command(Command::DiscordLogout, window, cx);
                        },
                    ))
                }))
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
                                d.bg(rgb(theme::ACCENT()))
                                    .text_color(rgb(0x1e1e1e))
                                    .font_weight(FontWeight::MEDIUM)
                            })
                            .when(!on, |d| {
                                d.text_color(rgb(theme::TEXT()))
                                    .hover(|s| s.bg(rgb(theme::HOVER())))
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
            .bg(rgb(if on { theme::ACCENT() } else { 0x3a3a3a }))
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
                            .text_color(rgb(theme::TEXT_DIM()))
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
                    .child(section("APPEARANCE"))
                    .child(Self::setting_row(
                        "File icon theme",
                        if self.model.icon_themes.is_empty() {
                            "Icons of files and folders. No icon theme extension is installed; install one such as Material Icon Theme and it shows up here."
                        } else {
                            "Icons of files and folders in the explorer and the tabs. A theme you install is switched on once; pick Built-in to go back."
                        },
                        self.icon_theme_picker(cx),
                    ))
                    .child(section("EDITOR"))
                    .child(Self::setting_row(
                        "Prettier",
                        "Which Prettier formats JavaScript, TypeScript, CSS, HTML, JSON, Markdown and YAML. Automatic uses the project's own, then an installed extension's, then the one on PATH; if the chosen one is gone, Automatic is used.",
                        self.prettier_picker(cx),
                    ))
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
                                    .text_color(rgb(theme::TEXT_DIM()))
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
            .bg(rgb(theme::EDITOR()))
            .child(self.scroll_box(
                "settings",
                page.into_any_element(),
                &self.scrolls.settings,
                cx,
            ))
            .into_any_element()
    }
}
