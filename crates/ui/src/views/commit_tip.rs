use std::time::Duration;

use gpui::{
    AnyElement, ClickEvent, ClipboardItem, Context, Entity, FontWeight, IntoElement, Pixels, Point,
    Render, SharedString, Subscription, Window, canvas, div, prelude::*, px, rgb,
};

use crate::{Workbench, assets, scm, theme};

const BODY_LINES: usize = 14;
// A fixed width lets long messages wrap to a known height, so the card grows downward instead of overlapping.
const CARD_WIDTH: f32 = 420.0;
// The first card waits so that sweeping the pointer across the list stays quiet; once it is up, moving to another row swaps it at once.
const SHOW_DELAY: Duration = Duration::from_millis(120);
const HIDE_DELAY: Duration = Duration::from_millis(220);
const PREFETCH_AROUND: usize = 4;

/// Where the card sits: beside the sidebar so it never covers the rows the pointer is moving across.
#[derive(Clone, Copy)]
pub(crate) struct CardAnchor {
    pub(crate) pointer_y: f32,
}

impl Workbench {
    pub(crate) fn hover_commit(&mut self, hash: &str, at: Point<Pixels>, cx: &mut Context<Self>) {
        // GPUI runs the handler of the row that gained the pointer before the one that lost it, so who holds the pointer is tracked by name.
        self.hovered_commit = Some(hash.to_owned());
        self.card_gen += 1;
        let gen_ = self.card_gen;
        let s = &mut self.model.scm;
        s.request_detail(hash);
        if let Some(i) = s.history.iter().position(|c| c.hash == hash) {
            let near: Vec<String> = s.history
                [i.saturating_sub(PREFETCH_AROUND)..(i + PREFETCH_AROUND + 1).min(s.history.len())]
                .iter()
                .map(|c| c.hash.clone())
                .collect();
            for h in near {
                s.request_detail(&h);
            }
        }
        let anchor = CardAnchor {
            pointer_y: f32::from(at.y),
        };
        let hash = hash.to_owned();
        if self.commit_card.is_some() {
            self.show_commit_card(&hash, anchor, cx);
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SHOW_DELAY).await;
            let _ = this.update(cx, |wb, cx| {
                if wb.card_gen == gen_ {
                    wb.show_commit_card(&hash, anchor, cx);
                }
            });
        })
        .detach();
    }

    fn show_commit_card(&mut self, hash: &str, anchor: CardAnchor, cx: &mut Context<Self>) {
        self.commit_card = Some(anchor);
        let tip = match &self.commit_tip {
            Some(t) => t.clone(),
            None => {
                let wb = cx.entity();
                let hash = hash.to_owned();
                let t = cx.new(|cx| CommitTip::new(wb, hash, cx));
                self.commit_tip = Some(t.clone());
                t
            }
        };
        tip.update(cx, |t, cx| t.set_hash(hash, cx));
        cx.notify();
    }

    pub(crate) fn leave_commit(&mut self, hash: &str, cx: &mut Context<Self>) {
        if self.hovered_commit.as_deref() == Some(hash) {
            self.hovered_commit = None;
            self.schedule_card_hide(cx);
        }
    }

    fn schedule_card_hide(&mut self, cx: &mut Context<Self>) {
        self.card_gen += 1;
        let gen_ = self.card_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(HIDE_DELAY).await;
            let _ = this.update(cx, |wb, cx| {
                let idle = wb.hovered_commit.is_none() && !wb.card_hovered;
                if wb.card_gen == gen_ && idle && wb.commit_card.take().is_some() {
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(crate) fn hover_card(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.card_hovered = hovered;
        if hovered {
            self.card_gen += 1;
        } else if self.hovered_commit.is_none() {
            self.schedule_card_hide(cx);
        }
    }

    pub(crate) fn commit_card_overlay(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let anchor = self.commit_card?;
        let tip = self.commit_tip.clone()?;
        let (vw, vh) = {
            let v = window.viewport_size();
            (f32::from(v.width), f32::from(v.height))
        };
        let layout = &self.model.layout;
        let side = crate::layout::ACTIVITY_BAR_WIDTH + layout.sidebar_width + 6.0;
        let left = match layout.sidebar_side {
            crate::layout::Side::Left => side,
            crate::layout::Side::Right => vw - side - CARD_WIDTH - 12.0,
        }
        .clamp(4.0, (vw - CARD_WIDTH - 12.0).max(4.0));
        let (slot, entity) = (self.card_bounds.clone(), cx.entity());
        let card = div()
            .id("commit-card")
            .absolute()
            .left(px(left))
            .on_hover(cx.listener(|this, hovered: &bool, _w, cx| {
                this.hover_card(*hovered, cx);
            }))
            .child(tip)
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, (), _window, cx| {
                        if slot.get() != Some(bounds) {
                            slot.set(Some(bounds));
                            let entity = entity.clone();
                            cx.defer(move |cx| entity.update(cx, |_, cx| cx.notify()));
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        Some(if anchor.pointer_y > vh * 0.55 {
            card.bottom(px((vh - anchor.pointer_y - 24.0).max(6.0)))
        } else {
            card.top(px((anchor.pointer_y - 14.0).max(6.0)))
        })
    }
}

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

    pub(crate) fn set_hash(&mut self, hash: &str, cx: &mut Context<Self>) {
        if self.hash != hash {
            self.hash = hash.to_owned();
            cx.notify();
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
