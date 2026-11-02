//! Extension manager (the Extensions view): lists what is installed, installs a local `.vsix`,
//! removes an extension, and remembers which one fills the right-hand panel at the next start.
//!
//! Installing means unpacking a package the user chose; nothing is downloaded and the vendor's
//! files are never copied anywhere except the user's own extensions folder. A package is
//! rejected unless it contains `extension/package.json` with a name and a publisher.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension {
    pub dir: PathBuf,
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub description: String,
    /// `engines.vscode` range the extension declares.
    pub engine: String,
    /// Webview view type it contributes (usable in the right-hand panel), if any.
    pub view_type: Option<String>,
    pub commands: usize,
}

/// Where extensions live: `<config dir>/extensions`.
pub fn extensions_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("extensions")
}

fn read_manifest(dir: &Path) -> Option<Extension> {
    let text = std::fs::read_to_string(dir.join("package.json")).ok()?;
    let j: serde_json::Value = serde_json::from_str(&text).ok()?;
    let name = j.get("name")?.as_str()?.to_owned();
    let publisher = j
        .get("publisher")
        .and_then(|p| p.as_str())
        .unwrap_or("unknown");
    let version = j
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("?")
        .to_owned();
    let display = j
        .get("displayName")
        .and_then(|v| v.as_str())
        .filter(|d| !d.starts_with('%')) // "%extension.name%" placeholders need the l10n file
        .unwrap_or(&name)
        .to_owned();
    let description = j
        .get("description")
        .and_then(|v| v.as_str())
        .filter(|d| !d.starts_with('%'))
        .unwrap_or("")
        .to_owned();
    let engine = j
        .pointer("/engines/vscode")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    let mut view_type = None;
    if let Some(views) = j.pointer("/contributes/views").and_then(|v| v.as_object()) {
        'outer: for list in views.values().filter_map(|l| l.as_array()) {
            for v in list {
                if v.get("type").and_then(|t| t.as_str()) == Some("webview")
                    && let Some(id) = v.get("id").and_then(|i| i.as_str())
                {
                    view_type = Some(id.to_owned());
                    break 'outer;
                }
            }
        }
    }
    let commands = j
        .pointer("/contributes/commands")
        .and_then(|c| c.as_array())
        .map_or(0, Vec::len);
    Some(Extension {
        dir: dir.to_path_buf(),
        id: format!("{publisher}.{name}"),
        display_name: display,
        version,
        description,
        engine,
        view_type,
        commands,
    })
}

/// Extensions in `<config>/extensions/*`, plus `extra` (the one passed with `NANTI_EXTENSION`).
pub fn list_installed(config_dir: &Path, extra: Option<&Path>) -> Vec<Extension> {
    let mut out: Vec<Extension> = Vec::new();
    if let Ok(read) = std::fs::read_dir(extensions_dir(config_dir)) {
        for e in read.filter_map(Result::ok) {
            if e.file_type().is_ok_and(|t| t.is_dir())
                && let Some(x) = read_manifest(&e.path())
            {
                out.push(x);
            }
        }
    }
    if let Some(dir) = extra
        && !out.iter().any(|x| x.dir == dir)
        && let Some(x) = read_manifest(dir)
    {
        out.push(x);
    }
    out.sort_by_key(|x| x.display_name.to_lowercase());
    out
}

