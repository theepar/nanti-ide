use std::path::{Path, PathBuf};

use gpui::{EntityInputHandler, TestAppContext, VisualTestContext, WindowHandle};
use syntax::Token;
use ui::Workbench;
use ui::model::TabContent;

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn workspace(tag: &str, files: &[(&str, &str)]) -> Tmp {
    let root = std::env::temp_dir().join(format!("nanti-ui-fs-{tag}-{}", std::process::id()));
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
            TabContent::Diff(_) => panic!("unexpected diff tab"),
        })
        .unwrap()
}

fn selection(
    window: WindowHandle<Workbench>,
    cx: &mut VisualTestContext,
) -> std::ops::Range<usize> {
    window
        .read_with(cx, |wb, _| match &wb.model().active().unwrap().content {
            TabContent::Text(e) => e.selection(),
            TabContent::Unavailable(r) => panic!("{r}"),
            TabContent::Diff(_) => panic!("unexpected diff tab"),
        })
        .unwrap()
}

fn status(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) -> Option<String> {
    window
        .read_with(cx, |wb, _| wb.model().find.as_ref().map(|f| f.status()))
        .unwrap()
}

fn match_count(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) -> usize {
    window
        .read_with(cx, |wb, _| {
            wb.model().find.as_ref().map_or(0, |f| f.matches().len())
        })
        .unwrap()
}

