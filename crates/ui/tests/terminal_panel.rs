use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gpui::{ClipboardItem, EntityInputHandler, TestAppContext, VisualTestContext, WindowHandle};
use ui::Workbench;
use ui::model::TabContent;

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn workspace(tag: &str) -> Tmp {
    let root = std::env::temp_dir().join(format!("nanti-ui-term-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("a.txt"), "x").unwrap();
    Tmp(root)
}

fn open<'a>(
    cx: &'a mut TestAppContext,
    root: &Path,
) -> (WindowHandle<Workbench>, &'a mut VisualTestContext) {
    let root = root.to_path_buf();
    let path = root.join("a.txt");
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

fn editor_text(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) -> String {
    window
        .read_with(cx, |wb, _| match &wb.model().active().unwrap().content {
            TabContent::Text(e) => e.text(),
            TabContent::Unavailable(r) => panic!("{r}"),
            TabContent::Diff(_) | TabContent::Webview => panic!("unexpected tab"),
        })
        .unwrap()
}

fn wait_text(
    window: WindowHandle<Workbench>,
    cx: &mut VisualTestContext,
    needle: &str,
    secs: u64,
) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(secs) {
        let found = window
            .update(cx, |wb, _, _| {
                wb.pump();
                wb.terminal_text().is_some_and(|t| t.contains(needle))
            })
            .unwrap();
        if found {
            return true;
        }
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(30));
    }
    false
}

fn screen(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) -> String {
    window
        .read_with(cx, |wb, _| wb.terminal_text().unwrap_or_default())
        .unwrap()
}

fn wait_prompt(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) {
    let start = Instant::now();
    loop {
        let ready = window
            .update(cx, |wb, _, _| {
                wb.pump();
                wb.terminal_text()
                    .is_some_and(|t| t.lines().any(|l| l.trim_end().ends_with('>')))
            })
            .unwrap();
        if ready {
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "no prompt:\n{}",
            screen(window, cx)
        );
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(30));
    }
}

fn process_alive(pid: u32) -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
        .unwrap_or(false)
}

fn shell_pid(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) -> u32 {
    cx.simulate_input("echo (\"pid=\"+$PID+\"=\")");
    cx.simulate_keystrokes("enter");
    let start = Instant::now();
    loop {
        window.update(cx, |wb, _, _| wb.pump()).unwrap();
        let text = screen(window, cx);
        let found = text
            .lines()
            .filter_map(|l| l.split("pid=").nth(1))
            .filter_map(|r| r.split('=').next())
            .find_map(|n| n.trim().parse::<u32>().ok());
        if let Some(pid) = found {
            return pid;
        }
        assert!(start.elapsed() < Duration::from_secs(15), "no pid:\n{text}");
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(30));
    }
}

#[gpui::test]
fn ctrl_backtick_opens_a_terminal_in_the_workspace_and_typing_runs_commands(
    cx: &mut TestAppContext,
) {
    let ws = workspace("t1");
    let (window, cx) = open(cx, &ws.0);
    cx.simulate_keystrokes("ctrl-`");
    assert_eq!(
        window.read_with(cx, |wb, _| wb.terminal_count()).unwrap(),
        1
    );
    wait_prompt(window, cx);
    let leaf = ws.0.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        screen(window, cx).contains(&leaf),
        "prompt shows the workspace:\n{}",
        screen(window, cx)
    );
    cx.simulate_input("echo (\"mark-\"+\"er-\"+(40+2))");
    cx.simulate_keystrokes("enter");
    assert!(
        wait_text(window, cx, "mark-er-42", 15),
        "{}",
        screen(window, cx)
    );
    assert_eq!(
        editor_text(window, cx),
        "x",
        "nothing leaked into the editor"
    );
}

