//! Source control (GT-01, GT-02) against REAL git repositories: model-level flows, then the
//! workbench through GPUI's headless platform.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, WindowHandle};
use ui::Workbench;
use ui::model::{Activity, Model, TabContent};
use ui::scm::{DiffState, GroupKind, Tone};

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = Command::new("cmd")
            .args(["/C", "rmdir", "/S", "/Q"])
            .arg(&self.0)
            .output();
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A repo with one commit containing a.txt and dir/b.txt.
fn repo(tag: &str) -> Tmp {
    let root = std::env::temp_dir().join(format!("nanti-ui-git-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("dir")).unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
    for (k, v) in [
        ("user.name", "Nanti Test"),
        ("user.email", "test@example.com"),
        ("commit.gpgsign", "false"),
        ("core.autocrlf", "false"),
    ] {
        git(&root, &["config", k, v]);
    }
    std::fs::write(root.join("a.txt"), "one\ntwo\nthree\n").unwrap();
    std::fs::write(root.join("dir/b.txt"), "bee\n").unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "base"]);
    Tmp(root)
}

/// Apply git results until nothing is running or queued.
fn settle(m: &mut Model) {
    let start = Instant::now();
    loop {
        m.pump_scm();
        if !m.scm.is_busy() {
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "git never finished"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn model(root: &Path) -> Model {
    let mut m = Model::new(root.to_path_buf());
    m.enable_source_control();
    settle(&mut m);
    m
}

fn diff_state(m: &Model, tab: usize) -> &DiffState {
    match &m.tabs[tab].content {
        TabContent::Diff(v) => &v.state,
        _ => panic!("tab {tab} is not a diff"),
    }
}

fn rels(m: &Model, g: GroupKind) -> Vec<String> {
    m.scm.group(g).into_iter().map(|r| r.rel).collect()
}

#[test]
fn status_groups_branch_and_explorer_decorations() {
    let t = repo("status");
    std::fs::write(t.0.join("a.txt"), "one\nTWO\nthree\n").unwrap();
    std::fs::write(t.0.join("new.txt"), "n\n").unwrap();
    std::fs::remove_file(t.0.join("dir/b.txt")).unwrap();
    let m = model(&t.0);
    assert_eq!(m.scm.error, None);
    assert_eq!(m.branch_label(), "⎇ main*");
    assert_eq!(
        rels(&m, GroupKind::Changes),
        ["a.txt", "dir/b.txt", "new.txt"]
    );
    assert!(rels(&m, GroupKind::Staged).is_empty());
    let d = |p: &str, dir: bool| {
        m.scm
            .decor_for(&t.0.join(p), dir)
            .map(|d| (d.letter, d.tone))
    };
    assert_eq!(d("a.txt", false), Some(('M', Tone::Modified)));
    assert_eq!(d("new.txt", false), Some(('U', Tone::Added)));
    assert_eq!(d("dir/b.txt", false), Some(('D', Tone::Removed)));
    assert_eq!(d("dir", true), Some(('•', Tone::Modified)));
    assert_eq!(d("nothing.txt", false), None);
}

#[test]
fn a_clean_repo_shows_no_changes_and_a_plain_branch() {
    let t = repo("clean");
    let m = model(&t.0);
    assert_eq!(m.branch_label(), "⎇ main");
    assert_eq!(m.scm.change_count(), 0);
}

#[test]
fn stage_unstage_and_commit_change_the_real_repository() {
    let t = repo("flow");
    std::fs::write(t.0.join("a.txt"), "one\nTWO\nthree\n").unwrap();
    std::fs::write(t.0.join("new.txt"), "n\n").unwrap();
    let mut m = model(&t.0);

    m.scm.stage("a.txt");
    settle(&mut m);
    assert_eq!(rels(&m, GroupKind::Staged), ["a.txt"]);
    assert_eq!(
        git(&t.0, &["diff", "--cached", "--name-only"]).trim(),
        "a.txt"
    );

    m.scm.unstage("a.txt");
    settle(&mut m);
    assert!(rels(&m, GroupKind::Staged).is_empty());

    m.scm.stage_all();
    settle(&mut m);
    assert_eq!(rels(&m, GroupKind::Staged), ["a.txt", "new.txt"]);
    m.scm.unstage_all();
    settle(&mut m);
    assert_eq!(m.scm.staged_count(), 0);

    // refused before git runs: no message, then nothing staged
    assert!(m.scm.commit().is_err());
    m.scm.message.insert("only message");
    assert!(m.scm.commit().unwrap_err().contains("nothing is staged"));
    assert_eq!(git(&t.0, &["rev-list", "--count", "HEAD"]).trim(), "1");

    m.scm.stage("a.txt");
    settle(&mut m);
    assert!(m.scm.commit().is_ok());
    settle(&mut m);
    assert_eq!(git(&t.0, &["rev-list", "--count", "HEAD"]).trim(), "2");
    assert_eq!(
        git(&t.0, &["log", "-1", "--format=%s"]).trim(),
        "only message"
    );
    assert_eq!(
        m.scm.message.text(),
        "",
        "the message is cleared after a successful commit"
    );
    assert!(matches!(&m.scm.notice, Some((false, n)) if n.starts_with("Committed:")));
    assert_eq!(
        rels(&m, GroupKind::Changes),
        ["new.txt"],
        "the untouched file stays untracked"
    );
    assert!(
        m.message
            .as_deref()
            .is_some_and(|s| s.starts_with("Committed"))
    );
}

#[test]
fn a_failing_hook_keeps_the_message_and_shows_its_output() {
    let t = repo("hook");
    std::fs::write(t.0.join("a.txt"), "changed\n").unwrap();
    std::fs::write(
        t.0.join(".git/hooks/pre-commit"),
        "#!/bin/sh\necho 'hook says no' >&2\nexit 1\n",
    )
    .unwrap();
    let mut m = model(&t.0);
    m.scm.stage("a.txt");
    settle(&mut m);
    m.scm.message.insert("blocked");
    m.scm.commit().unwrap();
    settle(&mut m);
    assert!(
        matches!(&m.scm.notice, Some((true, n)) if n.contains("hook says no")),
        "{:?}",
        m.scm.notice
    );
    assert_eq!(
        m.scm.message.text(),
        "blocked",
        "the user's message survives a failed commit"
    );
    assert_eq!(git(&t.0, &["rev-list", "--count", "HEAD"]).trim(), "1");
    assert_eq!(m.scm.staged_count(), 1);
}

#[test]
fn diff_tab_loads_dedupes_and_follows_stage_changes() {
    let t = repo("difftab");
    std::fs::write(t.0.join("a.txt"), "one\nTWO\nthree\nfour\n").unwrap();
    let mut m = model(&t.0);

    m.open_diff("a.txt", false);
    assert_eq!(m.tabs.len(), 1);
    assert_eq!(m.tabs[0].title, "a.txt (Working Tree)");
    assert!(matches!(diff_state(&m, 0), DiffState::Loading));
    settle(&mut m);
    match diff_state(&m, 0) {
        DiffState::Ready(d) => assert_eq!((d.added(), d.removed()), (2, 1)),
        other => panic!("{other:?}"),
    }
    m.open_diff("a.txt", false);
    assert_eq!(m.tabs.len(), 1, "the same diff is not opened twice");

    // staging moves the change to the Index: the working-tree tab empties, the Index tab fills
    m.scm.stage("a.txt");
    settle(&mut m);
    assert!(matches!(diff_state(&m, 0), DiffState::Ready(d) if d.is_empty()));
    m.open_diff("a.txt", true);
    assert_eq!(m.tabs.len(), 2);
    assert_eq!(m.tabs[1].title, "a.txt (Index)");
    settle(&mut m);
    assert!(matches!(diff_state(&m, 1), DiffState::Ready(d) if d.added() == 2));

    // committing empties the Index tab too
    m.scm.message.insert("c");
    m.scm.commit().unwrap();
    settle(&mut m);
    assert!(matches!(diff_state(&m, 1), DiffState::Ready(d) if d.is_empty()));
}

#[test]
fn diff_tabs_never_mix_with_file_tabs() {
    let t = repo("mix");
    std::fs::write(t.0.join("a.txt"), "changed\n").unwrap();
    let mut m = model(&t.0);
    m.open_diff("a.txt", false);
    settle(&mut m);
    m.open_file(&t.0.join("a.txt"));
    assert_eq!(m.tabs.len(), 2, "the file opens in its own editor tab");
    assert!(matches!(m.tabs[1].content, TabContent::Text(_)));
    m.activate_tab(0);
    m.open_file(&t.0.join("a.txt"));
    assert_eq!(
        m.active_tab,
        Some(1),
        "open_file finds the editor tab, not the diff tab"
    );
    assert!(m.tabs[0].stamp.is_none() && !m.tabs[0].is_dirty());
    assert!(m.active_editor_mut().is_some());
    m.activate_tab(0);
    assert!(m.active_editor_mut().is_none(), "a diff tab has no editor");
    assert_eq!(m.save_tab(0, true), ui::model::SaveOutcome::NothingToSave);
}

#[test]
fn untracked_binary_deleted_and_renamed_files_open_sensible_diffs() {
    let t = repo("kinds");
    std::fs::write(t.0.join("untracked.txt"), "x\ny\n").unwrap();
    std::fs::write(t.0.join("blob.bin"), [0u8, 1, 2, 0, 9]).unwrap();
    std::fs::remove_file(t.0.join("dir/b.txt")).unwrap();
    git(&t.0, &["mv", "a.txt", "renamed.txt"]);
    let mut m = model(&t.0);
    m.open_diff("untracked.txt", false);
    m.open_diff("blob.bin", false);
    m.open_diff("dir/b.txt", false);
    m.open_diff("renamed.txt", true);
    settle(&mut m);
    assert!(
        matches!(diff_state(&m, 0), DiffState::Ready(d) if d.added() == 2 && d.notes == ["untracked file"])
    );
    assert!(matches!(diff_state(&m, 1), DiffState::Ready(d) if d.binary));
    assert!(
        matches!(diff_state(&m, 2), DiffState::Ready(d) if d.removed() == 1 && d.notes == ["deleted file"])
    );
    assert!(matches!(diff_state(&m, 3), DiffState::Ready(d) if d.notes == ["renamed from a.txt"]));
    // unstaging a rename restores both paths
    m.scm.unstage("renamed.txt");
    settle(&mut m);
    assert!(
        rels(&m, GroupKind::Staged).is_empty(),
        "{:?}",
        rels(&m, GroupKind::Staged)
    );
}

#[test]
fn a_conflicted_file_opens_in_the_editor_not_as_a_diff() {
    let t = repo("conflict");
    git(&t.0, &["checkout", "-q", "-b", "other"]);
    std::fs::write(t.0.join("a.txt"), "other\n").unwrap();
    git(&t.0, &["commit", "-qam", "other"]);
    git(&t.0, &["checkout", "-q", "main"]);
    std::fs::write(t.0.join("a.txt"), "main\n").unwrap();
    git(&t.0, &["commit", "-qam", "main"]);
    let merge = Command::new("git")
        .current_dir(&t.0)
        .args(["merge", "other"])
        .output()
        .unwrap();
    assert!(!merge.status.success());
    let mut m = model(&t.0);
    assert_eq!(rels(&m, GroupKind::Conflicts), ["a.txt"]);
    assert!(rels(&m, GroupKind::Changes).is_empty());
    m.open_diff("a.txt", false);
    assert_eq!(m.tabs.len(), 1);
    assert!(
        matches!(&m.tabs[0].content, TabContent::Text(e) if e.text().contains("<<<<<<<")),
        "the conflict markers are shown for editing"
    );
}

#[test]
fn saving_a_file_refreshes_status_and_open_diffs() {
    let t = repo("save");
    let mut m = model(&t.0);
    assert_eq!(m.scm.change_count(), 0);
    m.open_diff("a.txt", false);
    m.open_file(&t.0.join("a.txt"));
    m.active_editor_mut().unwrap().insert("EDIT ");
    assert_eq!(m.save_active(), ui::model::SaveOutcome::Saved);
    settle(&mut m);
    assert_eq!(rels(&m, GroupKind::Changes), ["a.txt"]);
    assert!(matches!(diff_state(&m, 0), DiffState::Ready(d) if d.added() == 1 && d.removed() == 1));
}

#[test]
fn outside_a_repository_the_panel_explains_instead_of_failing() {
    let root = std::env::temp_dir().join(format!("nanti-ui-git-none-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let t = Tmp(root);
    let mut m = model(&t.0);
    assert!(m.scm.status.is_none());
    assert!(
        m.scm
            .error
            .as_deref()
            .is_some_and(|e| e.contains("not inside a Git repository")),
        "{:?}",
        m.scm.error
    );
    assert_eq!(m.branch_label(), "no repository");
    assert!(m.scm.commit().is_err());
    m.open_diff("x.txt", false); // must not panic or hang
    settle(&mut m);
}

#[test]
fn a_workspace_inside_a_subfolder_of_the_repo_still_maps_paths() {
    let t = repo("subdir");
    std::fs::write(t.0.join("dir/b.txt"), "changed\n").unwrap();
    std::fs::write(t.0.join("a.txt"), "changed too\n").unwrap();
    let sub = t.0.join("dir");
    let mut m = model(&sub);
    assert_eq!(m.scm.error, None);
    assert_eq!(
        rels(&m, GroupKind::Changes),
        ["a.txt", "dir/b.txt"],
        "status paths are relative to the repo root"
    );
    assert_eq!(
        m.scm.decor_for(&sub.join("b.txt"), false).map(|d| d.letter),
        Some('M')
    );
    m.open_diff("dir/b.txt", false);
    settle(&mut m);
    assert!(matches!(diff_state(&m, 0), DiffState::Ready(d) if d.added() == 1));
}

#[test]
fn source_control_commands_run_from_the_command_model() {
    use ui::palette::Command;
    let t = repo("cmds");
    std::fs::write(t.0.join("a.txt"), "changed\n").unwrap();
    let mut m = model(&t.0);
    m.run_command(Command::GitStageAll);
    settle(&mut m);
    assert_eq!(m.scm.staged_count(), 1);
    m.run_command(Command::GitUnstageAll);
    settle(&mut m);
    assert_eq!(m.scm.staged_count(), 0);
    // commit without a message: opens Source Control and puts the caret in the message field
    m.activity = Activity::Explorer;
    m.run_command(Command::GitCommit);
    assert_eq!(m.activity, Activity::SourceControl);
    assert!(m.scm.focused);
    assert!(matches!(&m.scm.notice, Some((true, n)) if n.contains("commit message")));
    m.run_command(Command::GitRefresh);
    settle(&mut m);
}

// ---- through the workbench (headless GPUI) --------------------------------------------------

fn open<'a>(
    cx: &'a mut TestAppContext,
    root: &Path,
) -> (WindowHandle<Workbench>, &'a mut VisualTestContext) {
    let root = root.to_path_buf();
    let window = cx.add_window(move |_w, cx| {
        let mut wb = Workbench::new(root, cx);
        wb.enable_source_control();
        wb
    });
    let vcx = VisualTestContext::from_window(window.into(), cx).into_mut();
    window
        .update(vcx, |wb, window, _| window.focus(wb.editor_focus()))
        .unwrap();
    vcx.run_until_parked();
    (window, vcx)
}

fn wait_idle(window: WindowHandle<Workbench>, cx: &mut VisualTestContext) {
    let start = Instant::now();
    loop {
        let busy = window
            .update(cx, |wb, _, _| {
                wb.pump();
                wb.model().scm.is_busy()
            })
            .unwrap();
        cx.run_until_parked();
        if !busy {
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "git never finished"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[gpui::test]
fn keyboard_flow_opens_the_view_types_a_message_and_commits(cx: &mut TestAppContext) {
    let t = repo("ui-flow");
    std::fs::write(t.0.join("a.txt"), "edited\n").unwrap();
    let (window, cx) = open(cx, &t.0);
    wait_idle(window, cx);

    cx.simulate_keystrokes("ctrl-shift-g");
    let (activity, sidebar) = window
        .read_with(cx, |wb, _| (wb.model().activity, wb.model().layout.sidebar))
        .unwrap();
    assert_eq!((activity, sidebar), (Activity::SourceControl, true));
    wait_idle(window, cx);

    // a Ctrl+Enter with an empty message is refused with an explanation, nothing is committed
    window
        .update(cx, |wb, window, cx| {
            wb.focus_commit_message(window);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    cx.simulate_keystrokes("ctrl-enter");
    let notice = window
        .read_with(cx, |wb, _| wb.model().scm.notice.clone())
        .unwrap();
    assert!(
        matches!(&notice, Some((true, n)) if n.contains("commit message")),
        "{notice:?}"
    );

    // type a message (IME-style text input path), stage, commit with the keyboard
    cx.simulate_input("fix the thing");
    window
        .update(cx, |wb, _, _| wb.model_mut().scm.stage("a.txt"))
        .unwrap();
    wait_idle(window, cx);
    let typed = window
        .read_with(cx, |wb, _| wb.model().scm.message.text().to_owned())
        .unwrap();
    assert_eq!(
        typed, "fix the thing",
        "typing went to the commit field, not the editor"
    );
    cx.simulate_keystrokes("ctrl-enter");
    wait_idle(window, cx);
    assert_eq!(
        git(&t.0, &["log", "-1", "--format=%s"]).trim(),
        "fix the thing"
    );
    assert_eq!(git(&t.0, &["rev-list", "--count", "HEAD"]).trim(), "2");
    let cleared = window
        .read_with(cx, |wb, _| wb.model().scm.message.text().is_empty())
        .unwrap();
    assert!(cleared);
}

#[gpui::test]
fn plain_enter_in_the_message_field_never_commits(cx: &mut TestAppContext) {
    let t = repo("ui-enter");
    std::fs::write(t.0.join("a.txt"), "edited\n").unwrap();
    let (window, cx) = open(cx, &t.0);
    wait_idle(window, cx);
    window
        .update(cx, |wb, window, cx| {
            let m = wb.model_mut();
            m.activity = Activity::SourceControl;
            m.scm.stage("a.txt");
            wb.focus_commit_message(window);
            cx.notify();
        })
        .unwrap();
    wait_idle(window, cx);
    cx.simulate_input("msg");
    cx.simulate_keystrokes("enter");
    wait_idle(window, cx);
    assert_eq!(
        git(&t.0, &["rev-list", "--count", "HEAD"]).trim(),
        "1",
        "Enter alone is not a commit"
    );
    let text = window
        .read_with(cx, |wb, _| wb.model().scm.message.text().to_owned())
        .unwrap();
    assert_eq!(text, "msg");
}

#[gpui::test]
fn rendering_every_source_control_state_does_not_panic(cx: &mut TestAppContext) {
    let t = repo("ui-render");
    std::fs::write(t.0.join("a.txt"), "one\nTWO\nthree\n").unwrap();
    std::fs::write(t.0.join("new.txt"), "n\n").unwrap();
    std::fs::write(t.0.join("blob.bin"), [0u8, 1, 0]).unwrap();
    git(&t.0, &["add", "new.txt"]);
    let (window, cx) = open(cx, &t.0);
    wait_idle(window, cx);
    window
        .update(cx, |wb, _, _| {
            let m = wb.model_mut();
            m.activity = Activity::SourceControl;
            m.open_diff("a.txt", false);
            m.open_diff("new.txt", true);
            m.open_diff("blob.bin", false);
        })
        .unwrap();
    for _ in 0..3 {
        wait_idle(window, cx);
    }
    // every tab (diff, diff, diff) drawn, then the collapsed variant and the explorer decorations
    for i in 0..3 {
        window
            .update(cx, |wb, _, cx| {
                wb.model_mut().activate_tab(i);
                cx.notify();
            })
            .unwrap();
        cx.run_until_parked();
    }
    window
        .update(cx, |wb, _, cx| {
            let m = wb.model_mut();
            m.scm.toggle_group(GroupKind::Staged);
            m.scm.toggle_group(GroupKind::Changes);
            m.activity = Activity::Explorer;
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    let states = window
        .read_with(cx, |wb, _| {
            (0..3)
                .map(|i| match &wb.model().tabs[i].content {
                    TabContent::Diff(v) => matches!(v.state, DiffState::Ready(_)),
                    _ => false,
                })
                .collect::<Vec<_>>()
        })
        .unwrap();
    assert_eq!(states, [true, true, true]);
}