/// Unpack a `.vsix` (a zip) into `<config>/extensions/<publisher.name-version>`.
/// Returns the installed extension. An existing folder of the same id/version is not touched.
pub fn install_vsix(config_dir: &Path, vsix: &Path) -> Result<Extension, String> {
    if !vsix.is_file() {
        return Err(format!("{} is not a file", vsix.display()));
    }
    if !vsix
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("vsix"))
    {
        return Err("choose a .vsix file".into());
    }
    let root = extensions_dir(config_dir);
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let staging = root.join(format!(".staging-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let result = (|| {
        // bsdtar (shipped with Windows 10+) reads zip files and refuses absolute and `..` entries.
        let mut cmd = std::process::Command::new("tar");
        cmd.arg("-xf").arg(vsix).arg("-C").arg(&staging);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        let out = cmd.output().map_err(|e| format!("cannot run tar: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "cannot unpack the package: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        let inner = staging.join("extension");
        let manifest =
            read_manifest(&inner).ok_or("the package has no valid extension/package.json")?;
        if manifest.id.starts_with("unknown.") {
            return Err("the package manifest has no publisher".into());
        }
        let safe_id: String = manifest
            .id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let dest = root.join(format!(
            "{safe_id}-{}",
            manifest.version.replace(['/', '\\', ':'], "_")
        ));
        if dest.exists() {
            return Err(format!(
                "{} {} is already installed",
                manifest.id, manifest.version
            ));
        }
        std::fs::rename(&inner, &dest).map_err(|e| e.to_string())?;
        read_manifest(&dest).ok_or_else(|| "the installed manifest cannot be read".to_owned())
    })();
    let _ = std::fs::remove_dir_all(&staging);
    result
}

/// Delete an installed extension folder. Only folders directly inside `<config>/extensions` qualify.
pub fn uninstall(config_dir: &Path, dir: &Path) -> Result<(), String> {
    let root = extensions_dir(config_dir);
    if dir.parent() != Some(root.as_path()) || !dir.is_dir() {
        return Err("only extensions installed by nanti. can be removed here".into());
    }
    std::fs::remove_dir_all(dir).map_err(|e| e.to_string())
}

/// Remember which extension/view fills the right-hand panel (read at the next start).
pub fn save_panel_choice(config_dir: &Path, dir: &Path, view_type: &str) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let json = serde_json::json!({ "dir": dir.to_string_lossy(), "view": view_type });
    std::fs::write(config_dir.join("panel-extension.json"), json.to_string())
        .map_err(|e| e.to_string())
}

