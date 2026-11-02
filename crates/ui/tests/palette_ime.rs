use std::path::{Path, PathBuf};

use gpui::{EntityInputHandler, TestAppContext, VisualTestContext, WindowHandle};
use ui::Workbench;
use ui::model::TabContent;

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn workspace(tag: &str, files: &[(&str, &str)]) -> Tmp {
    let root = std::env::temp_dir().join(format!("nanti-ui-pi-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    for (name, text) in files {
        std::fs::write(root.join(name), text).unwrap();
    }
    Tmp(root)
}

fn open<'a>(
    cx: &'a mut TestAppContext,
    root: &Path,
    file: &str,
) -> (WindowHandle<Workbench>, &'a mut VisualTestContext) {
    let root = root.to_path_buf();
    let path = root.join(file);
    let window = cx.add_window(move |_w, cx| {
        let mut wb = Workbench::new(root, cx);
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

fn text(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) -> String {
    window
        .read_with(cx, |wb, _| match &wb.model().active().unwrap().content {
            TabContent::Text(e) => e.text(),
            TabContent::Unavailable(r) => panic!("{r}"),
            TabContent::Diff(_) | TabContent::Webview => panic!("unexpected tab"),
        })
        .unwrap()
}

fn query(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) -> Option<String> {
    window
        .read_with(cx, |wb, _| {
            wb.model()
                .palette
                .as_ref()
                .map(|p| p.field.text().to_owned())
        })
        .unwrap()
}

fn ime<R>(
    window: WindowHandle<Workbench>,
    cx: &mut VisualTestContext,
    f: impl FnOnce(&mut Workbench, &mut gpui::Window, &mut gpui::Context<Workbench>) -> R,
) -> R {
    let out = window.update(cx, f).unwrap();
    cx.run_until_parked();
    out
}

#[gpui::test]
fn ime_composition_updates_then_commits_as_one_undo_step(cx: &mut TestAppContext) {
    let ws = workspace("ime1", &[("a.txt", "ab")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("right");
    ime(window, cx, |wb, w, cx| {
        wb.replace_and_mark_text_in_range(None, "n", Some(1..1), w, cx)
    });
    assert_eq!(text(window, cx), "anb");
    let marked = ime(window, cx, |wb, w, cx| wb.marked_text_range(w, cx));
    assert_eq!(marked, Some(1..2));
    ime(window, cx, |wb, w, cx| {
        wb.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx)
    });
    assert_eq!(text(window, cx), "anib");
    ime(window, cx, |wb, w, cx| {
        wb.replace_and_mark_text_in_range(None, "你", Some(1..1), w, cx)
    });
    assert_eq!(text(window, cx), "a你b");
    ime(window, cx, |wb, w, cx| {
        wb.replace_text_in_range(None, "你", w, cx)
    });
    assert_eq!(text(window, cx), "a你b");
    assert_eq!(
        ime(window, cx, |wb, w, cx| wb.marked_text_range(w, cx)),
        None
    );
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(
        text(window, cx),
        "ab",
        "the whole composition is one undo step"
    );
}

#[gpui::test]
fn ime_commit_with_astral_chars_reports_utf16_selection(cx: &mut TestAppContext) {
    let ws = workspace("ime2", &[("a.txt", "")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    ime(window, cx, |wb, w, cx| {
        wb.replace_text_in_range(None, "a🌍", w, cx)
    });
    assert_eq!(text(window, cx), "a🌍");
    let sel = ime(window, cx, |wb, w, cx| wb.selected_text_range(false, w, cx)).unwrap();
    assert_eq!((sel.range, sel.reversed), (3..3, false));
    let mut adjusted = None;
    let got = ime(window, cx, |wb, w, cx| {
        wb.text_for_range(1..3, &mut adjusted, w, cx)
    });
    assert_eq!((got.as_deref(), adjusted), (Some("🌍"), Some(1..3)));
}

#[gpui::test]
fn ime_composition_over_selection_replaces_it(cx: &mut TestAppContext) {
    let ws = workspace("ime3", &[("a.txt", "hello world")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    ime(window, cx, |wb, w, cx| {
        wb.replace_and_mark_text_in_range(None, "こ", Some(1..1), w, cx)
    });
    assert_eq!(text(window, cx), "こ world");
    ime(window, cx, |wb, w, cx| {
        wb.replace_text_in_range(None, "今", w, cx)
    });
    assert_eq!(text(window, cx), "今 world");
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(text(window, cx), "hello world");
}

#[gpui::test]
fn ime_candidate_window_bounds_are_available(cx: &mut TestAppContext) {
    let ws = workspace("ime4", &[("a.txt", "one\ntwo")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    let b = ime(window, cx, |wb, w, cx| {
        let area = gpui::Bounds::default();
        wb.bounds_for_range(5..6, area, w, cx)
    })
    .expect("bounds for a range on line 2");
    let first = ime(window, cx, |wb, w, cx| {
        wb.bounds_for_range(0..1, gpui::Bounds::default(), w, cx)
    })
    .unwrap();
    assert!(b.origin.y > first.origin.y, "line 2 is below line 1");
    assert_eq!(f32::from(b.size.height), 20.0);
}

#[gpui::test]
fn typing_is_not_applied_twice_with_the_input_handler_active(cx: &mut TestAppContext) {
    let ws = workspace("once", &[("a.txt", "")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_input("héllo");
    assert_eq!(text(window, cx), "héllo");
}

#[gpui::test]
fn command_palette_filters_runs_a_command_and_returns_focus_to_the_editor(cx: &mut TestAppContext) {
    let ws = workspace("pal1", &[("a.txt", "x")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-shift-p");
    assert_eq!(query(window, cx), Some(String::new()));
    cx.simulate_input("toggle side");
    assert_eq!(query(window, cx).as_deref(), Some("toggle side"));
    assert_eq!(
        text(window, cx),
        "x",
        "typing went to the palette, not the editor"
    );
    assert!(
        window
            .read_with(cx, |wb, _| wb.model().layout.sidebar)
            .unwrap()
    );
    cx.simulate_keystrokes("enter");
    assert_eq!(query(window, cx), None, "palette closed");
    assert!(
        !window
            .read_with(cx, |wb, _| wb.model().layout.sidebar)
            .unwrap()
    );
    cx.simulate_input("y");
    assert_eq!(text(window, cx), "yx");
}

#[gpui::test]
fn palette_navigation_backspace_and_escape(cx: &mut TestAppContext) {
    let ws = workspace("pal2", &[("a.txt", "x")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-shift-p");
    cx.simulate_input("save");
    cx.simulate_keystrokes("backspace backspace");
    assert_eq!(query(window, cx).as_deref(), Some("sa"));
    cx.simulate_keystrokes("down down up");
    let selected = window
        .read_with(cx, |wb, _| wb.model().palette.as_ref().unwrap().selected)
        .unwrap();
    assert_eq!(selected, 1);
    cx.simulate_keystrokes("escape");
    assert_eq!(query(window, cx), None);
    cx.simulate_input("z");
    assert_eq!(
        text(window, cx),
        "zx",
        "after Escape the editor owns the keyboard again"
    );
}

fn wait_index(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) {
    let start = std::time::Instant::now();
    while window
        .update(cx, |wb, _, _| {
            wb.pump();
            wb.model().index_rx.is_some()
        })
        .unwrap()
    {
        assert!(
            start.elapsed() < std::time::Duration::from_secs(30),
            "the file index never arrived"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

#[gpui::test]
fn quick_open_opens_a_file_chosen_by_fuzzy_query(cx: &mut TestAppContext) {
    let ws = workspace("pal3", &[("a.txt", "A"), ("notes.md", "NOTES")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-p");
    wait_index(window, cx);
    cx.simulate_input("nts");
    cx.simulate_keystrokes("enter");
    let title = window
        .read_with(cx, |wb, _| wb.model().active().unwrap().title.clone())
        .unwrap();
    assert_eq!(title, "notes.md");
    assert_eq!(text(window, cx), "NOTES");
    assert_eq!(
        window.read_with(cx, |wb, _| wb.model().tabs.len()).unwrap(),
        2
    );
}

#[gpui::test]
fn palette_runs_save_and_tab_switch_commands(cx: &mut TestAppContext) {
    let ws = workspace("pal4", &[("a.txt", "A"), ("b.txt", "B")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_input("1");
    cx.simulate_keystrokes("ctrl-shift-p");
    cx.simulate_input("file: save");
    cx.simulate_keystrokes("enter");
    assert_eq!(std::fs::read_to_string(ws.0.join("a.txt")).unwrap(), "1A");
    cx.simulate_keystrokes("ctrl-p");
    wait_index(window, cx);
    cx.simulate_input("b.txt");
    cx.simulate_keystrokes("enter");
    assert_eq!(text(window, cx), "B");
    cx.simulate_keystrokes("ctrl-shift-tab");
    assert_eq!(text(window, cx), "1A");
    cx.simulate_keystrokes("ctrl-tab");
    assert_eq!(text(window, cx), "B");
}

#[gpui::test]
fn palette_accepts_ime_composition_in_its_query(cx: &mut TestAppContext) {
    let ws = workspace("pal5", &[("a.txt", "x")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-shift-p");
    ime(window, cx, |wb, w, cx| {
        wb.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx)
    });
    assert_eq!(query(window, cx).as_deref(), Some("ni"));
    ime(window, cx, |wb, w, cx| {
        wb.replace_text_in_range(None, "你", w, cx)
    });
    assert_eq!(query(window, cx).as_deref(), Some("你"));
    assert_eq!(text(window, cx), "x", "the editor was not touched");
    let count = window
        .read_with(cx, |wb, _| {
            wb.model().palette.as_ref().unwrap().result_count()
        })
        .unwrap();
    assert_eq!(count, 0, "no command matches 你");
}

#[gpui::test]
fn ctrl_p_switches_between_the_two_palettes_and_ignores_other_shortcuts(cx: &mut TestAppContext) {
    let ws = workspace("pal6", &[("a.txt", "x")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-shift-p");
    cx.simulate_keystrokes("ctrl-s ctrl-a"); // must not leak to the editor / save
    assert_eq!(text(window, cx), "x");
    cx.simulate_keystrokes("ctrl-p");
    let mode = window
        .read_with(cx, |wb, _| {
            format!("{:?}", wb.model().palette.as_ref().unwrap().mode)
        })
        .unwrap();
    assert_eq!(mode, "Files");
}
