pub(crate) mod chrome;
pub(crate) mod extensions;
pub(crate) mod panels;
pub(crate) mod settings;

use gpui::{
    AnyElement, Bounds, Context, IntoElement, MouseButton, MouseDownEvent, Pixels, ScrollHandle,
    SharedString, Window, WindowControlArea, canvas, div, fill, point, prelude::*, px, rgb, size,
};

use crate::assets;
use crate::model::PanelTab;
use crate::{MENU_BAR_HEIGHT, Workbench, theme};

pub(crate) const SCROLLBAR_W: f32 = 8.0;
pub(crate) const SCROLLBAR_MIN_THUMB: f32 = 30.0;

pub(crate) fn thumb_geometry(handle: &ScrollHandle, track: Bounds<Pixels>) -> Option<(f32, f32)> {
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
                        let thumb = Bounds {
                            origin: point(track.origin.x + px(3.), px(top)),
                            size: size(px(SCROLLBAR_W - 5.), px(height)),
                        };
                        window.paint_quad(
                            fill(thumb, rgb(theme::SCROLL_THUMB)).corner_radii(px(1.5)),
                        );
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

    pub(crate) fn caption_button(
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

    pub(crate) fn outline_button(
        id: impl Into<gpui::ElementId>,
        label: &'static str,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .flex()
            .flex_1()
            .items_center()
            .justify_center()
            .h(px(28.))
            .rounded_md()
            .cursor_pointer()
            .border_1()
            .border_color(rgb(0x4a4a4a))
            .bg(rgb(0x2a2a2a))
            .text_size(px(12.))
            .text_color(rgb(0xe6e6e6))
            .hover(|s| s.bg(rgb(0x363636)).border_color(rgb(0x6a6a6a)))
            .child(label)
    }

    pub(crate) fn flat_button(
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

    pub(crate) fn icon_button(
        id: &'static str,
        icon_name: &'static str,
    ) -> gpui::Stateful<gpui::Div> {
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
}