#[gpui::test]
fn ctrl_f_opens_find_and_typing_goes_to_the_query_not_the_editor(cx: &mut TestAppContext) {
    let ws = workspace("f1", &[("a.txt", "foo bar foo\nfoo")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-f");
    cx.simulate_input("foo");
    assert_eq!(text(window, cx), "foo bar foo\nfoo", "editor untouched");
    cx.run_until_parked();
    assert_eq!(match_count(window, cx), 3);
    assert_eq!(selection(window, cx), 0..3);
    assert_eq!(status(window, cx).as_deref(), Some("1 of 3"));
}

#[gpui::test]
fn enter_and_shift_enter_walk_the_matches_and_wrap(cx: &mut TestAppContext) {
    let ws = workspace("f2", &[("a.txt", "x a x b x")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-f");
    cx.simulate_input("x");
    assert_eq!(selection(window, cx), 0..1);
    cx.simulate_keystrokes("enter");
    assert_eq!(selection(window, cx), 4..5);
    cx.simulate_keystrokes("enter enter");
    assert_eq!(selection(window, cx), 0..1, "wrapped");
    cx.simulate_keystrokes("shift-enter");
    assert_eq!(selection(window, cx), 8..9, "previous wraps to the last");
    cx.simulate_keystrokes("down");
    assert_eq!(selection(window, cx), 0..1);
    cx.simulate_keystrokes("up");
    assert_eq!(selection(window, cx), 8..9);
}

#[gpui::test]
fn escape_closes_find_and_typing_returns_to_the_editor_and_f3_navigates(cx: &mut TestAppContext) {
    let ws = workspace("f3", &[("a.txt", "ab ab")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-f");
    cx.simulate_input("ab");
    cx.simulate_keystrokes("escape");
    assert_eq!(status(window, cx), None, "find closed");
    cx.simulate_input("Z");
    assert!(text(window, cx).contains('Z'), "typing went to the editor");
    cx.simulate_keystrokes("ctrl-z");
    cx.simulate_keystrokes("f3");
    assert!(status(window, cx).is_some());
}

#[gpui::test]
fn alt_toggles_change_the_match_set_and_invalid_regex_reports_an_error(cx: &mut TestAppContext) {
    let ws = workspace("f4", &[("a.txt", "Foo foo FOO")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-f");
    cx.simulate_input("foo");
    cx.run_until_parked();
    assert_eq!(match_count(window, cx), 3);
    cx.simulate_keystrokes("alt-c");
    cx.run_until_parked();
    assert_eq!(match_count(window, cx), 1, "case sensitive");
    cx.simulate_keystrokes("alt-c alt-r");
    cx.simulate_keystrokes("backspace backspace backspace");
    cx.simulate_input("(");
    cx.run_until_parked();
    let st = status(window, cx).unwrap();
    assert!(
        !st.is_empty() && st != "No results",
        "regex error shown: {st}"
    );
    assert_eq!(match_count(window, cx), 0);
    assert_eq!(text(window, cx), "Foo foo FOO");
}

#[gpui::test]
fn replace_current_and_replace_all_with_a_single_undo(cx: &mut TestAppContext) {
    let ws = workspace("f5", &[("a.txt", "a1 a2 a3")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-h");
    cx.simulate_input("a");
    cx.simulate_keystrokes("tab");
    cx.simulate_input("Z");
    assert_eq!(
        text(window, cx),
        "a1 a2 a3",
        "typing in the inputs does not edit the file"
    );
    cx.simulate_keystrokes("enter");
    assert_eq!(text(window, cx), "Z1 a2 a3");
    cx.simulate_keystrokes("shift-enter");
    assert_eq!(text(window, cx), "Z1 Z2 Z3");
    let msg = window
        .read_with(cx, |wb, _| wb.model().message.clone())
        .unwrap();
    assert!(msg.unwrap().starts_with("Replaced 2"));
    cx.simulate_keystrokes("escape");
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(
        text(window, cx),
        "Z1 a2 a3",
        "replace-all undoes as one step"
    );
}

#[gpui::test]
fn matches_follow_edits_made_while_find_is_open(cx: &mut TestAppContext) {
    let ws = workspace("f6", &[("a.txt", "ab")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-f");
    cx.simulate_input("ab");
    cx.run_until_parked();
    assert_eq!(match_count(window, cx), 1);
    window
        .update(cx, |wb, window, _| wb.focus_editor(window))
        .unwrap();
    cx.simulate_keystrokes("ctrl-end");
    cx.simulate_input(" ab ab");
    cx.run_until_parked();
    assert_eq!(match_count(window, cx), 3);
    cx.simulate_keystrokes("ctrl-z");
    cx.run_until_parked();
    assert_eq!(match_count(window, cx), 1);
}

#[gpui::test]
fn find_query_accepts_ime_composition(cx: &mut TestAppContext) {
    let ws = workspace("f7", &[("a.txt", "x 你 y 你")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-f");
    window
        .update(cx, |wb, w, cx| {
            wb.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx)
        })
        .unwrap();
    window
        .update(cx, |wb, w, cx| wb.replace_text_in_range(None, "你", w, cx))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(match_count(window, cx), 2);
    assert_eq!(text(window, cx), "x 你 y 你");
}

#[gpui::test]
fn ctrl_f_prefills_from_a_one_line_selection(cx: &mut TestAppContext) {
    let ws = workspace("f8", &[("a.txt", "hello world hello")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    cx.simulate_keystrokes("ctrl-f");
    cx.run_until_parked();
    let q = window
        .read_with(cx, |wb, _| {
            wb.model().find.as_ref().unwrap().query.text().to_owned()
        })
        .unwrap();
    assert_eq!(q, "hello");
    assert_eq!(match_count(window, cx), 2);
}

#[gpui::test]
fn find_in_files_runs_on_enter_and_lists_hits(cx: &mut TestAppContext) {
    let ws = workspace(
        "s1",
        &[
            ("a.txt", "needle one\nplain\n"),
            ("b.txt", "x needle y\nneedle\n"),
        ],
    );
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("ctrl-shift-f");
    cx.simulate_input("needle");
    assert!(
        window
            .read_with(cx, |wb, _| wb.model().search.results.is_none())
            .unwrap(),
        "search runs on Enter, not on every keystroke"
    );
    assert_eq!(
        text(window, cx),
        "needle one\nplain\n",
        "typing went to the search box"
    );
    cx.simulate_keystrokes("enter");
    window
        .update(cx, |wb, _, _| wb.model_mut().search.wait())
        .unwrap();
    let (hits, files) = window
        .read_with(cx, |wb, _| {
            let r = wb.model().search.results.as_ref().unwrap();
            (r.total_hits, r.files.len())
        })
        .unwrap();
    assert_eq!((hits, files), (3, 2));
    cx.simulate_keystrokes("escape");
    cx.simulate_input("Q");
    assert!(text(window, cx).starts_with('Q'));
}

#[gpui::test]
fn find_in_files_prefills_from_selection_and_alt_toggles_apply(cx: &mut TestAppContext) {
    let ws = workspace("s2", &[("a.txt", "Foo foo\n")]);
    let (window, cx) = open(cx, &ws.0, "a.txt");
    cx.simulate_keystrokes("shift-right shift-right shift-right ctrl-shift-f");
    cx.simulate_keystrokes("enter");
    window
        .update(cx, |wb, _, _| wb.model_mut().search.wait())
        .unwrap();
    let n = window
        .read_with(cx, |wb, _| {
            wb.model().search.results.as_ref().unwrap().total_hits
        })
        .unwrap();
    assert_eq!(n, 2, "case-insensitive by default");
    cx.simulate_keystrokes("alt-c enter");
    window
        .update(cx, |wb, _, _| wb.model_mut().search.wait())
        .unwrap();
    let n = window
        .read_with(cx, |wb, _| {
            wb.model().search.results.as_ref().unwrap().total_hits
        })
        .unwrap();
    assert_eq!(n, 1, "Alt+C = match case");
}

#[gpui::test]
fn rust_file_is_highlighted_and_follows_edits(cx: &mut TestAppContext) {
    let ws = workspace("h1", &[("main.rs", "fn main() {}\n")]);
    let (window, cx) = open(cx, &ws.0, "main.rs");
    cx.run_until_parked();
    let first = window
        .read_with(cx, |wb, _| wb.active_highlights())
        .unwrap()
        .expect("rust is highlighted");
    assert!(
        first[0]
            .iter()
            .any(|s| s.token == Token::Keyword && s.range == (0..2))
    );
    cx.simulate_keystrokes("ctrl-end");
    cx.simulate_input("// hi");
    cx.run_until_parked();
    let second = window
        .read_with(cx, |wb, _| wb.active_highlights())
        .unwrap()
        .unwrap();
    assert!(
        second[1].iter().any(|s| s.token == Token::Comment),
        "new text is highlighted after the edit: {second:?}"
    );
}

#[gpui::test]
fn plain_text_and_unknown_files_have_no_highlighting_and_crlf_rust_works(cx: &mut TestAppContext) {
    let ws = workspace(
        "h2",
        &[
            ("a.txt", "fn main() {}"),
            ("w.rs", "// c\r\nfn x() {}\r\n"),
            ("odd.rs", "a\rb"),
        ],
    );
    let (window, cx) = open(cx, &ws.0, "a.txt");
    assert!(
        window
            .read_with(cx, |wb, _| wb.active_highlights())
            .unwrap()
            .is_none()
    );
    window
        .update(cx, |wb, _, _| wb.open_path(&ws.0.join("w.rs")))
        .unwrap();
    let spans = window
        .read_with(cx, |wb, _| wb.active_highlights())
        .unwrap()
        .expect("crlf rust");
    assert!(spans[0].iter().any(|s| s.token == Token::Comment));
    assert!(spans[1].iter().any(|s| s.token == Token::Keyword));
    window
        .update(cx, |wb, _, _| wb.open_path(&ws.0.join("odd.rs")))
        .unwrap();
    assert!(
        window
            .read_with(cx, |wb, _| wb.active_highlights())
            .unwrap()
            .is_none()
    );
}
