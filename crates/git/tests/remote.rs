use std::path::{Path, PathBuf};
use std::process::Command;

use git::Repo;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=t@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

struct Dirs {
    base: PathBuf,
    remote: PathBuf,
    work: PathBuf,
    other: PathBuf,
}

impl Drop for Dirs {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

fn setup(tag: &str) -> Dirs {
    let base = std::env::temp_dir().join(format!("nanti-git-remote-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (remote, work, other) = (
        base.join("remote.git"),
        base.join("work"),
        base.join("other"),
    );
    std::fs::create_dir_all(&remote).unwrap();
    git(&remote, &["init", "-q", "--bare", "-b", "main"]);
    git(&base, &["clone", "-q", remote.to_str().unwrap(), "work"]);
    git(&work, &["checkout", "-q", "-B", "main"]);
    std::fs::write(work.join("a.txt"), "one\n").unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "first"]);
    Dirs {
        base,
        remote,
        work,
        other,
    }
}

#[test]
fn push_publishes_a_new_branch_then_pushes_new_commits() {
    let d = setup("push");
    let repo = Repo::discover(&d.work).unwrap();
    let msg = repo.push().expect("the first push sets the upstream");
    assert!(!msg.is_empty());
    assert_eq!(git(&d.remote, &["rev-list", "--count", "main"]).trim(), "1");
    std::fs::write(d.work.join("a.txt"), "two\n").unwrap();
    git(&d.work, &["commit", "-q", "-am", "second"]);
    repo.push().expect("a later push uses the upstream");
    assert_eq!(git(&d.remote, &["rev-list", "--count", "main"]).trim(), "2");
}

#[test]
fn pull_fast_forwards_and_refuses_a_diverged_branch() {
    let d = setup("pull");
    let repo = Repo::discover(&d.work).unwrap();
    repo.push().unwrap();
    git(
        &d.base,
        &["clone", "-q", d.remote.to_str().unwrap(), "other"],
    );
    std::fs::write(d.other.join("b.txt"), "b\n").unwrap();
    git(&d.other, &["add", "-A"]);
    git(&d.other, &["commit", "-q", "-m", "from other"]);
    git(&d.other, &["push", "-q"]);

    repo.fetch().expect("fetch works");
    assert_eq!(repo.status().unwrap().behind, 1);
    repo.pull().expect("fast-forward pull");
    assert!(d.work.join("b.txt").exists());

    std::fs::write(d.work.join("a.txt"), "mine\n").unwrap();
    git(&d.work, &["commit", "-q", "-am", "mine"]);
    std::fs::write(d.other.join("c.txt"), "c\n").unwrap();
    git(&d.other, &["add", "-A"]);
    git(&d.other, &["commit", "-q", "-m", "theirs"]);
    git(&d.other, &["push", "-q"]);
    repo.fetch().unwrap();
    assert!(
        repo.pull().is_err(),
        "diverged history is never merged silently"
    );
}

#[test]
fn push_without_a_remote_explains_itself() {
    let base = std::env::temp_dir().join(format!("nanti-git-noremote-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    git(&base, &["init", "-q", "-b", "main"]);
    std::fs::write(base.join("a.txt"), "x\n").unwrap();
    git(&base, &["add", "-A"]);
    git(&base, &["commit", "-q", "-m", "x"]);
    let err = Repo::discover(&base)
        .unwrap()
        .push()
        .unwrap_err()
        .to_string();
    assert!(err.contains("no remote"), "{err}");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn log_pages_commits_and_lists_the_files_of_a_commit() {
    let d = setup("log");
    for i in 0..5 {
        std::fs::write(d.work.join(format!("f{i}.txt")), "x\n").unwrap();
        git(&d.work, &["add", "-A"]);
        git(&d.work, &["commit", "-q", "-m", &format!("commit {i}")]);
    }
    let repo = Repo::discover(&d.work).unwrap();
    let first = repo.log(0, 3).unwrap();
    assert_eq!(first.len(), 3);
    assert_eq!(first[0].subject, "commit 4");
    assert!(first[0].refs.iter().any(|r| r.contains("main")));
    let rest = repo.log(3, 10).unwrap();
    assert_eq!(
        rest.len(),
        3,
        "5 new commits plus the first, minus the 3 already shown"
    );
    assert_eq!(rest.last().unwrap().subject, "first");

    let files = repo.commit_files(&first[0].hash).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!((files[0].letter, files[0].path.as_str()), ('A', "f4.txt"));
    let diff = repo.commit_diff(&first[0].hash, "f4.txt", None).unwrap();
    assert_eq!(diff.added(), 1);
    assert!(
        repo.commit_files("--output=evil").is_err(),
        "ids are validated"
    );
}

#[test]
fn log_of_an_empty_repository_is_empty() {
    let base = std::env::temp_dir().join(format!("nanti-git-emptylog-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    git(&base, &["init", "-q", "-b", "main"]);
    assert!(
        Repo::discover(&base)
            .unwrap()
            .log(0, 10)
            .unwrap()
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(&base);
}
