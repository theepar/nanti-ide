use gpui::{
    AnyElement, ClickEvent, ClipboardItem, Context, Entity, FontWeight, IntoElement, Render,
    SharedString, Subscription, Window, div, prelude::*, px, rgb,
};

use crate::{Workbench, assets, scm, theme};

const BODY_LINES: usize = 14;
// A fixed width lets long messages wrap to a known height, so the card grows downward instead of overlapping.
const CARD_WIDTH: f32 = 420.0;

/// The hover card of a commit in the history: who, when, the whole message, the size of the change and links.
pub(crate) struct CommitTip {
    workbench: Entity<Workbench>,
    hash: String,
    _watch: Subscription,
}

impl CommitTip {
    pub(crate) fn new(workbench: Entity<Workbench>, hash: String, cx: &mut Context<Self>) -> Self {
        let _watch = cx.observe(&workbench, |_, _, cx| cx.notify());
        Self {
            workbench,
            hash,
            _watch,
        }
    }

    fn link_button(
        id: &'static str,
        icon: &'static str,
        label: impl Into<SharedString>,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .flex()
            .items_center()
            .gap_1()
            .px_1()
            .rounded_sm()
            .cursor_pointer()
            .text_color(rgb(0x3794ff))
            .hover(|s| s.bg(rgb(theme::HOVER)))
            .child(assets::icon(icon, 13., 0x3794ff))
            .child(label.into())
    }
}

impl Render for CommitTip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let card = div()
            .id("commit-tip")
            .occlude()
            .flex()
            .flex_col()
            .w(px(CARD_WIDTH))
            .gap(px(4.))
            .px_2()
            .py(px(6.))
            .bg(rgb(theme::MENU_BG))
            .border_1()
            .border_color(rgb(0x454545))
            .rounded_md()
            .shadow_lg()
            .text_size(px(13.))
            .text_color(rgb(theme::TEXT));
        let wb = self.workbench.read(cx);
        let s = &wb.model().scm;
        let Some(commit) = s.history.iter().find(|c| c.hash == self.hash) else {
            return card.child("This commit is no longer in the list.");
        };
        let detail = s.commit_details.get(&self.hash);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        let offset = detail.map_or(0, |d| d.offset_min);
        let when = format!(
            "{} ({})",
            scm::relative_long(now, commit.time),
            scm::format_commit_date(commit.time, offset)
        );
        let message = detail.map_or_else(|| commit.subject.clone(), |d| d.body.clone());
        let lines: Vec<&str> = message.lines().collect();
        let shown: Vec<AnyElement> = lines
            .iter()
            .take(BODY_LINES)
            .map(|l| {
                div()
                    .flex_none()
                    .min_h(px(4.))
                    .child(SharedString::from((*l).to_owned()))
                    .into_any_element()
            })
            .collect();
        let more = (lines.len() > BODY_LINES).then(|| {
            div()
                .text_color(rgb(theme::TEXT_DIM))
                .child(SharedString::from(format!(
                    "... {} more lines",
                    lines.len() - BODY_LINES
                )))
        });
        let stats: AnyElement = match detail {
            None => div()
                .flex_none()
                .text_color(rgb(theme::TEXT_DIM))
                .child("Loading the size of this change...")
                .into_any_element(),
            Some(d) if d.files == 0 => div()
                .flex_none()
                .text_color(rgb(theme::TEXT_DIM))
                .child("No file changes")
                .into_any_element(),
            Some(d) => div()
                .flex()
                .flex_none()
                .flex_wrap()
                .child(SharedString::from(format!(
                    "{} file{} changed",
                    d.files,
                    if d.files == 1 { "" } else { "s" }
                )))
                .children((d.insertions > 0).then(|| {
                    div()
                        .text_color(rgb(0x81b88b))
                        .child(SharedString::from(format!(
                            ", {} insertions(+)",
                            d.insertions
                        )))
                }))
                .children((d.deletions > 0).then(|| {
                    div()
                        .text_color(rgb(0xc74e39))
                        .child(SharedString::from(format!(
                            ", {} deletions(-)",
                            d.deletions
                        )))
                }))
                .into_any_element(),
        };
        let hash = commit.hash.clone();
        let copy_hash = commit.hash.clone();
        let short = commit.short().to_owned();
        let web = s.web_remote.clone();
        let open = web.as_ref().and_then(|w| {
            w.commit_url(&hash)
                .map(|url| (format!("Open on {}", w.host_label), url))
        });
        card.child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x3794ff))
                        .child(assets::icon("person", 14., 0x3794ff))
                        .child(SharedString::from(commit.author.clone())),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_color(rgb(theme::TEXT_DIM))
                        .child(assets::icon("clock", 14., theme::TEXT_DIM))
                        .child(SharedString::from(when)),
                ),
        )
        .child(
            div()
                .flex()
                .flex_none()
                .flex_col()
                .children(shown)
                .children(more),
        )
        .child(stats)
        .child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap_2()
                .pt(px(4.))
                .border_t_1()
                .border_color(rgb(0x3a3a3a))
                .child(
                    Self::link_button("commit-tip-copy", "copy", short).on_click(cx.listener(
                        move |_this, _: &ClickEvent, _w, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy_hash.clone()));
                        },
                    )),
                )
                .children(open.map(|(label, url)| {
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().text_color(rgb(theme::TEXT_DIM)).child("|"))
                        .child(
                            Self::link_button("commit-tip-open", "external", label).on_click(
                                cx.listener(move |_this, _: &ClickEvent, _w, cx| {
                                    if Workbench::valid_external_url(&url) {
                                        cx.open_url(&url);
                                    }
                                }),
                            ),
                        )
                })),
        )
    }
}
