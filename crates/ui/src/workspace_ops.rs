//! File operations behind the explorer (create, rename, delete to the Recycle Bin). GUI-free.
//!
//! Safety rules: every new path must stay inside the workspace, names are validated for Windows,
//! nothing existing is ever overwritten, and delete goes to the Recycle Bin (recoverable), never
//! a permanent remove.

use std::path::{Component, Path, PathBuf};

const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Check one path component name.
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." {
        return Err("enter a name".into());
    }
    if name.chars().count() > 200 {
        return Err("the name is too long".into());
    }
    if let Some(c) = name
        .chars()
        .find(|c| c.is_control() || "<>:\"|?*\\/".contains(*c))
    {
        return Err(format!("the name cannot contain {c:?}"));
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Err("the name cannot end with a dot or a space".into());
    }
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        return Err(format!("{name:?} is a reserved Windows name"));
    }
    Ok(())
}

/// Resolve `input` (a name or a relative path like `src/new/mod.rs`) below `dir`.
/// Fails for empty, absolute or `..` paths, bad names, anything leaving `root`, or an existing target.
pub fn resolve_new(root: &Path, dir: &Path, input: &str) -> Result<PathBuf, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("enter a name".into());
    }
    let rel = Path::new(input);
    if rel.is_absolute() || input.starts_with(['/', '\\']) || input.contains(':') {
        return Err("use a name relative to the selected folder".into());
    }
    let mut target = dir.to_path_buf();
    for comp in rel.components() {
        match comp {
            Component::Normal(n) => {
                let n = n.to_string_lossy();
                validate_name(&n)?;
                target.push(&*n);
            }
            Component::CurDir => {}
            _ => return Err("paths with `..` are not allowed".into()),
        }
    }
    if target == dir {
        return Err("enter a name".into());
    }
    if !target.starts_with(root) {
        return Err("the new item must stay inside the workspace".into());
    }
    if target.exists() {
        return Err(format!(
            "{} already exists",
            target
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
        ));
    }
    Ok(target)
}

pub fn create_file(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // create_new: fails instead of truncating if somebody created it in the meantime
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn create_folder(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())
}

/// Rename `from` to `new_name` in the same folder. Returns the new path.
pub fn rename(root: &Path, from: &Path, new_name: &str) -> Result<PathBuf, String> {
    let new_name = new_name.trim();
    validate_name(new_name)?;
    if from == root {
        return Err("the workspace folder cannot be renamed here".into());
    }
    let to = from.with_file_name(new_name);
    if to == from {
        return Err("the name did not change".into());
    }
    // a case-only change (a.txt -> A.txt) is the same file on Windows and must be allowed
    let case_only = to
        .to_string_lossy()
        .eq_ignore_ascii_case(&from.to_string_lossy());
    if to.exists() && !case_only {
        return Err(format!("{new_name} already exists"));
    }
    std::fs::rename(from, &to).map_err(|e| e.to_string())?;
    Ok(to)
}

/// Move `path` to the Recycle Bin (recoverable). The workspace root itself is refused.
pub fn send_to_recycle_bin(root: &Path, path: &Path) -> Result<(), String> {
    if path == root || !path.starts_with(root) {
        return Err("only items inside the workspace can be deleted".into());
    }
    if !path.exists() {
        return Err("the item no longer exists".into());
    }
    recycle(path)
}

