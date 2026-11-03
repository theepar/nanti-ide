use std::path::PathBuf;

use gpui::{TestAppContext, VisualTestContext, WindowHandle};
use ui::Workbench;

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn open<'a>(
    cx: &'a mut TestAppContext,
    root: PathBuf,
    file: &str,
) -> (WindowHandle<Workbench>, &'a mut VisualTestContext) {
    let path = root.join(file);
    let window = cx.add_window(move |_w, cx| {
        let mut wb = Workbench::new(root, cx);
        wb.model_mut().open_file(&path);
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
        .update(cx, |wb, _, _| {
            wb.model_mut().active_editor_mut().unwrap().text()
        })
        .unwrap()
}

#[gpui::test]
fn bang_then_tab_writes_the_html_boilerplate(cx: &mut TestAppContext) {
    let dir = std::env::temp_dir().join(format!("nanti-emmet-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let t = Tmp(dir);
    std::fs::write(t.0.join("index.html"), "").unwrap();
    std::fs::write(t.0.join("notes.txt"), "").unwrap();

    let (window, cx) = open(cx, t.0.clone(), "index.html");
    cx.simulate_input("!");
    cx.simulate_keystrokes("tab");
    let out = text(window, cx);
    assert!(
        out.starts_with("<!DOCTYPE html>\n<html lang=\"en\">"),
        "{out}"
    );
    assert!(out.contains("<title>Document</title>"), "{out}");
    assert!(out.ends_with("</body>\n</html>"), "{out}");
    let (line, col) = window
        .update(cx, |wb, _, _| {
            wb.model_mut().active_editor_mut().unwrap().line_col()
        })
        .unwrap();
    assert_eq!((line, col), (8, 4), "the cursor waits inside the body");

    cx.simulate_keystrokes("ctrl-a backspace");
    cx.simulate_input("ul>li*2");
    cx.simulate_keystrokes("tab");
    assert_eq!(
        text(window, cx),
        "<ul>\n    <li></li>\n    <li></li>\n</ul>"
    );
}

#[gpui::test]
fn tab_stays_a_tab_outside_html_and_after_plain_words(cx: &mut TestAppContext) {
    let dir = std::env::temp_dir().join(format!("nanti-emmet2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let t = Tmp(dir);
    std::fs::write(t.0.join("index.html"), "").unwrap();
    std::fs::write(t.0.join("notes.txt"), "").unwrap();

    let (window, cx) = open(cx, t.0.clone(), "notes.txt");
    cx.simulate_input("!");
    cx.simulate_keystrokes("tab");
    assert_eq!(text(window, cx), "!    ");

    let (window, cx) = open(cx, t.0.clone(), "index.html");
    cx.simulate_input("hello");
    cx.simulate_keystrokes("tab");
    assert_eq!(text(window, cx), "hello    ");
}
