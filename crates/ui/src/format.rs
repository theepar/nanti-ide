use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(4);
const SETTINGS_FILE: &str = "format.json";
const PRETTIER_FILES: [&str; 19] = [
    "js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts", "json", "jsonc", "css", "scss", "less",
    "html", "vue", "md", "yaml", "yml", "graphql",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Stdin,
    // dart format only knows files, so the text goes to a temporary sibling that sees the same analysis_options.yaml.
    TempSibling,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub name: &'static str,
    pub program: OsString,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
    pub input: Input,
}

pub fn load_format_on_save(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join(SETTINGS_FILE))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')).ok())
        .and_then(|v| v.get("formatOnSave").and_then(Value::as_bool))
        .unwrap_or(true)
}

/// Which Prettier formats the web file types.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum PrettierSource {
    /// The project's own, then an installed extension's, then the one on PATH.
    #[default]
    Auto,
    /// The Prettier inside one installed extension, by the name of its folder.
    Extension(String),
    Path,
}

impl PrettierSource {
    fn key(&self) -> String {
        match self {
            Self::Auto => "auto".to_owned(),
            Self::Extension(folder) => format!("ext:{folder}"),
            Self::Path => "path".to_owned(),
        }
    }

    fn from_key(key: &str) -> Self {
        match key {
            "path" => Self::Path,
            k if k.starts_with("ext:") && k.len() > 4 => Self::Extension(k[4..].to_owned()),
            _ => Self::Auto,
        }
    }
}

fn update_settings(dir: &Path, key: &str, value: Value) -> Result<(), String> {
    let mut object = std::fs::read_to_string(dir.join(SETTINGS_FILE))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    object[key] = value;
    let text = serde_json::to_string_pretty(&object).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(SETTINGS_FILE), text).map_err(|e| e.to_string())
}

pub fn save_format_on_save(dir: &Path, on: bool) -> Result<(), String> {
    update_settings(dir, "formatOnSave", json!(on))
}

pub fn load_prettier_source(dir: &Path) -> PrettierSource {
    std::fs::read_to_string(dir.join(SETTINGS_FILE))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')).ok())
        .and_then(|v| {
            v.get("prettier")
                .and_then(Value::as_str)
                .map(PrettierSource::from_key)
        })
        .unwrap_or_default()
}

pub fn save_prettier_source(dir: &Path, source: &PrettierSource) -> Result<(), String> {
    update_settings(dir, "prettier", json!(source.key()))
}

pub fn prettier_on_path() -> bool {
    on_path(&["prettier.cmd", "prettier.exe", "prettier"]).is_some()
}

/// Picks the formatter for a file: the project's own tool first, then what the installed extensions bring, then PATH.
pub fn plan(path: &Path, extension_dirs: &[PathBuf]) -> Option<Plan> {
    plan_with(path, extension_dirs, &PrettierSource::Auto)
}

pub fn plan_with(path: &Path, extension_dirs: &[PathBuf], source: &PrettierSource) -> Option<Plan> {
    let cwd = path.parent()?.to_path_buf();
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "rs" => Some(Plan {
            name: "rustfmt",
            program: "rustfmt".into(),
            args: vec!["--edition".into(), rust_edition(&cwd).into()],
            cwd,
            input: Input::Stdin,
        }),
        "go" => Some(Plan {
            name: "gofmt",
            program: "gofmt".into(),
            args: Vec::new(),
            cwd,
            input: Input::Stdin,
        }),
        "dart" => Some(Plan {
            name: "dart format",
            program: "dart".into(),
            args: vec![
                "format".into(),
                "--output=show".into(),
                "--summary=none".into(),
            ],
            cwd,
            input: Input::TempSibling,
        }),
        e if PRETTIER_FILES.contains(&e) => {
            let (program, mut args) = find_prettier(&cwd, extension_dirs, source)?;
            args.push("--stdin-filepath".into());
            args.push(path.as_os_str().to_owned());
            Some(Plan {
                name: "Prettier",
                program,
                args,
                cwd,
                input: Input::Stdin,
            })
        }
        _ => None,
    }
}

fn rust_edition(dir: &Path) -> String {
    for d in dir.ancestors() {
        let Ok(text) = std::fs::read_to_string(d.join("Cargo.toml")) else {
            continue;
        };
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("edition")
                && let Some(value) = rest.trim_start().strip_prefix('=')
            {
                let year = value.trim().trim_matches('"');
                if year.len() == 4 && year.bytes().all(|b| b.is_ascii_digit()) {
                    return year.to_owned();
                }
            }
        }
    }
    "2021".to_owned()
}