#[gpui::test]
fn terminal_keeps_the_keyboard_until_toggled_away(cx: &mut TestAppContext) {
    let ws = workspace("t2");
    let (window, cx) = open(cx, &ws.0);
    cx.simulate_keystrokes("ctrl-`");
    wait_prompt(window, cx);
    cx.simulate_input("zzz");
    assert_eq!(editor_text(window, cx), "x");
    cx.simulate_keystrokes("ctrl-`");
    assert!(
        !window
            .read_with(cx, |wb, _| wb.model().layout.panel)
            .unwrap()
    );
    cx.simulate_input("Q");
    assert_eq!(editor_text(window, cx), "Qx");
    cx.simulate_keystrokes("ctrl-`");
    assert_eq!(
        window.read_with(cx, |wb, _| wb.terminal_count()).unwrap(),
        1
    );
}

#[gpui::test]
fn ctrl_c_interrupts_and_ctrl_v_pastes(cx: &mut TestAppContext) {
    let ws = workspace("t3");
    let (window, cx) = open(cx, &ws.0);
    cx.simulate_keystrokes("ctrl-`");
    wait_prompt(window, cx);
    let prompts = |wb: &Workbench| {
        wb.terminal_text()
            .map(|t| t.lines().filter(|l| l.trim_end().ends_with('>')).count())
            .unwrap_or(0)
    };
    cx.simulate_input("echo (\"busy-\"+\"now\"); Start-Sleep 60");
    cx.simulate_keystrokes("enter");
    assert!(wait_text(window, cx, "busy-now", 30), "the command started");
    std::thread::sleep(Duration::from_millis(300));
    let before = window.update(cx, |wb, _, _| prompts(wb)).unwrap();
    cx.simulate_keystrokes("ctrl-c");
    let start = Instant::now();
    while window
        .update(cx, |wb, _, _| {
            wb.pump();
            prompts(wb)
        })
        .unwrap()
        <= before
    {
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "no prompt after Ctrl+C:\n{}",
            screen(window, cx)
        );
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(30));
    }
    cx.write_to_clipboard(ClipboardItem::new_string(
        "echo (\"pa-\"+\"ste\")".to_owned(),
    ));
    cx.simulate_keystrokes("ctrl-v enter");
    assert!(
        wait_text(window, cx, "pa-ste", 15),
        "{}",
        screen(window, cx)
    );
    assert_eq!(editor_text(window, cx), "x");
}

#[gpui::test]
fn arrow_up_recalls_history_and_special_keys_reach_the_shell(cx: &mut TestAppContext) {
    let ws = workspace("t4");
    let (window, cx) = open(cx, &ws.0);
    cx.simulate_keystrokes("ctrl-`");
    wait_prompt(window, cx);
    cx.simulate_input("echo (\"hist-\"+\"ory\")");
    cx.simulate_keystrokes("enter");
    assert!(wait_text(window, cx, "hist-ory", 15));
    cx.simulate_keystrokes("up enter");
    let start = Instant::now();
    loop {
        window.update(cx, |wb, _, _| wb.pump()).unwrap();
        let n = screen(window, cx)
            .lines()
            .filter(|l| l.trim() == "hist-ory")
            .count();
        if n >= 2 {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "history not recalled:\n{}",
            screen(window, cx)
        );
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(30));
    }
}

#[gpui::test]
fn new_terminal_exit_status_and_kill_command(cx: &mut TestAppContext) {
    let ws = workspace("t5");
    let (window, cx) = open(cx, &ws.0);
    cx.simulate_keystrokes("ctrl-`");
    wait_prompt(window, cx);
    cx.simulate_keystrokes("ctrl-shift-`");
    assert_eq!(
        window.read_with(cx, |wb, _| wb.terminal_count()).unwrap(),
        2
    );
    wait_prompt(window, cx);
    cx.simulate_input("exit 3");
    cx.simulate_keystrokes("enter");
    let start = Instant::now();
    while window
        .update(cx, |wb, _, _| {
            wb.pump();
            wb.terminal_alive()
        })
        .unwrap()
        != Some(false)
    {
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "shell did not exit"
        );
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(30));
    }
    assert_eq!(
        window
            .read_with(cx, |wb, _| wb.terminal_exit_code())
            .unwrap(),
        Some(3)
    );
    cx.simulate_input("ignored"); // typing into an exited terminal must not panic or reach the editor
    assert_eq!(editor_text(window, cx), "x");
    cx.simulate_keystrokes("ctrl-shift-p");
    cx.simulate_input("kill active");
    cx.simulate_keystrokes("enter");
    assert_eq!(
        window.read_with(cx, |wb, _| wb.terminal_count()).unwrap(),
        1
    );
}