pub fn load_panel_choice(config_dir: &Path) -> Option<(PathBuf, String)> {
    let text = std::fs::read_to_string(config_dir.join("panel-extension.json")).ok()?;
    let j: serde_json::Value = serde_json::from_str(&text).ok()?;
    let dir = PathBuf::from(j.get("dir")?.as_str()?);
    let view = j.get("view")?.as_str()?.to_owned();
    (dir.join("package.json").is_file() && !view.is_empty()).then_some((dir, view))
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
    fn tmp(tag: &str) -> Tmp {
        let p = std::env::temp_dir().join(format!("nanti-ext-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }

    fn write_manifest(dir: &Path, json: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("package.json"), json).unwrap();
    }

    /// Build a .vsix (zip) with bsdtar so the install path is exercised for real.
    fn make_vsix(work: &Path, name: &str, manifest: &str) -> PathBuf {
        let src = work.join(format!("src-{name}"));
        write_manifest(&src.join("extension"), manifest);
        std::fs::write(src.join("extension/readme.md"), "hi").unwrap();
        std::fs::write(src.join("[Content_Types].xml"), "<x/>").unwrap();
        let out = work.join(format!("{name}.vsix"));
        let st = std::process::Command::new("tar")
            .args(["-a", "-cf"])
            .arg(&out)
            .arg("-C")
            .arg(&src)
            .args(["extension", "[Content_Types].xml"])
            .status()
            .expect("tar");
        assert!(st.success());
        out
    }

    #[test]
    fn manifests_are_read_and_localization_placeholders_are_not_shown() {
        let t = tmp("manifest");
        write_manifest(
            &t.0.join("a"),
            r#"{"name":"claude-code","publisher":"acme","version":"1.2.3","displayName":"%ext.name%","description":"%d%",
               "engines":{"vscode":"^1.90.0"},
               "contributes":{"commands":[{"command":"x"},{"command":"y"}],
                              "views":{"explorer":[{"id":"tree"}],"panel":[{"id":"chat","type":"webview"}]}}}"#,
        );
        write_manifest(&t.0.join("broken"), "{oops");
        let list = list_installed(&t.0.join("none"), Some(&t.0.join("a")));
        assert_eq!(list.len(), 1);
        let x = &list[0];
        assert_eq!(
            (x.id.as_str(), x.version.as_str(), x.commands),
            ("acme.claude-code", "1.2.3", 2)
        );
        assert_eq!(
            x.display_name, "claude-code",
            "a %placeholder% falls back to the name"
        );
        assert_eq!(x.view_type.as_deref(), Some("chat"));
        assert_eq!(x.engine, "^1.90.0");
        assert!(list_installed(&t.0, Some(&t.0.join("broken"))).is_empty());
    }

    #[test]
    fn install_unpacks_validates_and_refuses_duplicates() {
        let t = tmp("install");
        let cfg = t.0.join("cfg");
        let vsix = make_vsix(
            &t.0,
            "good",
            r#"{"name":"tool","publisher":"me","version":"0.1.0"}"#,
        );
        let x = install_vsix(&cfg, &vsix).unwrap();
        assert_eq!(x.id, "me.tool");
        assert!(x.dir.starts_with(extensions_dir(&cfg)) && x.dir.join("readme.md").is_file());
        assert!(
            !extensions_dir(&cfg)
                .join(format!(".staging-{}", std::process::id()))
                .exists(),
            "staging is cleaned up"
        );
        assert!(
            install_vsix(&cfg, &vsix)
                .unwrap_err()
                .contains("already installed")
        );
        assert_eq!(list_installed(&cfg, None).len(), 1);
    }

    #[test]
    fn install_rejects_bad_packages_without_leaving_anything_behind() {
        let t = tmp("badinstall");
        let cfg = t.0.join("cfg");
        assert!(install_vsix(&cfg, &t.0.join("missing.vsix")).is_err());
        std::fs::write(t.0.join("x.txt"), "x").unwrap();
        assert!(
            install_vsix(&cfg, &t.0.join("x.txt"))
                .unwrap_err()
                .contains(".vsix")
        );
        std::fs::write(t.0.join("fake.vsix"), "not a zip").unwrap();
        assert!(install_vsix(&cfg, &t.0.join("fake.vsix")).is_err());
        let nopub = make_vsix(&t.0, "nopub", r#"{"name":"tool","version":"1.0.0"}"#);
        assert!(
            install_vsix(&cfg, &nopub)
                .unwrap_err()
                .contains("publisher")
        );
        assert!(list_installed(&cfg, None).is_empty());
        let leftovers: Vec<_> = std::fs::read_dir(extensions_dir(&cfg))
            .map(|r| r.filter_map(Result::ok).collect())
            .unwrap_or_default();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn uninstall_only_touches_folders_inside_the_extensions_directory() {
        let t = tmp("uninstall");
        let cfg = t.0.join("cfg");
        let vsix = make_vsix(
            &t.0,
            "u",
            r#"{"name":"t","publisher":"p","version":"1.0.0"}"#,
        );
        let x = install_vsix(&cfg, &vsix).unwrap();
        assert!(uninstall(&cfg, &t.0).is_err());
        assert!(uninstall(&cfg, &x.dir.join("..")).is_err());
        uninstall(&cfg, &x.dir).unwrap();
        assert!(!x.dir.exists());
    }

    #[test]
    fn panel_choice_round_trips_and_ignores_stale_entries() {
        let t = tmp("choice");
        let cfg = t.0.join("cfg");
        let ext = t.0.join("ext");
        write_manifest(&ext, r#"{"name":"n","publisher":"p"}"#);
        assert!(load_panel_choice(&cfg).is_none());
        save_panel_choice(&cfg, &ext, "chat").unwrap();
        assert_eq!(
            load_panel_choice(&cfg),
            Some((ext.clone(), "chat".to_owned()))
        );
        std::fs::remove_dir_all(&ext).unwrap();
        assert!(
            load_panel_choice(&cfg).is_none(),
            "an extension that was removed is not loaded"
        );
    }
}
