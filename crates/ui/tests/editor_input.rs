use std::path::{Path, PathBuf};

use gpui::{TestAppContext, VisualTestContext};
use ui::Workbench;
use ui::model::{Prompt, TabContent};

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn workspace(tag: &str, files: &[(&str, &[u8])]) -> Tmp {
    let root = std::env::temp_dir().join(format!("nanti-ui-it-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    for (name, bytes) in files {
        std::fs::write(root.join(name), bytes).unwrap();
    }
    Tmp(root)
}

fn open<'a>(
    cx: &'a mut TestAppContext,
    root: &Path,
    file: &str,
) -> (gpui::WindowHandle<Workbench>, &'a mut VisualTestContext) {
    let root = root.to_path_buf();
    let path = root.join(file);
    let window = cx.add_window(move |_window, cx| {
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

fn text_of(window: gpui::WindowHandle<Workbench>, cx: &mut VisualTestContext) -> String {
    window
        .read_with(cx, |wb, _| match &wb.model().active().unwrap().content {
            TabContent::Text(e) => e.text(),
            TabContent::Unavailable(r) => panic!("{r}"),
            TabContent::Diff(_) => panic!("unexpected diff tab"),
        })
        .unwrap()
}

fn prompt_of(window: gpui::WindowHandle<Workbench>, cx: &mut VisualTestContext) -> Option<Prompt> {
    window
        .read_with(cx, |wb, _| wb.model().prompt.clone())
        .unwrap()
}

#[gpui::test]
fn typing_enter_and_unicode_then_ctrl_s_writes_the_file(cx: &mut TestAppContext) {
    let ws = workspace("type", &[("a.txt", b"hi\n")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-end");
    cx.simulate_input("héllo 日本");
    cx.simulate_keystrokes("enter");
    cx.simulate_input("🌍");
    assert_eq!(text_of(window, cx), "hi\nhéllo 日本\n🌍");
    cx.simulate_keystrokes("ctrl-s");
    assert_eq!(
        std::fs::read_to_string(ws.0.join("a.txt")).unwrap(),
        "hi\nhéllo 日本\n🌍"
    );
    assert!(
        !window
            .read_with(cx, |wb, _| wb.model().tabs[0].is_dirty())
            .unwrap()
    );
}

#[gpui::test]
fn backspace_undo_redo(cx: &mut TestAppContext) {
    let ws = workspace("undo", &[("a.txt", b"")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_input("abc");
    cx.simulate_keystrokes("backspace");
    assert_eq!(text_of(window, cx), "ab");
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(text_of(window, cx), "abc");
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(text_of(window, cx), "");
    cx.simulate_keystrokes("ctrl-shift-z");
    assert_eq!(text_of(window, cx), "abc");
    cx.simulate_keystrokes("ctrl-y");
    assert_eq!(text_of(window, cx), "ab");
}

#[gpui::test]
fn selection_typing_replace_and_clipboard(cx: &mut TestAppContext) {
    let ws = workspace("sel", &[("a.txt", b"hello world")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    cx.simulate_keystrokes("ctrl-c");
    assert_eq!(
        cx.read_from_clipboard().and_then(|c| c.text()).as_deref(),
        Some("hello")
    );
    cx.simulate_input("bye");
    assert_eq!(text_of(window, cx), "bye world");
    cx.simulate_keystrokes("ctrl-end ctrl-v");
    assert_eq!(text_of(window, cx), "bye worldhello");
    cx.simulate_keystrokes("ctrl-a ctrl-x");
    assert_eq!(text_of(window, cx), "");
    assert_eq!(
        cx.read_from_clipboard().and_then(|c| c.text()).as_deref(),
        Some("bye worldhello")
    );
}

#[gpui::test]
fn crlf_file_keeps_crlf_when_typing_enter_and_saving(cx: &mut TestAppContext) {
    let ws = workspace("crlf", &[("w.txt", b"a\r\nb")]);
    let (_window, cx) = open(cx, &ws.0, "w.txt");
    cx.simulate_keystrokes("ctrl-end enter");
    cx.simulate_input("c");
    cx.simulate_keystrokes("ctrl-s");
    assert_eq!(std::fs::read(ws.0.join("w.txt")).unwrap(), b"a\r\nb\r\nc");
}

#[gpui::test]
fn arrow_home_end_and_tab_keys(cx: &mut TestAppContext) {
    let ws = workspace("nav", &[("a.txt", b"one\ntwo")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("down end");
    cx.simulate_input("!");
    cx.simulate_keystrokes("home tab");
    assert_eq!(text_of(window, cx), "one\n    two!");
    cx.simulate_keystrokes("up home delete");
    assert_eq!(text_of(window, cx), "ne\n    two!");
}

#[gpui::test]
fn dirty_tab_close_asks_then_save_closes(cx: &mut TestAppContext) {
    let ws = workspace("close", &[("a.txt", b"x")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_input("y");
    cx.simulate_keystrokes("ctrl-w");
    assert_eq!(prompt_of(window, cx), Some(Prompt::CloseDirty(0)));
    assert_eq!(
        window.read_with(cx, |wb, _| wb.model().tabs.len()).unwrap(),
        1
    );
    cx.simulate_keystrokes("escape");
    assert_eq!(prompt_of(window, cx), None);
    cx.simulate_keystrokes("ctrl-w enter");
    assert_eq!(
        window.read_with(cx, |wb, _| wb.model().tabs.len()).unwrap(),
        0
    );
    assert_eq!(std::fs::read_to_string(ws.0.join("a.txt")).unwrap(), "yx");
}

#[gpui::test]
fn external_change_raises_conflict_and_does_not_overwrite(cx: &mut TestAppContext) {
    let ws = workspace("conf", &[("a.txt", b"mine")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_input("!");
    let path = ws.0.join("a.txt");
    std::fs::write(&path, "theirs").unwrap();
    let f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    f.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(120))
        .unwrap();
    cx.simulate_keystrokes("ctrl-s");
    assert!(matches!(
        prompt_of(window, cx),
        Some(Prompt::SaveConflict { .. })
    ));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs");
    cx.simulate_input("zzz");
    assert_eq!(text_of(window, cx), "!mine");
    cx.simulate_keystrokes("escape");
    assert_eq!(prompt_of(window, cx), None);
}

#[gpui::test]
fn window_close_with_unsaved_changes_is_intercepted(cx: &mut TestAppContext) {
    let ws = workspace("quit", &[("a.txt", b"x")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    assert!(
        window
            .update(cx, |wb, _, _| wb.allow_window_close())
            .unwrap(),
        "clean: may close"
    );
    cx.simulate_input("y");
    assert!(
        !window
            .update(cx, |wb, _, _| wb.allow_window_close())
            .unwrap(),
        "dirty: must ask"
    );
    assert_eq!(prompt_of(window, cx), Some(Prompt::QuitDirty));
    assert_eq!(
        std::fs::read_to_string(ws.0.join("a.txt")).unwrap(),
        "x",
        "nothing written"
    );
}

#[gpui::test]
fn binary_file_is_not_editable(cx: &mut TestAppContext) {
    let ws = workspace("bin", &[("b.bin", &[0xff, 0xfe, 0x00])]);
    let root = ws.0.clone();
    let window = cx.add_window(move |_w, cx| {
        let mut wb = Workbench::new(root.clone(), cx);
        wb.open_path(&root.join("b.bin"));
        wb
    });
    let cx = VisualTestContext::from_window(window.into(), cx).into_mut();
    cx.simulate_input("abc");
    cx.simulate_keystrokes("ctrl-s");
    assert_eq!(
        std::fs::read(ws.0.join("b.bin")).unwrap(),
        vec![0xff, 0xfe, 0x00]
    );
}