#[gpui::test]
fn palette_command_opens_a_terminal(cx: &mut TestAppContext) {
    let ws = workspace("t6");
    let (window, cx) = open(cx, &ws.0);
    cx.simulate_keystrokes("ctrl-shift-p");
    cx.simulate_input("new terminal");
    cx.simulate_keystrokes("enter");
    assert_eq!(
        window.read_with(cx, |wb, _| wb.terminal_count()).unwrap(),
        1
    );
    wait_prompt(window, cx);
    cx.simulate_input("echo (\"pal-\"+\"ok\")");
    cx.simulate_keystrokes("enter");
    assert!(wait_text(window, cx, "pal-ok", 15));
}

#[gpui::test]
fn ime_composition_stays_in_the_preedit_until_committed(cx: &mut TestAppContext) {
    let ws = workspace("t7");
    let (window, cx) = open(cx, &ws.0);
    cx.simulate_keystrokes("ctrl-`");
    wait_prompt(window, cx);
    window
        .update(cx, |wb, w, cx| {
            wb.replace_and_mark_text_in_range(None, "qzq", Some(3..3), w, cx)
        })
        .unwrap();
    let marked = window
        .update(cx, |wb, w, cx| wb.marked_text_range(w, cx))
        .unwrap();
    assert_eq!(marked, Some(0..3));
    std::thread::sleep(Duration::from_millis(200));
    window.update(cx, |wb, _, _| wb.pump()).unwrap();
    assert!(
        !screen(window, cx).contains("qzq"),
        "pre-edit text is not sent to the shell"
    );
    window
        .update(cx, |wb, w, cx| {
            wb.replace_text_in_range(None, "echo (\"im-\"+\"e-ok\")", w, cx)
        })
        .unwrap();
    assert_eq!(
        window
            .update(cx, |wb, w, cx| wb.marked_text_range(w, cx))
            .unwrap(),
        None
    );
    cx.simulate_keystrokes("enter");
    assert!(
        wait_text(window, cx, "im-e-ok", 15),
        "{}",
        screen(window, cx)
    );
    assert_eq!(editor_text(window, cx), "x");
}

#[gpui::test]
fn closing_the_window_kills_every_shell(cx: &mut TestAppContext) {
    let ws = workspace("t8");
    let (window, cx) = open(cx, &ws.0);
    cx.simulate_keystrokes("ctrl-`");
    wait_prompt(window, cx);
    let pid = shell_pid(window, cx);
    assert!(process_alive(pid));
    assert!(
        window
            .update(cx, |wb, _, _| wb.allow_window_close())
            .unwrap()
    );
    let start = Instant::now();
    while process_alive(pid) && start.elapsed() < Duration::from_secs(15) {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        !process_alive(pid),
        "shell {pid} survived the window closing"
    );
    assert_eq!(
        window.read_with(cx, |wb, _| wb.terminal_count()).unwrap(),
        0
    );
}

#[gpui::test]
fn terminal_and_editor_shortcuts_coexist(cx: &mut TestAppContext) {
    let ws = workspace("t9");
    let (window, cx) = open(cx, &ws.0);
    cx.simulate_keystrokes("ctrl-`");
    wait_prompt(window, cx);
    cx.simulate_keystrokes("ctrl-p");
    assert!(
        window
            .read_with(cx, |wb, _| wb.model().palette.is_some())
            .unwrap()
    );
    cx.simulate_keystrokes("escape");
    assert!(
        window
            .read_with(cx, |wb, _| wb.model().palette.is_none())
            .unwrap()
    );
    cx.simulate_input("echo (\"back-\"+\"ok\")");
    cx.simulate_keystrokes("enter");
    assert!(
        wait_text(window, cx, "back-ok", 15),
        "{}",
        screen(window, cx)
    );
}
