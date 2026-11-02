use std::path::PathBuf;
use std::time::{Duration, Instant};

use ui::model::Model;
use ui::palette::{Command, PaletteMode};

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tree(tag: &str, files: usize) -> Tmp {
    let root = std::env::temp_dir().join(format!("nanti-perf-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for i in 0..files {
        let dir = root
            .join(format!("pkg{:03}", i % 200))
            .join(format!("sub{}", i % 7));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("file{i}.rs")), "x").unwrap();
    }
    Tmp(root)
}

fn settle(m: &mut Model) {
    let start = Instant::now();
    while m.index_rx.is_some() {
        m.pump_index();
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "the index never finished"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn quick_open_does_not_block_on_a_big_workspace_and_is_instant_the_second_time() {
    let t = tree("quickopen", 20_000);
    let mut m = Model::new(t.0.clone());

    let start = Instant::now();
    m.run_command(Command::QuickOpen);
    let opened = start.elapsed();
    assert!(m.palette.as_ref().unwrap().loading);
    assert!(
        opened < Duration::from_millis(50),
        "the palette opened in {opened:?}"
    );

    settle(&mut m);
    let indexed = start.elapsed();
    let p = m.palette.as_mut().unwrap();
    assert!(!p.loading);
    p.field.insert("file19999");
    p.refilter();
    assert_eq!(p.result_count(), 1);
    println!(
        "quick open, 20000 files: palette on screen in {opened:?}, searchable after {indexed:?}"
    );

    m.close_palette();
    let again = Instant::now();
    m.open_palette(PaletteMode::Files);
    let reopened = again.elapsed();
    assert!(
        !m.palette.as_ref().unwrap().loading,
        "served from the cache"
    );
    assert!(m.index_rx.is_none(), "no new scan was started");
    println!("quick open again (cached index): usable in {reopened:?}");
    assert!(reopened < indexed.max(Duration::from_millis(1)));
}

#[test]
fn the_file_index_is_rebuilt_after_files_change() {
    let t = tree("reindex", 50);
    let mut m = Model::new(t.0.clone());
    m.open_palette(PaletteMode::Files);
    settle(&mut m);
    m.close_palette();
    std::fs::write(t.0.join("brand_new_file.txt"), "x").unwrap();
    m.invalidate_explorer();
    m.open_palette(PaletteMode::Files);
    assert!(
        m.palette.as_ref().unwrap().loading,
        "a changed workspace is scanned again"
    );
    settle(&mut m);
    let p = m.palette.as_mut().unwrap();
    p.field.insert("brand_new");
    p.refilter();
    assert_eq!(p.result_count(), 1);
}

#[test]
fn typing_before_the_index_arrives_keeps_the_query_and_fills_in_later() {
    let t = tree("typing", 300);
    let mut m = Model::new(t.0.clone());
    m.open_palette(PaletteMode::Files);
    m.palette.as_mut().unwrap().field.insert("file299");
    m.palette.as_mut().unwrap().refilter();
    assert_eq!(m.palette.as_ref().unwrap().result_count(), 0);
    settle(&mut m);
    assert_eq!(m.palette.as_ref().unwrap().field.text(), "file299");
    assert_eq!(m.palette.as_ref().unwrap().result_count(), 1);
}

#[test]
fn closing_the_palette_before_the_index_arrives_is_harmless() {
    let t = tree("closed", 300);
    let mut m = Model::new(t.0.clone());
    m.open_palette(PaletteMode::Files);
    m.close_palette();
    settle(&mut m);
    assert!(m.palette.is_none());
    assert!(m.file_index.is_some());
}

fn rust_file(lines: usize) -> String {
    let unit = "pub fn compute(values: &[u32], factor: u32) -> Vec<u32> { values.iter().map(|v| v * factor).collect() } // note\n";
    unit.repeat(lines)
}

fn open<'a>(
    cx: &'a mut gpui::TestAppContext,
    root: &std::path::Path,
    file: &std::path::Path,
) -> (
    gpui::WindowHandle<ui::Workbench>,
    &'a mut gpui::VisualTestContext,
) {
    let (root, file) = (root.to_path_buf(), file.to_path_buf());
    let window = cx.add_window(move |_w, cx| {
        let mut wb = ui::Workbench::new(root, cx);
        wb.open_path(&file);
        wb
    });
    let vcx = gpui::VisualTestContext::from_window(window.into(), cx).into_mut();
    window
        .update(vcx, |wb, window, _| window.focus(wb.editor_focus()))
        .unwrap();
    vcx.run_until_parked();
    (window, vcx)
}

#[gpui::test]
fn typing_in_a_big_file_never_waits_for_syntax_highlighting(cx: &mut gpui::TestAppContext) {
    let t = tree("highlight", 1);
    let big = t.0.join("big.rs");
    let text = rust_file(3000);
    std::fs::write(&big, &text).unwrap();
    let (window, cx) = open(cx, &t.0, &big);

    let first = window
        .read_with(cx, |wb, _| {
            let start = Instant::now();
            let h = wb.active_highlights();
            (h.is_some(), start.elapsed())
        })
        .unwrap();
    assert!(
        first.1 < Duration::from_millis(30),
        "first frame took {:?}",
        first.1
    );

    let begun = Instant::now();
    let fresh = loop {
        let got = window
            .update(cx, |wb, _, _| {
                wb.pump();
                wb.active_highlights()
            })
            .unwrap();
        if let Some(spans) = got {
            break spans;
        }
        assert!(
            begun.elapsed() < Duration::from_secs(60),
            "highlights never arrived"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    let first_colors = begun.elapsed();
    let mut reference = syntax::SyntaxHighlighter::new();
    let sync_start = Instant::now();
    let expected = reference.highlight(syntax::Language::Rust, &text).unwrap();
    let sync_cost = sync_start.elapsed();
    assert_eq!(*fresh, expected);

    cx.simulate_input("x");
    let typed = window
        .read_with(cx, |wb, _| {
            let start = Instant::now();
            let h = wb.active_highlights();
            (h.map(|s| s.len()), start.elapsed())
        })
        .unwrap();
    assert_eq!(
        typed.0,
        Some(3001),
        "the previous colors stay on screen while typing"
    );
    assert!(
        typed.1 < Duration::from_millis(30),
        "a keystroke waited {:?}",
        typed.1
    );
    println!(
        "3000-line file: colors after open {first_colors:?}; a synchronous re-highlight costs {sync_cost:?} per keystroke, \
         the frame after typing took {:?}",
        typed.1
    );

    cx.simulate_keystrokes("enter");
    let after_enter = window
        .read_with(cx, |wb, _| wb.active_highlights().map(|s| s.len()))
        .unwrap();
    assert_eq!(
        after_enter,
        Some(3002),
        "a new line keeps the stale colors aligned"
    );

    let begun = Instant::now();
    loop {
        let (version_text, spans) = window
            .update(cx, |wb, _, _| {
                wb.pump();
                let text = wb.model().active().and_then(|t| match &t.content {
                    ui::model::TabContent::Text(e) => Some(e.text()),
                    _ => None,
                });
                (text, wb.active_highlights())
            })
            .unwrap();
        let expected = reference
            .highlight(syntax::Language::Rust, version_text.as_deref().unwrap())
            .unwrap();
        if spans.is_some_and(|s| *s == expected) {
            break;
        }
        assert!(
            begun.elapsed() < Duration::from_secs(60),
            "the colors never caught up"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    println!("colors caught up with the edit {:?} later", begun.elapsed());
}

#[gpui::test]
fn small_files_are_still_highlighted_in_the_same_frame(cx: &mut gpui::TestAppContext) {
    let t = tree("smallhl", 1);
    let small = t.0.join("small.rs");
    std::fs::write(&small, "fn main() {}\n").unwrap();
    let (window, cx) = open(cx, &t.0, &small);
    let got = window
        .read_with(cx, |wb, _| wb.active_highlights().is_some())
        .unwrap();
    assert!(got);
    cx.simulate_input("//");
    let got = window
        .read_with(cx, |wb, _| wb.active_highlights().is_some())
        .unwrap();
    assert!(got);
}
