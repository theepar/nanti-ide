use gpui::{
    AnyElement, ClickEvent, Context, FontWeight, IntoElement, ObjectFit, SharedString, Window, div,
    img, prelude::*, px, rgb,
};

use crate::model::DetailTab;
use crate::palette::Command;
use crate::{Target, Workbench, theme};

pub(crate) fn short_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

pub(crate) fn parse_link(b: &[char], open: usize) -> Option<(String, usize)> {
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

pub(crate) fn strip_markup(s: &str) -> String {
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

pub(crate) fn readme_blocks(text: &str) -> Vec<AnyElement> {
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

impl Workbench {
    pub(crate) fn ext_icon(
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

    pub(crate) fn detail_button(
        id: impl Into<gpui::ElementId>,
        label: &'static str,
        danger: bool,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .flex()
            .items_center()
            .h(px(30.))
            .px_4()
            .rounded_md()
            .cursor_pointer()
            .border_1()
            .border_color(rgb(if danger { 0x7a3a3a } else { 0x5a5a5a }))
            .bg(rgb(0x2a2a2a))
            .text_size(px(13.))
            .text_color(rgb(if danger { 0xf0a8a0 } else { 0xe6e6e6 }))
            .hover(|s| s.bg(rgb(if danger { 0x4a2222 } else { 0x3a3a3a })))
            .child(label)
    }

    pub(crate) fn manifest_note(text: String, color: u32) -> AnyElement {
        div()
            .text_size(px(13.))
            .text_color(rgb(color))
            .child(SharedString::from(text))
            .into_any_element()
    }

    pub(crate) fn features_tab(&self, manifest: &crate::extensions::ManifestState) -> AnyElement {
        use crate::extensions::ManifestState;
        let info = match manifest {
            ManifestState::Loading => {
                return Self::manifest_note("Loading the features...".into(), theme::TEXT_DIM);
            }
            ManifestState::Failed(e) => return Self::manifest_note(e.clone(), 0xe5534b),
            ManifestState::Ready(info) => info,
        };
        if info.features.is_empty() {
            return Self::manifest_note(
                "This extension does not list any contributions.".into(),
                theme::TEXT_DIM,
            );
        }
        const SHOWN: usize = 60;
        div()
            .flex()
            .flex_col()
            .gap_4()
            .max_w(px(900.))
            .children(info.features.iter().map(|g| {
                let more = g.items.len().saturating_sub(SHOWN);
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(SharedString::from(format!(
                                "{} ({})",
                                g.title,
                                g.items.len()
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .rounded_md()
                            .bg(rgb(0x202020))
                            .border_1()
                            .border_color(rgb(theme::BORDER))
                            .children(g.items.iter().take(SHOWN).map(|item| {
                                div()
                                    .px_3()
                                    .py(px(3.))
                                    .text_size(px(12.))
                                    .text_color(rgb(0xd0d0d0))
                                    .child(SharedString::from(item.clone()))
                            }))
                            .children((more > 0).then(|| {
                                div()
                                    .px_3()
                                    .py(px(3.))
                                    .text_size(px(11.))
                                    .text_color(rgb(theme::TEXT_DIM))
                                    .child(SharedString::from(format!("and {more} more")))
                            })),
                    )
            }))
            .into_any_element()
    }

    pub(crate) fn pack_tab(
        &self,
        manifest: &crate::extensions::ManifestState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use crate::extensions::ManifestState;
        let info = match manifest {
            ManifestState::Loading => {
                return Self::manifest_note("Loading the package...".into(), theme::TEXT_DIM);
            }
            ManifestState::Failed(e) => return Self::manifest_note(e.clone(), 0xe5534b),
            ManifestState::Ready(info) => info,
        };
        if info.pack.is_empty() && info.dependencies.is_empty() {
            return Self::manifest_note(
                "This extension is not an extension pack and has no dependencies.".into(),
                theme::TEXT_DIM,
            );
        }
        let section = |title: &'static str, ids: &[String], base: usize, cx: &mut Context<Self>| {
            let rows: Vec<AnyElement> = ids
                .iter()
                .enumerate()
                .map(|(i, id)| {
                    let installed = self
                        .model
                        .extensions
                        .iter()
                        .any(|e| e.id.eq_ignore_ascii_case(id));
                    let query = id.clone();
                    div()
                        .id(("pack-member", base + i))
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_3()
                        .py(px(5.))
                        .text_size(px(12.))
                        .when(!installed, |d| {
                            d.cursor_pointer().hover(|s| s.bg(rgb(theme::HOVER)))
                        })
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            if installed {
                                return;
                            }
                            this.model.close_ext_detail();
                            this.model.ext_query.clear();
                            this.model.ext_query.insert(&query);
                            this.model.search_marketplace(&query);
                            cx.notify();
                        }))
                        .child(SharedString::from(id.clone()))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(if installed { 0x8fd19e } else { theme::TEXT_DIM }))
                                .child(if installed {
                                    "Installed"
                                } else {
                                    "Find in marketplace"
                                }),
                        )
                        .into_any_element()
                })
                .collect();
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(SharedString::from(format!("{title} ({})", ids.len()))),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .rounded_md()
                        .bg(rgb(0x202020))
                        .border_1()
                        .border_color(rgb(theme::BORDER))
                        .children(rows),
                )
                .into_any_element()
        };
        let mut parts: Vec<AnyElement> = Vec::new();
        if !info.pack.is_empty() {
            parts.push(section("Extensions in this pack", &info.pack, 0, cx));
        }
        if !info.dependencies.is_empty() {
            parts.push(section("Dependencies", &info.dependencies, 1000, cx));
        }
        div()
            .flex()
            .flex_col()
            .gap_4()
            .max_w(px(900.))
            .children(parts)
            .into_any_element()
    }

    pub(crate) fn ext_detail_page(
        &self,
        d: &crate::extensions::ExtDetail,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use crate::extensions::Readme;
        let installed = self.model.extensions.iter().find(|e| {
            e.id.eq_ignore_ascii_case(&d.id) || e.display_name.eq_ignore_ascii_case(&d.display_name)
        });
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
            let in_panel = self.model.panel_extension.as_deref() == Some(x.dir.as_path());
            let view = x.view_type.clone();
            let panel_dir = x.dir.clone();
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .h(px(30.))
                        .px_3()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(0x3f6b49))
                        .bg(rgb(0x1f2f24))
                        .text_size(px(13.))
                        .text_color(rgb(0x8fd19e))
                        .child("Installed"),
                )
                .children(view.filter(|_| !in_panel).map(|view| {
                    Self::detail_button("ext-detail-panel", "Use as extension panel", false)
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.model.use_as_panel_extension(&panel_dir, &view);
                            this.drain_requests(window, cx);
                            cx.notify();
                        }))
                }))
                .children(in_panel.then(|| {
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(0x8fd19e))
                        .child("Showing as the extension panel")
                }))
                .children(removable.then(|| {
                    Self::detail_button("ext-detail-remove", "Uninstall", true).on_click(
                        cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.model.confirm_remove_extension(&dir);
                            cx.notify();
                        }),
                    )
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

        let current = self.model.ext_detail_tab;
        let tabs = div()
            .flex()
            .gap_6()
            .border_b_1()
            .border_color(rgb(theme::BORDER))
            .children(
                [
                    (DetailTab::Details, "DETAILS"),
                    (DetailTab::Features, "FEATURES"),
                    (DetailTab::Pack, "EXTENSION PACK"),
                ]
                .into_iter()
                .enumerate()
                .map(|(i, (tab, label))| {
                    let active = tab == current;
                    div()
                        .id(("ext-tab", i))
                        .pb(px(6.))
                        .cursor_pointer()
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(if active { 0xffffff } else { theme::TEXT_DIM }))
                        .border_b_2()
                        .border_color(if active {
                            rgb(theme::ACCENT)
                        } else {
                            rgb(theme::EDITOR)
                        })
                        .hover(|s| s.text_color(rgb(0xffffff)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.model.ext_detail_tab = tab;
                            cx.notify();
                        }))
                        .child(label)
                }),
            );
        let body: AnyElement = match current {
            DetailTab::Details => div()
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
                .child(facts)
                .into_any_element(),
            DetailTab::Features => self.features_tab(&d.manifest),
            DetailTab::Pack => self.pack_tab(&d.manifest, cx),
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
            .child(tabs)
            .child(body);

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

    fn render_marketplace_item(
        &self,
        i: usize,
        m: &crate::extensions::MarketplaceExtension,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_installed = self.model.extensions.iter().any(|e| {
            e.id.eq_ignore_ascii_case(&m.id)
                || e.id.eq_ignore_ascii_case(&m.name)
                || e.display_name.eq_ignore_ascii_case(&m.display_name)
        });
        let download_url = m.download_url.clone();
        let stats = if m.downloads > 0 {
            format!("{}  ·  {}", m.publisher, short_count(m.downloads))
        } else {
            m.publisher.clone()
        };
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
                                    .child(SharedString::from(self.model.install_label()))
                                    .into_any_element()
                            } else {
                                Self::flat_button(("mkt-install-btn", i), "Install")
                                    .flex_none()
                                    .bg(rgb(0x2f2f2f))
                                    .text_color(rgb(theme::TEXT))
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                        cx.stop_propagation();
                                        this.model.install_from_marketplace(&download_url);
                                        cx.notify();
                                    }))
                                    .into_any_element()
                            }),
                    ),
            )
            .into_any_element()
    }

    fn recommended_rows(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let mut rows: Vec<AnyElement> = Vec::new();

        if self.model.marketplace_searching {
            rows.push(
                div()
                    .px_3()
                    .py(px(4.))
                    .bg(rgb(theme::PANEL))
                    .text_size(px(11.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("RECOMMENDED")
                    .into_any_element(),
            );
            rows.push(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("Loading recommendations from Open VSX...")
                    .into_any_element(),
            );
        } else if !self.model.marketplace_results.is_empty() {
            rows.push(
                div()
                    .px_3()
                    .py(px(4.))
                    .bg(rgb(theme::PANEL))
                    .text_size(px(11.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(theme::TEXT_DIM))
                    .child(SharedString::from(format!(
                        "RECOMMENDED ({})",
                        self.model.marketplace_results.len()
                    )))
                    .into_any_element(),
            );
            for (i, m) in self.model.marketplace_results.iter().enumerate() {
                rows.push(self.render_marketplace_item(i, m, cx));
            }
        } else if let Some(err) = &self.model.marketplace_message {
            rows.push(
                div()
                    .px_3()
                    .py(px(4.))
                    .bg(rgb(theme::PANEL))
                    .text_size(px(11.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(theme::TEXT_DIM))
                    .child("RECOMMENDED")
                    .into_any_element(),
            );
            rows.push(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(rgb(0xe5534b))
                    .child(SharedString::from(err.clone()))
                    .into_any_element(),
            );
        }
        rows
    }

    pub(crate) fn extensions_panel(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let mut list_rows: Vec<AnyElement> = Vec::new();
        let query = self.model.ext_query.text().trim().to_owned();

        if !query.is_empty() {
            if self.model.marketplace_searching && self.model.marketplace_results.is_empty() {
                list_rows.push(
                    div()
                        .px_3()
                        .py_2()
                        .text_size(px(12.))
                        .text_color(rgb(theme::TEXT_DIM))
                        .child("Searching Open VSX Marketplace...")
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
                            } else if self.model.marketplace_searching {
                                "UPDATING"
                            } else {
                                "MARKETPLACE"
                            },
                            self.model.marketplace_results.len()
                        )))
                        .into_any_element(),
                );
                for (i, m) in self.model.marketplace_results.iter().enumerate() {
                    list_rows.push(self.render_marketplace_item(i, m, cx));
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
            }
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
                list_rows.push(
                    div()
                        .id(("ext", i))
                        .flex()
                        .items_center()
                        .gap_3()
                        .px_3()
                        .my(px(6.))
                        .h(px(56.))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(theme::HOVER)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.model.open_installed_detail(i);
                            cx.notify();
                        }))
                        .child(self.ext_icon(&x.id, x.icon.as_ref(), &x.display_name, 36.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .gap(px(1.))
                                .child(
                                    div()
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(SharedString::from(x.display_name.clone())),
                                )
                                .child(
                                    div()
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .text_size(px(11.))
                                        .text_color(rgb(theme::TEXT_DIM))
                                        .child(SharedString::from(x.description.clone())),
                                )
                                .child(
                                    div()
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .text_size(px(11.))
                                        .text_color(rgb(0xb0b0b0))
                                        .child(SharedString::from(x.publisher.clone())),
                                ),
                        )
                        .children(active.then(|| {
                            div()
                                .flex_none()
                                .size(px(8.))
                                .rounded_full()
                                .bg(rgb(0x81b88b))
                        }))
                        .into_any_element(),
                );
            }
        }

        if query.is_empty() {
            list_rows.extend(self.recommended_rows(cx));
        }

        let search_field = self.field_widget(
            window,
            cx,
            "ext-search-query",
            &self.model.ext_query,
            self.model.ext_query_focused,
            "Search Extensions in Marketplace...",
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
                    .gap_2()
                    .px_2()
                    .pb_2()
                    .child(
                        Self::outline_button("ext-install", "Install from VSIX").on_click(
                            cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.apply_command(Command::InstallVsix, window, cx)
                            }),
                        ),
                    )
                    .child(
                        Self::outline_button("ext-refresh", "Refresh").on_click(cx.listener(
                            |this, _: &ClickEvent, _w, cx| {
                                this.model.refresh_extensions();
                                cx.notify();
                            },
                        )),
                    ),
            )
            .child(self.scroll_list("ext-list", &self.scrolls.ext, list_rows, None, cx))
            .into_any_element()
    }
}
