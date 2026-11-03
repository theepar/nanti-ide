use std::path::PathBuf;
use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, WindowHandle};
use ui::Workbench;
use ui::model::{Activity, PanelTab};

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn open<'a>(
    cx: &'a mut TestAppContext,
    root: PathBuf,
) -> (WindowHandle<Workbench>, &'a mut VisualTestContext) {
    let window = cx.add_window(move |_w, cx| Workbench::new(root, cx));
    let vcx = VisualTestContext::from_window(window.into(), cx).into_mut();
    vcx.run_until_parked();
    (window, vcx)
}

fn per_frame(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) -> Duration {
    let start = Instant::now();
    for _ in 0..10 {
        window.update(cx, |_, _, cx| cx.notify()).unwrap();
        cx.run_until_parked();
    }
    start.elapsed() / 10
}

#[gpui::test]
fn two_thousand_search_hits_and_output_lines_keep_frames_cheap(cx: &mut TestAppContext) {
    let dir = std::env::temp_dir().join(format!("nanti-lists-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let t = Tmp(dir);
    for i in 0..40 {
        std::fs::write(t.0.join(format!("f{i}.txt")), "needle here\n".repeat(60)).unwrap();
    }
    let (window, cx) = open(cx, t.0.clone());

    let baseline = per_frame(window, cx);

    window
        .update(cx, |wb, _, _| {
            let m = wb.model_mut();
            m.activity = Activity::Search;
            m.search.query.insert("needle");
            m.run_search();
        })
        .unwrap();
    let start = Instant::now();
    loop {
        let done = window
            .update(cx, |wb, _, _| {
                wb.model_mut().pump_search();
                !wb.model().search.running && wb.model().search.results.is_some()
            })
            .unwrap();
        if done {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "search never finished"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let hits = window
        .read_with(cx, |wb, _| {
            wb.model()
                .search
                .results
                .as_ref()
                .map_or(0, |r| r.total_hits)
        })
        .unwrap();
    assert!(hits >= 2000, "{hits} hits");
    let search = per_frame(window, cx);

    window
        .update(cx, |wb, _, _| {
            let m = wb.model_mut();
            for i in 0..2000 {
                m.output
                    .push("Test", &format!("line {i} with some text to show"));
            }
            m.activity = Activity::Explorer;
            m.layout.panel = true;
            m.panel_tab = PanelTab::Output;
        })
        .unwrap();
    let output = per_frame(window, cx);

    eprintln!("baseline {baseline:?}, 2000 hits {search:?}, 2000 output lines {output:?}");
    for (what, took) in [("search", search), ("output", output)] {
        assert!(
            took < baseline + Duration::from_millis(60),
            "{what} frame {took:?} against an empty frame {baseline:?}"
        );
    }
}