#[cfg(windows)]
fn recycle(path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let method = if path.is_dir() {
        "DeleteDirectory"
    } else {
        "DeleteFile"
    };
    // The path is passed through an environment variable, never spliced into the script.
    let script = format!(
        "Add-Type -AssemblyName Microsoft.VisualBasic; \
         [Microsoft.VisualBasic.FileIO.FileSystem]::{method}($env:NANTI_TRASH_PATH, \
         'OnlyErrorDialogs', 'SendToRecycleBin')"
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .env("NANTI_TRASH_PATH", path)
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|e| format!("cannot start PowerShell: {e}"))?;
    if out.status.success() && !path.exists() {
        Ok(())
    } else {
        Err(format!(
            "could not move to the Recycle Bin: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

#[cfg(not(windows))]
fn recycle(_path: &Path) -> Result<(), String> {
    Err("moving to the trash is only implemented on Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tmp(PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn ws(tag: &str) -> Tmp {
        let p = std::env::temp_dir().join(format!("nanti-ops-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }

    #[test]
    fn names_are_validated_for_windows() {
        for bad in [
            "",
            ".",
            "..",
            "a:b",
            "a|b",
            "a?b",
            "a*b",
            "a\"b",
            "a<b",
            "trail.",
            "trail ",
            "con",
            "NUL.txt",
            "lpt1",
            "tab\there",
        ] {
            assert!(validate_name(bad).is_err(), "{bad:?} must be rejected");
        }
        for good in [
            "a.txt",
            ".gitignore",
            "日本語.rs",
            "with space.md",
            "console.log",
            "COM10",
            "x",
        ] {
            assert!(validate_name(good).is_ok(), "{good:?} must be accepted");
        }
        assert!(validate_name(&"x".repeat(201)).is_err());
    }

    #[test]
    fn resolve_new_stays_inside_the_workspace_and_never_overwrites() {
        let t = ws("resolve");
        let root = &t.0;
        std::fs::write(root.join("exists.txt"), "x").unwrap();
        assert_eq!(
            resolve_new(root, root, "new.txt").unwrap(),
            root.join("new.txt")
        );
        assert_eq!(
            resolve_new(root, root, "a/b/c.rs").unwrap(),
            root.join("a").join("b").join("c.rs")
        );
        assert_eq!(
            resolve_new(root, root, "a\\b.rs").unwrap(),
            root.join("a").join("b.rs")
        );
        assert_eq!(
            resolve_new(root, root, "  padded.txt  ").unwrap(),
            root.join("padded.txt")
        );
        for bad in [
            "",
            "  ",
            "../escape.txt",
            "a/../../b",
            "/abs.txt",
            "\\abs.txt",
            "C:\\x.txt",
            "exists.txt",
            "a/CON/b",
            "./",
            ".",
        ] {
            assert!(
                resolve_new(root, root, bad).is_err(),
                "{bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn create_file_and_folder_make_parents_and_refuse_to_truncate() {
        let t = ws("create");
        let f = t.0.join("deep/er/file.txt");
        create_file(&f).unwrap();
        assert!(f.is_file());
        std::fs::write(&f, "keep me").unwrap();
        assert!(
            create_file(&f).is_err(),
            "an existing file is never truncated"
        );
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "keep me");
        create_folder(&t.0.join("x/y")).unwrap();
        assert!(t.0.join("x/y").is_dir());
    }

    #[test]
    fn rename_keeps_the_folder_refuses_collisions_and_allows_case_changes() {
        let t = ws("rename");
        let root = &t.0;
        std::fs::write(root.join("a.txt"), "a").unwrap();
        std::fs::write(root.join("b.txt"), "b").unwrap();
        assert_eq!(
            rename(root, &root.join("a.txt"), "c.txt").unwrap(),
            root.join("c.txt")
        );
        assert!(root.join("c.txt").exists() && !root.join("a.txt").exists());
        assert!(
            rename(root, &root.join("c.txt"), "b.txt").is_err(),
            "no overwrite"
        );
        assert_eq!(std::fs::read_to_string(root.join("b.txt")).unwrap(), "b");
        assert!(
            rename(root, &root.join("c.txt"), "sub/x.txt").is_err(),
            "a name, not a path"
        );
        assert!(rename(root, &root.join("c.txt"), "c.txt").is_err());
        assert!(
            rename(root, root, "other").is_err(),
            "the workspace root cannot be renamed"
        );
        let up = rename(root, &root.join("c.txt"), "C.txt").unwrap();
        assert_eq!(up.file_name().unwrap(), "C.txt");
    }

    #[test]
    fn recycle_bin_removes_files_and_folders_but_refuses_the_root_and_outsiders() {
        let t = ws("trash");
        let root = &t.0;
        std::fs::write(root.join("gone.txt"), "x").unwrap();
        std::fs::create_dir_all(root.join("dir/inner")).unwrap();
        std::fs::write(root.join("dir/inner/f.txt"), "y").unwrap();
        assert!(send_to_recycle_bin(root, root).is_err());
        assert!(
            send_to_recycle_bin(root, &std::env::temp_dir().join("outside-nanti.txt")).is_err()
        );
        assert!(send_to_recycle_bin(root, &root.join("missing.txt")).is_err());
        if cfg!(windows) {
            send_to_recycle_bin(root, &root.join("gone.txt")).unwrap();
            send_to_recycle_bin(root, &root.join("dir")).unwrap();
            assert!(!root.join("gone.txt").exists() && !root.join("dir").exists());
        }
    }
}
