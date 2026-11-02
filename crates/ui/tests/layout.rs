//! Customizable layout: show/hide every region, drag the splitters, maximize the panel, persist.

use std::path::{Path, PathBuf};

use gpui::{
    Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, TestAppContext, VisualTestContext,
    WindowHandle, point, px,
};
use ui::Workbench;
use ui::layout::{Layout, PANEL_MIN, SIDEBAR_MIN};

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn workspace(tag: &str) -> Tmp {
    let root = std::env::temp_dir().join(format!("nanti-ui-lay-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("a.txt"), "x").unwrap();
    Tmp(root)
}

fn open<'a>(
    cx: &'a mut TestAppContext,
    root: &Path,
    layout_file: Option<PathBuf>,
) -> (WindowHandle<Workbench>, &'a mut VisualTestContext) {
    let root = root.to_path_buf();
    let path = root.join("a.txt");
    let window = cx.add_window(move |_w, cx| {
        let mut wb = Workbench::new(root, cx);
        if let Some(f) = layout_file {
            wb.set_layout_path(f);
        }
        wb.open_path(&path);
        wb
    });
    let vcx = VisualTestContext::from_window(window.into(), cx).into_mut();
    window
        .update(vcx, |wb, window, _| window.focus(wb.editor_focus()))
        .unwrap();
    vcx.run_until_parked();
    (window, vcx)
}

fn layout(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) -> Layout {
    window
        .read_with(cx, |wb, _| wb.model().layout.clone())
        .unwrap()
}

fn viewport(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) -> (f32, f32) {
    window
        .update(cx, |_, w, _| {
            let v = w.viewport_size();
            (f32::from(v.width), f32::from(v.height))
        })
        .unwrap()
}

fn drag(cx: &mut VisualTestContext, from: (f32, f32), to: (f32, f32)) {
    cx.simulate_mouse_down(
        point(px(from.0), px(from.1)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_move(
        point(px(to.0), px(to.1)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_up(
        point(px(to.0), px(to.1)),
        MouseButton::Left,
        Modifiers::default(),
    );
}

#[gpui::test]
fn shortcuts_and_palette_toggle_every_region_and_reset_restores_defaults(cx: &mut TestAppContext) {
    let ws = workspace("toggle");
    let (window, cx) = open(cx, &ws.0, None);
    assert_eq!(layout(window, cx), Layout::default());
    cx.simulate_keystrokes("ctrl-b"); // sidebar
    cx.simulate_keystrokes("ctrl-j"); // panel
    cx.simulate_keystrokes("ctrl-alt-b"); // right panel
    let l = layout(window, cx);
    assert!(!l.sidebar && !l.panel && !l.right);
    for query in ["toggle activity bar", "toggle status bar"] {
        cx.simulate_keystrokes("ctrl-shift-p");
        cx.simulate_input(query);
        cx.simulate_keystrokes("enter");
    }
    let l = layout(window, cx);
    assert!(!l.activity_bar && !l.status_bar);
    // bring everything back one by one
    cx.simulate_keystrokes("ctrl-b ctrl-j ctrl-alt-b");
    let l = layout(window, cx);
    assert!(l.sidebar && l.panel && l.right && !l.activity_bar && !l.status_bar);
    cx.simulate_keystrokes("ctrl-shift-p");
    cx.simulate_input("reset layout");
    cx.simulate_keystrokes("enter");
    assert_eq!(layout(window, cx), Layout::default());
}

#[gpui::test]
fn dragging_the_sidebar_splitter_resizes_clamps_and_ignores_moves_without_the_button(
    cx: &mut TestAppContext,
) {
    let ws = workspace("dsb");
    let (window, cx) = open(cx, &ws.0, None);
    let (_vw, vh) = viewport(window, cx);
    // activity bar 48 + sidebar 260 = 308; the splitter occupies [308, 313)
    drag(cx, (310.0, vh / 2.0), (410.0, vh / 2.0));
    let w = layout(window, cx).sidebar_width;
    assert!(
        (w - (410.0 - 48.0)).abs() < 1.0,
        "sidebar follows the pointer: {w}"
    );
    drag(cx, (48.0 + w + 2.0, vh / 2.0), (5.0, vh / 2.0));
    assert_eq!(
        layout(window, cx).sidebar_width,
        SIDEBAR_MIN,
        "clamped to the minimum"
    );
    // a mouse move with no button pressed must not resize anything
    let before = layout(window, cx);
    cx.simulate_mouse_move(point(px(500.0), px(vh / 2.0)), None, Modifiers::default());
    assert_eq!(layout(window, cx), before);
}

#[gpui::test]
fn dragging_the_right_and_bottom_splitters(cx: &mut TestAppContext) {
    let ws = workspace("drp");
    let (window, cx) = open(cx, &ws.0, None);
    // with no extension the right panel is not shown at all; lay it out empty for this test
    window
        .update(cx, |wb, _, cx| {
            wb.show_empty_right_panel_for_tests();
            cx.notify(); // redraw so the splitter exists before the drag
        })
        .unwrap();
    cx.run_until_parked();
    let (vw, vh) = viewport(window, cx);
    // right panel is 320 wide; its splitter sits just left of it
    drag(cx, (vw - 322.0, vh / 2.0), (vw - 420.0, vh / 2.0));
    let r = layout(window, cx).right_width;
    assert!(
        (r - 420.0).abs() < 1.0,
        "right panel width follows the pointer: {r}"
    );
    // bottom panel: 240 high above the 22 px status bar; splitter just above it
    let top = vh - 22.0 - 240.0;
    drag(cx, (500.0, top - 2.0), (500.0, top - 102.0));
    let h = layout(window, cx).panel_height;
    assert!(
        (h - 342.0).abs() < 1.0, // pointer at top-102 => 240 + 102
        "panel height follows the pointer: {h}"
    );
    drag(cx, (500.0, vh - 22.0 - h - 2.0), (500.0, vh - 5.0));
    assert_eq!(
        layout(window, cx).panel_height,
        PANEL_MIN,
        "clamped to the minimum"
    );
}

#[gpui::test]
fn double_clicking_a_splitter_resets_that_size(cx: &mut TestAppContext) {
    let ws = workspace("dbl");
    let (window, cx) = open(cx, &ws.0, None);
    let (_vw, vh) = viewport(window, cx);
    drag(cx, (310.0, vh / 2.0), (450.0, vh / 2.0));
    let w = layout(window, cx).sidebar_width;
    assert!(w > 300.0);
    let x = 48.0 + w + 2.0;
    cx.simulate_event(MouseDownEvent {
        position: point(px(x), px(vh / 2.0)),
        modifiers: Modifiers::default(),
        button: MouseButton::Left,
        click_count: 2,
        first_mouse: false,
    });
    cx.simulate_event(MouseUpEvent {
        position: point(px(x), px(vh / 2.0)),
        modifiers: Modifiers::default(),
        button: MouseButton::Left,
        click_count: 2,
    });
    assert_eq!(
        layout(window, cx).sidebar_width,
        Layout::default().sidebar_width
    );
}

#[gpui::test]
fn maximize_and_hide_the_panel_with_the_keyboard(cx: &mut TestAppContext) {
    let ws = workspace("max");
    let (window, cx) = open(cx, &ws.0, None);
    cx.simulate_keystrokes("ctrl-shift-m");
    let l = layout(window, cx);
    assert!(l.panel && l.panel_maximized);
    cx.simulate_keystrokes("ctrl-shift-m");
    assert!(!layout(window, cx).panel_maximized, "second press restores");
    cx.simulate_keystrokes("ctrl-shift-m ctrl-j");
    let l = layout(window, cx);
    assert!(!l.panel && !l.panel_maximized, "hiding also drops maximize");
    // the editor still works after all of that
    cx.simulate_input("Q");
    let t = window
        .read_with(cx, |wb, _| match &wb.model().active().unwrap().content {
            ui::model::TabContent::Text(e) => e.text(),
            ui::model::TabContent::Unavailable(r) => panic!("{r}"),
            ui::model::TabContent::Diff(_) => panic!("unexpected diff tab"),
        })
        .unwrap();
    assert_eq!(t, "Qx");
}

#[gpui::test]
fn layout_is_saved_and_restored_across_instances(cx: &mut TestAppContext) {
    let ws = workspace("persist");
    let file = ws.0.join("cfg").join("layout.json");
    {
        let (window, cx) = open(cx, &ws.0, Some(file.clone()));
        assert!(!file.exists(), "nothing is written until something changes");
        // drag first: a hidden sidebar has no splitter to grab
        let (_vw, vh) = viewport(window, cx);
        drag(cx, (48.0 + 260.0 + 2.0, vh / 2.0), (48.0 + 330.0, vh / 2.0));
        cx.simulate_keystrokes("ctrl-b ctrl-alt-b");
        window.update(cx, |wb, _, _| wb.save_layout_now()).unwrap();
        assert!(file.exists());
    }
    let saved = Layout::load(&file);
    assert!(!saved.right && !saved.sidebar, "{saved:?}");
    assert!((saved.sidebar_width - 330.0).abs() < 2.0);
    let (window, cx) = open(cx, &ws.0, Some(file.clone()));
    assert_eq!(
        layout(window, cx),
        saved,
        "a new window starts with the saved layout"
    );
}

#[gpui::test]
fn pump_saves_after_a_drag_ends_but_not_in_the_middle_of_one(cx: &mut TestAppContext) {
    let ws = workspace("pump");
    let file = ws.0.join("layout.json");
    let (window, cx) = open(cx, &ws.0, Some(file.clone()));
    let (_vw, vh) = viewport(window, cx);
    cx.simulate_mouse_down(
        point(px(310.0), px(vh / 2.0)),
        MouseButton::Left,
        Modifiers::default(),
    );
    cx.simulate_mouse_move(
        point(px(400.0), px(vh / 2.0)),
        MouseButton::Left,
        Modifiers::default(),
    );
    window.update(cx, |wb, _, _| wb.pump()).unwrap();
    assert!(
        !file.exists(),
        "no writes while the splitter is being dragged"
    );
    cx.simulate_mouse_up(
        point(px(400.0), px(vh / 2.0)),
        MouseButton::Left,
        Modifiers::default(),
    );
    window.update(cx, |wb, _, _| wb.pump()).unwrap();
    assert!(file.exists(), "saved once the drag ended");
    assert!((Layout::load(&file).sidebar_width - 352.0).abs() < 2.0);
}

#[gpui::test]
fn a_corrupt_layout_file_falls_back_to_defaults(cx: &mut TestAppContext) {
    let ws = workspace("corrupt");
    let file = ws.0.join("layout.json");
    std::fs::write(&file, "{ definitely not json").unwrap();
    let (window, cx) = open(cx, &ws.0, Some(file));
    assert_eq!(layout(window, cx), Layout::default());
}

#[gpui::test]
fn shortcuts_work_before_anything_was_focused_and_after_the_editor_is_hidden(
    cx: &mut TestAppContext,
) {
    let ws = workspace("nofocus");
    let root = ws.0.clone();
    // never focused by the test: this is the state right after the app starts
    let window = cx.add_window(move |_w, cx| Workbench::new(root, cx));
    let cx = VisualTestContext::from_window(window.into(), cx).into_mut();
    cx.run_until_parked();
    cx.simulate_keystrokes("ctrl-j");
    assert!(
        !layout(window, cx).panel,
        "Ctrl+J works with nothing focused"
    );
    cx.simulate_keystrokes("ctrl-shift-m");
    assert!(layout(window, cx).panel_maximized);
    cx.simulate_keystrokes("ctrl-shift-m");
    assert!(
        !layout(window, cx).panel_maximized,
        "still works while the editor is not drawn"
    );
    cx.simulate_keystrokes("ctrl-b");
    assert!(!layout(window, cx).sidebar);
}