fn find_prettier(
    dir: &Path,
    extension_dirs: &[PathBuf],
    source: &PrettierSource,
) -> Option<(OsString, Vec<OsString>)> {
    let script = |root: &Path| {
        ["bin/prettier.cjs", "bin-prettier.js"]
            .iter()
            .map(|rel| root.join("node_modules").join("prettier").join(rel))
            .find(|p| p.is_file())
    };
    let via_node = |p: PathBuf| {
        (
            OsString::from(crate::runtime::node()),
            vec![p.into_os_string()],
        )
    };
    // A chosen source that is gone (extension removed, nothing on PATH) falls back to automatic, so formatting keeps working.
    match source {
        PrettierSource::Extension(folder) => {
            if let Some(found) = extension_dirs
                .iter()
                .find(|e| {
                    e.file_name()
                        .is_some_and(|n| n.to_string_lossy() == folder.as_str())
                })
                .and_then(|e| script(e))
            {
                return Some(via_node(found));
            }
        }
        PrettierSource::Path => {
            if let Some(p) = on_path(&["prettier.cmd", "prettier.exe", "prettier"]) {
                return Some((p.into_os_string(), Vec::new()));
            }
        }
        PrettierSource::Auto => {}
    }
    if let Some(found) = dir.ancestors().find_map(script) {
        return Some(via_node(found));
    }
    for ext in extension_dirs {
        let name = ext
            .file_name()
            .map(|n| n.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if name.starts_with("esbenp.prettier-vscode")
            && let Some(found) = script(ext)
        {
            return Some(via_node(found));
        }
    }
    on_path(&["prettier.cmd", "prettier.exe", "prettier"]).map(|p| (p.into_os_string(), Vec::new()))
}

fn on_path(names: &[&str]) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|d| names.iter().map(|n| d.join(n)).find(|p| p.is_file()))
}

pub fn run(plan: &Plan, path: &Path, text: &str) -> Result<String, String> {
    let out = match plan.input {
        Input::Stdin => exec(plan, &plan.args, Some(text))?,
        Input::TempSibling => {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");
            let tmp = plan
                .cwd
                .join(format!(".nanti-format-{}.{ext}", std::process::id()));
            std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
            let mut args = plan.args.clone();
            args.push(tmp.clone().into_os_string());
            let out = exec(plan, &args, None);
            let _ = std::fs::remove_file(&tmp);
            out?
        }
    };
    if out.trim().is_empty() && !text.trim().is_empty() {
        return Err(format!("{} returned nothing", plan.name));
    }
    Ok(out)
}

