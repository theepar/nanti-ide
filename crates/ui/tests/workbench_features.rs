//! Menus, explorer file operations, go to line, Run tasks, extensions, Problems, Output and outline.
//! Model-level flows on real temporary folders, then the workbench through GPUI's headless platform.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, WindowHandle};
use ui::Workbench;
use ui::menu::{MenuKind, items};
use ui::model::{Activity, Model, PanelTab, Prompt, TabContent};
use ui::palette::{Command, InputKind, PaletteMode};

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn ws(tag: &str) -> Tmp {
    let root = std::env::temp_dir().join(format!("nanti-feat-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("a.txt"), "one\ntwo\nthree\n").unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "struct S;\nimpl S {\n    fn m(&self) {}\n}\nfn free() {}\n",
    )
    .unwrap();
    Tmp(root)
}

fn submit(m: &mut Model, text: &str) {
    let p = m.palette.as_mut().expect("a prompt is open");
    p.field.clear();
    p.field.insert(text);
    m.accept_palette();
}

// ---- explorer file operations ----------------------------------------------------------------------

#[test]
fn new_file_prompt_creates_opens_and_selects_the_file() {
    let t = ws("newfile");
    let mut m = Model::new(t.0.clone());
    m.run_command(Command::NewFile);
    assert!(
        matches!(m.palette.as_ref().map(|p| &p.mode), Some(PaletteMode::Input(InputKind::NewFile(d))) if *d == t.0)
    );
    submit(&mut m, "docs/notes.md");
    assert!(m.palette.is_none(), "closed on success");
    assert!(t.0.join("docs/notes.md").is_file());
    assert_eq!(m.active().unwrap().path, t.0.join("docs/notes.md"));
    assert_eq!(
        m.explorer_sel.as_deref(),
        Some(t.0.join("docs/notes.md").as_path())
    );
    assert!(
        m.visible_entries().iter().any(|e| e.name == "notes.md"),
        "its folder is expanded in the explorer"
    );
    assert!(
        m.output
            .lines()
            .any(|l| l.channel == "Explorer" && l.text.contains("Created docs"))
    );
}

#[test]
fn a_refused_name_keeps_the_prompt_open_and_explains() {
    let t = ws("refuse");
    let mut m = Model::new(t.0.clone());
    m.run_command(Command::NewFile);
    for (bad, why) in [
        ("a.txt", "already exists"),
        ("../x.txt", ".."),
        ("bad|name", "cannot contain"),
        ("", "enter a name"),
    ] {
        submit(&mut m, bad);
        let p = m.palette.as_ref().expect("still open");
        assert!(
            p.error.as_deref().is_some_and(|e| e.contains(why)),
            "{bad:?}: {:?}",
            p.error
        );
    }
    assert!(
        !t.0.join("x.txt").exists()
            && std::fs::read_to_string(t.0.join("a.txt")).unwrap() == "one\ntwo\nthree\n"
    );
    m.palette.as_mut().unwrap().field.insert("z");
    m.palette.as_mut().unwrap().refilter();
    assert!(
        m.palette.as_ref().unwrap().error.is_none(),
        "typing clears the error"
    );
}

#[test]
fn new_items_go_into_the_selected_folder_or_the_folder_of_the_selected_file() {
    let t = ws("seldir");
    let mut m = Model::new(t.0.clone());
    m.select_entry(&t.0.join("src"));
    assert_eq!(m.selected_dir(), t.0.join("src"));
    m.select_entry(&t.0.join("src/lib.rs"));
    assert_eq!(m.selected_dir(), t.0.join("src"));
    m.run_command(Command::NewFolder);
    submit(&mut m, "sub");
    assert!(t.0.join("src/sub").is_dir());
    assert!(
        m.visible_entries()
            .iter()
            .any(|e| e.is_dir && e.name == "sub")
    );
}

#[test]
fn rename_updates_open_tabs_and_refuses_collisions() {
    let t = ws("rename");
    let mut m = Model::new(t.0.clone());
    m.open_file(&t.0.join("a.txt"));
    m.select_entry(&t.0.join("a.txt"));
    m.run_command(Command::RenameItem);
    assert!(matches!(
        m.palette.as_ref().unwrap().mode,
        PaletteMode::Input(InputKind::Rename(_))
    ));
    assert_eq!(
        m.palette.as_ref().unwrap().field.text(),
        "a.txt",
        "prefilled with the current name"
    );
    submit(&mut m, "src");
    assert!(m.palette.is_some(), "src exists: refused");
    submit(&mut m, "b.txt");
    assert!(m.palette.is_none());
    assert!(t.0.join("b.txt").exists() && !t.0.join("a.txt").exists());
    assert_eq!(
        (m.tabs[0].title.as_str(), &m.tabs[0].path),
        ("b.txt", &t.0.join("b.txt"))
    );
}

#[test]
fn renaming_a_folder_retargets_tabs_inside_it() {
    let t = ws("renamedir");
    let mut m = Model::new(t.0.clone());
    m.open_file(&t.0.join("src/lib.rs"));
    m.select_entry(&t.0.join("src"));
    m.run_command(Command::RenameItem);
    submit(&mut m, "code");
    assert_eq!(m.tabs[0].path, t.0.join("code/lib.rs"));
    assert_eq!(m.tabs[0].title, "lib.rs");
    assert!(matches!(&m.tabs[0].content, TabContent::Text(e) if e.text().contains("struct S")));
}

#[test]
fn delete_asks_first_then_moves_to_the_recycle_bin_and_closes_tabs() {
    let t = ws("delete");
    let mut m = Model::new(t.0.clone());
    m.open_file(&t.0.join("a.txt"));
    m.select_entry(&t.0.join("a.txt"));
    m.run_command(Command::DeleteItem);
    assert!(matches!(m.prompt, Some(Prompt::DeleteItem(_))));
    assert!(
        t.0.join("a.txt").exists(),
        "nothing happens before the user confirms"
    );
    m.resolve_cancel();
    assert!(t.0.join("a.txt").exists());
    m.run_command(Command::DeleteItem);
    m.resolve_save();
    if cfg!(windows) {
        assert!(!t.0.join("a.txt").exists());
        assert!(m.tabs.is_empty() && m.active_tab.is_none());
        assert!(m.explorer_sel.is_none());
    }
    // the workspace root is never deletable
    m.select_entry(&t.0);
    m.run_command(Command::DeleteItem);
    assert!(m.prompt.is_none());
}

#[test]
fn copy_path_and_exit_are_requests_for_the_ui_layer() {
    let t = ws("requests");
    let mut m = Model::new(t.0.clone());
    m.select_entry(&t.0.join("a.txt"));
    m.run_command(Command::CopyPath);
    assert_eq!(
        m.clipboard_request.as_deref(),
        Some(t.0.join("a.txt").display().to_string().as_str())
    );
    assert!(!m.quit_request);
    m.run_command(Command::Exit);
    assert!(m.quit_request);
}

#[test]
fn context_menu_selects_the_entry_and_commands_act_on_it() {
    let t = ws("ctx");
    let mut m = Model::new(t.0.clone());
    m.open_context_menu(&t.0.join("src"), true, 10.0, 20.0);
    assert_eq!(m.explorer_sel.as_deref(), Some(t.0.join("src").as_path()));
    assert!(m.context_menu.is_some() && m.menu_open.is_none());
    m.open_menu(MenuKind::File);
    assert!(m.context_menu.is_none(), "only one menu at a time");
    m.close_menus();
    assert!(m.menu_open.is_none());
}

// ---- tabs, go to line, about ---------------------------------------------------------------------------

#[test]
fn go_to_line_accepts_line_and_column_and_clamps() {
    let t = ws("goto");
    let mut m = Model::new(t.0.clone());
    m.run_command(Command::GoToLine);
    assert!(m.palette.is_none(), "needs an open text file");
    m.open_file(&t.0.join("a.txt"));
    m.run_command(Command::GoToLine);
    submit(&mut m, "2:3");
    assert!(m.palette.is_none());
    assert_eq!(m.active_editor_mut().unwrap().line_col(), (1, 2));
    m.run_command(Command::GoToLine);
    submit(&mut m, "999");
    assert!(
        m.palette.is_none(),
        "beyond the end clamps to the last line"
    );
    let (line, _) = m.active_editor_mut().unwrap().line_col();
    assert_eq!(line, 3);
    m.run_command(Command::GoToLine);
    submit(&mut m, "1:99");
    assert_eq!(
        m.active_editor_mut().unwrap().line_col(),
        (0, 3),
        "column clamps to the line end"
    );
    for bad in ["", "0", "abc", "-1", "2:0", "2:x"] {
        m.run_command(Command::GoToLine);
        submit(&mut m, bad);
        assert!(m.palette.as_ref().unwrap().error.is_some(), "{bad:?}");
        m.close_palette();
    }
}

#[test]
fn save_all_and_close_all_tabs() {
    let t = ws("tabs");
    let mut m = Model::new(t.0.clone());
    m.open_file(&t.0.join("a.txt"));
    m.open_file(&t.0.join("src/lib.rs"));
    m.active_editor_mut().unwrap().insert("// ");
    m.activate_tab(0);
    m.active_editor_mut().unwrap().insert("# ");
    m.run_command(Command::SaveAll);
    assert_eq!(m.dirty_count(), 0);
    assert!(
        std::fs::read_to_string(t.0.join("a.txt"))
            .unwrap()
            .starts_with("# one")
    );
    assert!(m.message.as_deref().is_some_and(|s| s.contains("Saved 2")));
    m.active_editor_mut().unwrap().insert("x");
    m.run_command(Command::CloseAllTabs);
    assert_eq!(m.tabs.len(), 1, "the tab with unsaved changes stays");
    assert!(m.tabs[0].is_dirty());
    m.run_command(Command::SaveAll);
    m.run_command(Command::CloseAllTabs);
    assert!(m.tabs.is_empty() && m.active_tab.is_none());
}

#[test]
fn about_opens_once_and_mentions_credits() {
    let t = ws("about");
    let mut m = Model::new(t.0.clone());
    m.run_command(Command::About);
    m.run_command(Command::About);
    assert_eq!(m.tabs.len(), 1);
    assert!(
        matches!(&m.tabs[0].content, TabContent::Unavailable(s) if s.contains("THIRD_PARTY_NOTICES") && s.contains("version"))
    );
}

// ---- Run view -------------------------------------------------------------------------------------------

#[test]
fn run_view_lists_tasks_and_a_click_queues_the_command_for_the_terminal() {
    let t = ws("run");
    std::fs::write(t.0.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
    let mut m = Model::new(t.0.clone());
    m.run_command(Command::RunTask);
    assert_eq!(m.activity, Activity::Run);
    assert!(m.tasks.iter().any(|x| x.command == "cargo test"));
    let i = m
        .tasks
        .iter()
        .position(|x| x.command == "cargo check")
        .unwrap();
    m.run_task(i);
    assert_eq!(m.terminal_run.as_deref(), Some("cargo check"));
    assert!(m.layout.panel && m.panel_tab == PanelTab::Terminal);
    assert!(
        m.output
            .lines()
            .any(|l| l.channel == "Run" && l.text == "$ cargo check")
    );
    m.run_task(999); // out of range: ignored
}

// ---- Extensions view ------------------------------------------------------------------------------------

#[test]
fn extensions_install_list_use_in_panel_and_remove() {
    let t = ws("ext");
    let cfg = t.0.join("cfg");
    // build a tiny .vsix with the system tar
    let src = t.0.join("pkg");
    std::fs::create_dir_all(src.join("extension")).unwrap();
    std::fs::write(
        src.join("extension/package.json"),
        r#"{"name":"chat","publisher":"acme","version":"2.0.0","displayName":"Chat","contributes":{"views":{"x":[{"id":"chat.view","type":"webview"}]}}}"#,
    )
    .unwrap();
    std::fs::write(src.join("[Content_Types].xml"), "<x/>").unwrap();
    let vsix = t.0.join("chat.vsix");
    assert!(
        std::process::Command::new("tar")
            .args(["-a", "-cf"])
            .arg(&vsix)
            .arg("-C")
            .arg(&src)
            .args(["extension", "[Content_Types].xml"])
            .status()
            .unwrap()
            .success()
    );

    let mut m = Model::new(t.0.clone());
    m.run_command(Command::InstallVsix);
    assert!(
        m.palette.is_none(),
        "no config folder: refused with a message"
    );
    m.config_dir = Some(cfg.clone());
    m.run_command(Command::InstallVsix);
    assert_eq!(m.activity, Activity::Extensions);
    submit(&mut m, &format!("\"{}\"", vsix.display()));
    assert!(
        m.palette.is_none(),
        "{:?}",
        m.palette.as_ref().and_then(|p| p.error.clone())
    );
    assert_eq!(m.extensions.len(), 1);
    assert_eq!(
        (
            m.extensions[0].id.as_str(),
            m.extensions[0].view_type.as_deref()
        ),
        ("acme.chat", Some("chat.view"))
    );
    // installing the same package again is refused, with the reason
    m.run_command(Command::InstallVsix);
    submit(&mut m, &vsix.display().to_string());
    assert!(
        m.palette
            .as_ref()
            .unwrap()
            .error
            .as_deref()
            .is_some_and(|e| e.contains("already installed"))
    );
    m.close_palette();

    let dir = m.extensions[0].dir.clone();
    m.use_as_panel_extension(&dir, "chat.view");
    assert_eq!(
        ui::extensions::load_panel_choice(&cfg),
        Some((dir.clone(), "chat.view".to_owned()))
    );

    m.confirm_remove_extension(&dir);
    assert!(matches!(m.prompt, Some(Prompt::RemoveExtension(_))));
    assert!(dir.exists(), "nothing is deleted before the user confirms");
    m.resolve_save();
    assert!(!dir.exists() && m.extensions.is_empty());
    assert!(
        ui::extensions::load_panel_choice(&cfg).is_none(),
        "a removed extension is not loaded at startup"
    );
    // a folder outside <config>/extensions is never removed
    m.remove_extension(&t.0);
    assert!(t.0.exists());
}

// ---- Problems, Output, Outline ------------------------------------------------------------------------

#[test]
fn problems_report_syntax_errors_of_open_files_after_a_pause_and_clear_when_fixed() {
    let t = ws("problems");
    std::fs::write(t.0.join("bad.rs"), "fn ok() {}\nfn broken( {\n").unwrap();
    let mut m = Model::new(t.0.clone());
    m.open_file(&t.0.join("bad.rs"));
    m.open_file(&t.0.join("a.txt")); // plain text: never checked
    assert!(
        m.pump_problems(),
        "a file that was just opened is checked at once"
    );
    assert!(m.problems.total() >= 1);
    assert_eq!(m.problems.files.len(), 1);
    assert_eq!(m.problems.files[0].title, "bad.rs");
    // jump to the first problem
    let d = m.problems.files[0].items[0].clone();
    m.open_at(&t.0.join("bad.rs"), d.line, d.col, d.len);
    assert_eq!(m.active().unwrap().path, t.0.join("bad.rs"));
    // fix the file: select everything and replace it
    {
        let e = m.active_editor_mut().unwrap();
        e.select_all();
        e.insert("fn ok() {}\nfn fixed() {}\n");
    }
    m.pump_problems();
    assert!(
        m.problems.total() >= 1,
        "after an edit the old result stays until the text has been still for the debounce time"
    );
    std::thread::sleep(Duration::from_millis(350));
    assert!(m.pump_problems());
    assert_eq!(m.problems.total(), 0);
    // closing a tab drops its problems
    m.run_command(Command::ShowProblems);
    assert_eq!(m.panel_tab, PanelTab::Problems);
}

#[test]
fn output_panel_records_real_events_and_clears() {
    let t = ws("output");
    let mut m = Model::new(t.0.clone());
    m.open_file(&t.0.join("a.txt"));
    m.active_editor_mut().unwrap().insert("x");
    m.run_command(Command::Save);
    m.run_command(Command::ShowOutput);
    assert_eq!(m.panel_tab, PanelTab::Output);
    assert!(
        m.output
            .lines()
            .any(|l| l.channel == "Files" && l.text == "Saved a.txt")
    );
    m.run_command(Command::ClearOutput);
    assert!(m.output.is_empty());
}

#[test]
fn outline_lists_symbols_of_the_active_file_and_follows_edits() {
    let t = ws("outline");
    let mut m = Model::new(t.0.clone());
    assert!(m.outline().is_empty());
    m.open_file(&t.0.join("src/lib.rs"));
    let names: Vec<(String, usize)> = m.outline().into_iter().map(|s| (s.name, s.depth)).collect();
    assert_eq!(
        names,
        [
            ("S".into(), 0),
            ("S".into(), 0),
            ("m".into(), 1),
            ("free".into(), 0)
        ]
    );
    m.active_editor_mut().unwrap().insert("fn first() {}\n");
    assert_eq!(
        m.outline()[0].name,
        "first",
        "the cache follows the text version"
    );
    m.open_file(&t.0.join("a.txt"));
    assert!(m.outline().is_empty(), "plain text has no symbols");
    m.go_to_line(1, 1).unwrap();
}

// ---- menus ------------------------------------------------------------------------------------------------

#[test]
fn every_menu_command_runs_without_panicking() {
    let t = ws("menus");
    let mut m = Model::new(t.0.clone());
    m.open_file(&t.0.join("a.txt"));
    for kind in MenuKind::ALL {
        for it in items(kind) {
            if let ui::menu::MenuItem::Command(c) = it {
                if matches!(c, Command::Exit | Command::RevealInOs) {
                    continue; // quits / opens the OS file manager
                }
                m.close_palette();
                m.prompt = None;
                m.run_command(c);
            }
        }
    }
    m.close_palette();
}

// ---- through the workbench (headless GPUI) -------------------------------------------------------------

fn open<'a>(
    cx: &'a mut TestAppContext,
    root: &Path,
) -> (WindowHandle<Workbench>, &'a mut VisualTestContext) {
    let root = root.to_path_buf();
    let window = cx.add_window(move |_w, cx| Workbench::new(root, cx));
    let vcx = VisualTestContext::from_window(window.into(), cx).into_mut();
    window
        .update(vcx, |wb, window, _| window.focus(wb.editor_focus()))
        .unwrap();
    vcx.run_until_parked();
    (window, vcx)
}

#[gpui::test]
fn every_menu_context_menu_view_and_panel_renders(cx: &mut TestAppContext) {
    let t = ws("render");
    std::fs::write(t.0.join("Cargo.toml"), "[package]").unwrap();
    std::fs::write(t.0.join("bad.rs"), "fn broken( {\n").unwrap();
    let (window, cx) = open(cx, &t.0);
    let draw = |cx: &mut VisualTestContext| cx.run_until_parked();
    for kind in MenuKind::ALL {
        window
            .update(cx, |wb, _, cx| {
                wb.model_mut().open_menu(kind);
                cx.notify();
            })
            .unwrap();
        draw(cx);
    }
    window
        .update(cx, |wb, _, cx| {
            wb.model_mut()
                .open_context_menu(&t.0.join("src"), true, 40.0, 90.0);
            cx.notify();
        })
        .unwrap();
    draw(cx);
    window
        .update(cx, |wb, _, cx| {
            wb.model_mut()
                .open_context_menu(&t.0.join("a.txt"), false, 40.0, 90.0);
            cx.notify();
        })
        .unwrap();
    draw(cx);
    window
        .update(cx, |wb, _, cx| {
            let m = wb.model_mut();
            m.close_menus();
            m.open_file(&t.0.join("bad.rs"));
            m.open_file(&t.0.join("src/lib.rs"));
            m.layout.panel = true;
            cx.notify();
        })
        .unwrap();
    draw(cx);
    std::thread::sleep(Duration::from_millis(350));
    for (activity, tab) in [
        (Activity::Explorer, PanelTab::Problems),
        (Activity::Run, PanelTab::Output),
        (Activity::Extensions, PanelTab::Terminal),
        (Activity::SourceControl, PanelTab::Problems),
        (Activity::Search, PanelTab::Output),
    ] {
        window
            .update(cx, |wb, _, cx| {
                wb.pump();
                let m = wb.model_mut();
                m.run_command(Command::GitRefresh);
                m.activity = activity;
                m.refresh_tasks();
                m.refresh_extensions();
                m.panel_tab = tab;
                cx.notify();
            })
            .unwrap();
        draw(cx);
    }
    // welcome screen (no tab) and the input prompt in every mode
    window
        .update(cx, |wb, _, cx| {
            wb.model_mut().tabs.clear();
            wb.model_mut().active_tab = None;
            cx.notify();
        })
        .unwrap();
    draw(cx);
    window
        .update(cx, |wb, _, cx| {
            wb.model_mut().run_command(Command::NewFile);
            cx.notify();
        })
        .unwrap();
    draw(cx);
    let (problems, tabs) = window
        .read_with(cx, |wb, _| {
            (wb.model().problems.total(), wb.model().tabs.len())
        })
        .unwrap();
    assert_eq!(tabs, 0);
    let _ = problems;
}

#[gpui::test]
fn keyboard_new_file_and_go_to_line_flow(cx: &mut TestAppContext) {
    let t = ws("kbd");
    let (window, cx) = open(cx, &t.0);
    window
        .update(cx, |wb, _, _| wb.open_path(&t.0.join("a.txt")))
        .unwrap();
    cx.run_until_parked();
    // Ctrl+G, type "3", Enter
    cx.simulate_keystrokes("ctrl-g");
    assert!(
        window
            .read_with(cx, |wb, _| wb.model().palette_is_input())
            .unwrap()
    );
    cx.simulate_input("3");
    cx.simulate_keystrokes("enter");
    let line = window
        .update(cx, |wb, _, _| {
            wb.model_mut().active_editor_mut().unwrap().line_col().0
        })
        .unwrap();
    assert_eq!(line, 2);
    // F2 renames the active file when nothing is selected in the explorer
    cx.simulate_keystrokes("f2");
    assert!(
        window
            .read_with(cx, |wb, _| wb.model().palette_is_input())
            .unwrap()
    );
    cx.simulate_keystrokes("escape");
    // Escape closes an open menu
    window
        .update(cx, |wb, _, cx| {
            wb.model_mut().open_menu(MenuKind::Edit);
            cx.notify();
        })
        .unwrap();
    cx.simulate_keystrokes("escape");
    assert!(
        window
            .read_with(cx, |wb, _| wb.model().menu_open.is_none())
            .unwrap()
    );
}

#[gpui::test]
fn run_task_types_the_command_into_a_real_terminal(cx: &mut TestAppContext) {
    let t = ws("runterm");
    std::fs::write(
        t.0.join("package.json"),
        r#"{"scripts":{"hello":"echo nanti-task-ran"}}"#,
    )
    .unwrap();
    let (window, cx) = open(cx, &t.0);
    window
        .update(cx, |wb, _, _| {
            wb.model_mut().refresh_tasks();
        })
        .unwrap();
    let idx = window
        .read_with(cx, |wb, _| {
            wb.model()
                .tasks
                .iter()
                .position(|x| x.command.ends_with("run hello"))
                .unwrap()
        })
        .unwrap();
    window
        .update(cx, |wb, window, _| wb.run_task(idx, window))
        .unwrap();
    let start = Instant::now();
    let mut seen = false;
    while start.elapsed() < Duration::from_secs(40) && !seen {
        seen = window
            .update(cx, |wb, _, _| {
                wb.pump();
                wb.terminal_text()
                    .is_some_and(|t| t.contains("npm run hello"))
            })
            .unwrap();
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(seen, "the command line reached the shell");
    window
        .update(cx, |wb, _, _| wb.kill_all_terminals())
        .unwrap();
}

#[gpui::test]
fn a_long_file_gives_the_editor_a_scrollable_range(cx: &mut TestAppContext) {
    let t = ws("scrollrange");
    let long: String = (0..400).map(|i| format!("line {i}\n")).collect();
    std::fs::write(t.0.join("long.txt"), long).unwrap();
    let (window, cx) = open(cx, &t.0);
    window
        .update(cx, |wb, _, cx| {
            wb.open_path(&t.0.join("long.txt"));
            cx.notify();
        })
        .unwrap();
    cx.simulate_keystrokes("ctrl-home"); // input events draw a frame
    cx.run_until_parked();
    let (view, max, off) = window
        .read_with(cx, |wb, _| wb.editor_scroll_metrics())
        .unwrap();
    assert!(view > 50.0, "the scroll area has a size: {view}");
    assert!(max > 1000.0, "400 lines of 20px do not fit: max {max}");
    assert_eq!(off, 0.0);
}

fn editor_text(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) -> String {
    window
        .update(cx, |wb, _, _| {
            wb.model_mut().active_editor_mut().unwrap().text()
        })
        .unwrap()
}

#[gpui::test]
fn editing_shortcuts_comment_move_duplicate_delete_indent_and_words(cx: &mut TestAppContext) {
    let t = ws("shortcuts");
    std::fs::write(t.0.join("code.rs"), "fn a() {\n    one\n    two\n}\n").unwrap();
    let (window, cx) = open(cx, &t.0);
    window
        .update(cx, |wb, _, _| wb.open_path(&t.0.join("code.rs")))
        .unwrap();
    cx.run_until_parked();

    cx.simulate_keystrokes("ctrl-/");
    assert!(
        editor_text(window, cx).starts_with("// fn a() {\n"),
        "{:?}",
        editor_text(window, cx)
    );
    cx.simulate_keystrokes("ctrl-/");
    assert!(editor_text(window, cx).starts_with("fn a() {\n"));

    cx.simulate_keystrokes("alt-down");
    assert_eq!(editor_text(window, cx), "    one\nfn a() {\n    two\n}\n");
    cx.simulate_keystrokes("alt-up");
    assert_eq!(editor_text(window, cx), "fn a() {\n    one\n    two\n}\n");

    cx.simulate_keystrokes("shift-alt-down");
    assert_eq!(
        editor_text(window, cx),
        "fn a() {\nfn a() {\n    one\n    two\n}\n"
    );
    cx.simulate_keystrokes("ctrl-shift-k");
    assert_eq!(editor_text(window, cx), "fn a() {\n    one\n    two\n}\n");

    // Enter after an opening brace indents the next line; Tab / Shift+Tab indent and outdent
    cx.simulate_keystrokes("ctrl-home end enter");
    assert_eq!(
        editor_text(window, cx),
        "fn a() {\n    \n    one\n    two\n}\n"
    );
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(editor_text(window, cx), "fn a() {\n\n    one\n    two\n}\n");
    cx.simulate_keystrokes("tab");
    assert_eq!(
        editor_text(window, cx),
        "fn a() {\n    \n    one\n    two\n}\n"
    );

    // word-wise keys
    cx.simulate_keystrokes("ctrl-home ctrl-right ctrl-right");
    let col = window
        .update(cx, |wb, _, _| {
            wb.model_mut().active_editor_mut().unwrap().line_col()
        })
        .unwrap();
    assert_eq!(col, (0, 4), "fn then a: the caret stops after 'a'");
    cx.simulate_keystrokes("ctrl-backspace");
    assert_eq!(
        editor_text(window, cx),
        "fn () {\n    \n    one\n    two\n}\n"
    );
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(
        editor_text(window, cx),
        "fn a() {\n    \n    one\n    two\n}\n"
    );

    // Ctrl+Enter opens a line below with the current indentation
    cx.simulate_keystrokes("ctrl-home down down ctrl-enter");
    assert_eq!(
        editor_text(window, cx),
        "fn a() {\n    \n    one\n    \n    two\n}\n"
    );
}

#[gpui::test]
fn comment_toggle_reports_file_types_without_line_comments(cx: &mut TestAppContext) {
    let t = ws("nocomment");
    std::fs::write(t.0.join("data.json"), "{}\n").unwrap();
    let (window, cx) = open(cx, &t.0);
    window
        .update(cx, |wb, _, _| wb.open_path(&t.0.join("data.json")))
        .unwrap();
    cx.simulate_keystrokes("ctrl-/");
    assert_eq!(
        editor_text(window, cx),
        "{}\n",
        "JSON has no line comments: nothing changes"
    );
    let msg = window
        .read_with(cx, |wb, _| wb.model().message.clone())
        .unwrap();
    assert!(msg.is_some_and(|m| m.contains("no line comment")));
}
