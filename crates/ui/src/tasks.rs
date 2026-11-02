//! Run tasks detected from the project files (the Run view). Nothing runs by itself: a task is a
//! command line the user clicks, and it runs in the integrated terminal where its output and any
//! prompt are visible.

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub label: String,
    /// Exact text typed into the terminal.
    pub command: String,
    /// Which project file it came from.
    pub source: &'static str,
}

fn task(label: &str, command: &str, source: &'static str) -> Task {
    Task {
        label: label.to_owned(),
        command: command.to_owned(),
        source,
    }
}

/// Tasks for the project at `root` (Cargo, npm/pnpm/yarn, Go, Dart/Flutter).
pub fn detect(root: &Path) -> Vec<Task> {
    let mut out = Vec::new();
    if root.join("Cargo.toml").is_file() {
        for (l, c) in [
            ("cargo run", "cargo run"),
            ("cargo build", "cargo build"),
            ("cargo test", "cargo test"),
            ("cargo check", "cargo check"),
            ("cargo clippy", "cargo clippy --all-targets"),
            ("cargo fmt (check)", "cargo fmt --all -- --check"),
        ] {
            out.push(task(l, c, "Cargo.toml"));
        }
    }
    if let Ok(text) = std::fs::read_to_string(root.join("package.json"))
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(scripts) = json.get("scripts").and_then(|s| s.as_object())
    {
        let runner = if root.join("pnpm-lock.yaml").is_file() {
            "pnpm"
        } else if root.join("yarn.lock").is_file() {
            "yarn"
        } else {
            "npm"
        };
        let mut names: Vec<&String> = scripts.keys().collect();
        names.sort();
        for n in names {
            // A script name goes into a shell command line: only plain names are offered.
            if n.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.'))
            {
                out.push(task(
                    &format!("{runner} run {n}"),
                    &format!("{runner} run {n}"),
                    "package.json",
                ));
            }
        }
    }
    if root.join("go.mod").is_file() {
        for (l, c) in [
            ("go run .", "go run ."),
            ("go build", "go build ./..."),
            ("go test", "go test ./..."),
            ("go vet", "go vet ./..."),
        ] {
            out.push(task(l, c, "go.mod"));
        }
    }
    if root.join("pubspec.yaml").is_file() {
        let flutter = std::fs::read_to_string(root.join("pubspec.yaml"))
            .is_ok_and(|t| t.contains("flutter:"));
        let tool = if flutter { "flutter" } else { "dart" };
        for (l, c) in [
            (format!("{tool} run"), format!("{tool} run")),
            (format!("{tool} test"), format!("{tool} test")),
            (
                format!("{tool} analyze"),
                if flutter {
                    "flutter analyze".to_owned()
                } else {
                    "dart analyze".to_owned()
                },
            ),
            (format!("{tool} pub get"), format!("{tool} pub get")),
        ] {
            out.push(Task {
                label: l,
                command: c,
                source: "pubspec.yaml",
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct Tmp(PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn ws(tag: &str) -> Tmp {
        let p = std::env::temp_dir().join(format!("nanti-tasks-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }

    #[test]
    fn an_empty_folder_has_no_tasks() {
        assert!(detect(&ws("empty").0).is_empty());
    }

    #[test]
    fn cargo_go_and_dart_projects_get_their_standard_tasks() {
        let t = ws("multi");
        std::fs::write(t.0.join("Cargo.toml"), "[package]").unwrap();
        std::fs::write(t.0.join("go.mod"), "module x").unwrap();
        std::fs::write(
            t.0.join("pubspec.yaml"),
            "name: a\nflutter:\n  uses-material-design: true\n",
        )
        .unwrap();
        let tasks = detect(&t.0);
        let cmds: Vec<&str> = tasks.iter().map(|t| t.command.as_str()).collect();
        assert!(
            cmds.contains(&"cargo test")
                && cmds.contains(&"go test ./...")
                && cmds.contains(&"flutter run")
        );
        assert!(!cmds.contains(&"dart run"));
    }

    #[test]
    fn npm_scripts_use_the_right_runner_and_skip_unsafe_names() {
        let t = ws("npm");
        std::fs::write(
            t.0.join("package.json"),
            r#"{"scripts": {"test": "x", "build:prod": "y", "evil; calc": "z", "$(whoami)": "w"}}"#,
        )
        .unwrap();
        let cmds: Vec<String> = detect(&t.0).into_iter().map(|t| t.command).collect();
        assert_eq!(cmds, ["npm run build:prod", "npm run test"]);
        std::fs::write(t.0.join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(detect(&t.0)[0].command, "pnpm run build:prod");
    }

    #[test]
    fn broken_package_json_is_ignored() {
        let t = ws("broken");
        std::fs::write(t.0.join("package.json"), "{not json").unwrap();
        assert!(detect(&t.0).is_empty());
    }
}