fn exec(plan: &Plan, args: &[OsString], stdin: Option<&str>) -> Result<String, String> {
    let mut cmd = Command::new(&plan.program);
    cmd.args(args)
        .current_dir(&plan.cwd)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x0800_0000);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("{} is not available ({e})", plan.name))?;
    let writer = stdin.zip(child.stdin.take()).map(|(text, mut pipe)| {
        let text = text.to_owned();
        std::thread::spawn(move || {
            let _ = pipe.write_all(text.as_bytes());
        })
    });
    let read = |mut pipe: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            buf
        })
    };
    let stdout = child.stdout.take().map(|p| read(Box::new(p)));
    let stderr = child.stderr.take().map(|p| read(Box::new(p)));
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if start.elapsed() > TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{} took longer than {}s",
                    plan.name,
                    TIMEOUT.as_secs()
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => return Err(e.to_string()),
        }
    };
    if let Some(w) = writer {
        let _ = w.join();
    }
    let out = stdout
        .map(|h| h.join().unwrap_or_default())
        .unwrap_or_default();
    let err = stderr
        .map(|h| h.join().unwrap_or_default())
        .unwrap_or_default();
    if !status.success() {
        let msg = String::from_utf8_lossy(&err);
        let first = msg
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("failed");
        return Err(format!("{}: {}", plan.name, first.trim()));
    }
    Ok(String::from_utf8_lossy(&out).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nanti-fmt-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn have(program: &str) -> bool {
        Command::new(program)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    #[test]
    fn rust_files_use_the_edition_of_the_nearest_cargo_manifest() {
        let d = tmp("edition");
        std::fs::create_dir_all(d.join("crates/a/src")).unwrap();
        std::fs::write(
            d.join("Cargo.toml"),
            "[workspace.package]\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::write(
            d.join("crates/a/Cargo.toml"),
            "[package]\nedition.workspace = true\n",
        )
        .unwrap();
        let p = plan(&d.join("crates/a/src/lib.rs"), &[]).unwrap();
        assert_eq!(p.name, "rustfmt");
        assert_eq!(p.args, ["--edition", "2024"]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_project_prettier_wins_over_the_extension_and_the_extension_over_nothing() {
        let d = tmp("prettier");
        let ext = d.join("extensions/esbenp.prettier-vscode-12.0.0");
        std::fs::create_dir_all(ext.join("node_modules/prettier/bin")).unwrap();
        std::fs::write(ext.join("node_modules/prettier/bin/prettier.cjs"), "").unwrap();
        let file = d.join("app/src/a.ts");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();

        let with_ext = plan(&file, std::slice::from_ref(&ext)).unwrap();
        assert_eq!(with_ext.name, "Prettier");
        assert_eq!(with_ext.program, "node");
        assert_eq!(
            PathBuf::from(&with_ext.args[0]),
            ext.join("node_modules/prettier/bin/prettier.cjs")
        );
        assert_eq!(with_ext.args[1], "--stdin-filepath");

        std::fs::create_dir_all(d.join("app/node_modules/prettier/bin")).unwrap();
        std::fs::write(d.join("app/node_modules/prettier/bin/prettier.cjs"), "").unwrap();
        let local = plan(&file, &[ext]).unwrap();
        assert_eq!(
            PathBuf::from(&local.args[0]),
            d.join("app/node_modules/prettier/bin/prettier.cjs")
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_chosen_prettier_source_wins_and_a_missing_one_falls_back() {
        let d = tmp("source");
        let one = d.join("extensions/esbenp.prettier-vscode-1.0.0");
        let two = d.join("extensions/esbenp.prettier-vscode-2.0.0");
        for e in [&one, &two] {
            std::fs::create_dir_all(e.join("node_modules/prettier/bin")).unwrap();
            std::fs::write(e.join("node_modules/prettier/bin/prettier.cjs"), "").unwrap();
        }
        let file = d.join("app/a.ts");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let dirs = [one.clone(), two.clone()];
        let script = |p: &Plan| PathBuf::from(&p.args[0]);

        let auto = plan_with(&file, &dirs, &PrettierSource::Auto).unwrap();
        assert_eq!(
            script(&auto),
            one.join("node_modules/prettier/bin/prettier.cjs")
        );
        let chosen = plan_with(
            &file,
            &dirs,
            &PrettierSource::Extension("esbenp.prettier-vscode-2.0.0".into()),
        )
        .unwrap();
        assert_eq!(
            script(&chosen),
            two.join("node_modules/prettier/bin/prettier.cjs")
        );
        let gone = plan_with(&file, &dirs, &PrettierSource::Extension("removed-9".into())).unwrap();
        assert_eq!(
            script(&gone),
            one.join("node_modules/prettier/bin/prettier.cjs")
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_prettier_choice_is_saved_next_to_format_on_save() {
        let d = tmp("sourcefile");
        assert_eq!(load_prettier_source(&d), PrettierSource::Auto);
        save_prettier_source(&d, &PrettierSource::Extension("x-1".into())).unwrap();
        save_format_on_save(&d, false).unwrap();
        assert_eq!(
            load_prettier_source(&d),
            PrettierSource::Extension("x-1".into())
        );
        assert!(
            !load_format_on_save(&d),
            "saving one setting keeps the other"
        );
        save_prettier_source(&d, &PrettierSource::Path).unwrap();
        assert_eq!(load_prettier_source(&d), PrettierSource::Path);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn unknown_file_types_have_no_formatter() {
        assert!(plan(Path::new("C:/x/readme.txt"), &[]).is_none());
        assert!(plan(Path::new("C:/x/Makefile"), &[]).is_none());
    }

    #[test]
    fn rustfmt_formats_text_from_stdin() {
        if !have("rustfmt") {
            return;
        }
        let d = tmp("rustfmt");
        let file = d.join("a.rs");
        let p = plan(&file, &[]).unwrap();
        let out = run(&p, &file, "fn main(){let x=1;}\n").unwrap();
        assert_eq!(out, "fn main() {\n    let x = 1;\n}\n");
        let err = run(&p, &file, "fn main( {").unwrap_err();
        assert!(err.starts_with("rustfmt:"), "{err}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn gofmt_formats_text_from_stdin() {
        if !have("gofmt") {
            return;
        }
        let d = tmp("gofmt");
        let file = d.join("a.go");
        let p = plan(&file, &[]).unwrap();
        let out = run(&p, &file, "package main\nfunc main(){\nx:=1\n_=x}\n").unwrap();
        assert_eq!(out, "package main\n\nfunc main() {\n\tx := 1\n\t_ = x\n}\n");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_missing_tool_is_reported_not_panicked() {
        let p = Plan {
            name: "nope",
            program: "nanti-no-such-formatter".into(),
            args: Vec::new(),
            cwd: std::env::temp_dir(),
            input: Input::Stdin,
        };
        let err = run(&p, Path::new("a.rs"), "x").unwrap_err();
        assert!(err.contains("not available"), "{err}");
    }

    #[test]
    fn format_on_save_defaults_to_on_and_remembers_the_choice() {
        let d = tmp("settings");
        assert!(load_format_on_save(&d));
        save_format_on_save(&d, false).unwrap();
        assert!(!load_format_on_save(&d));
        let _ = std::fs::remove_dir_all(&d);
    }
}
