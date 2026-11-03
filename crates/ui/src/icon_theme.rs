use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::extensions::Extension;

const SETTINGS_FILE: &str = "icon-theme.json";

/// A file icon theme that an installed extension contributes (`contributes.iconThemes`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeEntry {
    // "<extension folder>/<theme id>": stable across restarts and unique across extensions.
    pub key: String,
    pub label: String,
    pub json: PathBuf,
}

pub fn discover(extensions: &[Extension]) -> Vec<ThemeEntry> {
    let mut out = Vec::new();
    for ext in extensions {
        let Ok(text) = std::fs::read_to_string(ext.dir.join("package.json")) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}'))
        else {
            continue;
        };
        let Some(themes) = manifest["contributes"]["iconThemes"].as_array() else {
            continue;
        };
        let folder = ext
            .dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        for t in themes {
            let (Some(id), Some(path)) = (t["id"].as_str(), t["path"].as_str()) else {
                continue;
            };
            out.push(ThemeEntry {
                key: format!("{folder}/{id}"),
                label: t["label"]
                    .as_str()
                    .map_or_else(|| id.to_owned(), str::to_owned),
                json: ext.dir.join(path.trim_start_matches("./")),
            });
        }
    }
    out
}

#[derive(Debug, Default)]
pub struct IconTheme {
    defs: HashMap<String, PathBuf>,
    file: Option<String>,
    folder: Option<String>,
    folder_expanded: Option<String>,
    root_folder: Option<String>,
    root_folder_expanded: Option<String>,
    file_names: HashMap<String, String>,
    file_extensions: HashMap<String, String>,
    folder_names: HashMap<String, String>,
    folder_names_expanded: HashMap<String, String>,
    root_folder_names: HashMap<String, String>,
    root_folder_names_expanded: HashMap<String, String>,
}

fn map_of(v: &Value, key: &str) -> HashMap<String, String> {
    v[key]
        .as_object()
        .map(|o| {
            o.iter()
                .filter_map(|(k, v)| Some((k.to_lowercase(), v.as_str()?.to_owned())))
                .collect()
        })
        .unwrap_or_default()
}

fn text_of(v: &Value, key: &str) -> Option<String> {
    v[key].as_str().map(str::to_owned)
}

impl IconTheme {
    pub fn load(json: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(json).map_err(|e| e.to_string())?;
        let v: Value =
            serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())?;
        let base = json.parent().unwrap_or(Path::new("."));
        let defs = v["iconDefinitions"]
            .as_object()
            .map(|o| {
                o.iter()
                    .filter_map(|(id, d)| {
                        let rel = d["iconPath"].as_str()?;
                        // Only vector and bitmap files are drawn; font glyph themes are not supported.
                        let ok = Path::new(rel).extension().is_some_and(|e| {
                            ["svg", "png"].iter().any(|x| e.eq_ignore_ascii_case(x))
                        });
                        ok.then(|| (id.clone(), base.join(rel)))
                    })
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();
        if defs.is_empty() {
            return Err("the theme has no svg or png icons".into());
        }
        Ok(Self {
            defs,
            file: text_of(&v, "file"),
            folder: text_of(&v, "folder"),
            folder_expanded: text_of(&v, "folderExpanded"),
            root_folder: text_of(&v, "rootFolder"),
            root_folder_expanded: text_of(&v, "rootFolderExpanded"),
            file_names: map_of(&v, "fileNames"),
            file_extensions: map_of(&v, "fileExtensions"),
            folder_names: map_of(&v, "folderNames"),
            folder_names_expanded: map_of(&v, "folderNamesExpanded"),
            root_folder_names: map_of(&v, "rootFolderNames"),
            root_folder_names_expanded: map_of(&v, "rootFolderNamesExpanded"),
        })
    }

    fn path(&self, id: Option<&String>) -> Option<PathBuf> {
        self.defs.get(id?).cloned()
    }

    /// The icon of a file: its exact name, then its longest known extension, then the theme's plain file icon.
    pub fn file_icon(&self, name: &str) -> Option<PathBuf> {
        let lower = name.to_lowercase();
        if let Some(p) = self.path(self.file_names.get(&lower)) {
            return Some(p);
        }
        for (i, _) in lower.match_indices('.') {
            if let Some(p) = self.path(self.file_extensions.get(&lower[i + 1..])) {
                return Some(p);
            }
        }
        self.path(self.file.as_ref())
    }

    pub fn folder_icon(&self, name: &str, expanded: bool, root: bool) -> Option<PathBuf> {
        let lower = name.to_lowercase();
        let (named, plain) = match (root, expanded) {
            (true, false) => (&self.root_folder_names, &self.root_folder),
            (true, true) => (&self.root_folder_names_expanded, &self.root_folder_expanded),
            (false, false) => (&self.folder_names, &self.folder),
            (false, true) => (&self.folder_names_expanded, &self.folder_expanded),
        };
        if let Some(p) = self.path(named.get(&lower)) {
            return Some(p);
        }
        // A root without its own icon falls back to the ordinary folder icons.
        let fallback = if expanded {
            &self.folder_expanded
        } else {
            &self.folder
        };
        self.path(plain.as_ref())
            .or_else(|| self.path(fallback.as_ref()))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Choice {
    pub active: Option<String>,
    // Every theme already offered once, so a theme is only switched on automatically the first time it shows up.
    pub seen: Vec<String>,
}

pub fn load_choice(dir: &Path) -> Choice {
    load_choice_in(dir, SETTINGS_FILE)
}

pub fn save_choice(dir: &Path, choice: &Choice) -> Result<(), String> {
    save_choice_in(dir, SETTINGS_FILE, choice)
}

pub fn load_choice_in(dir: &Path, file: &str) -> Choice {
    let Some(v) = std::fs::read_to_string(dir.join(file))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')).ok())
    else {
        return Choice::default();
    };
    Choice {
        active: v["active"].as_str().map(str::to_owned),
        seen: v["seen"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

pub fn save_choice_in(dir: &Path, file: &str, choice: &Choice) -> Result<(), String> {
    let text = serde_json::to_string_pretty(&json!({
        "active": choice.active,
        "seen": choice.seen,
    }))
    .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(file), text).map_err(|e| e.to_string())
}

/// Switches a theme on the first time it appears. Returns the label of the theme that was switched on.
pub fn adopt_new(choice: &mut Choice, available: &[ThemeEntry]) -> Option<String> {
    let mut adopted = None;
    for t in available {
        if choice.seen.contains(&t.key) {
            continue;
        }
        choice.seen.push(t.key.clone());
        if adopted.is_none() {
            choice.active = Some(t.key.clone());
            adopted = Some(t.label.clone());
        }
    }
    adopted
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

    fn theme_dir(tag: &str) -> Tmp {
        let d = std::env::temp_dir().join(format!("nanti-icons-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("dist")).unwrap();
        std::fs::write(
            d.join("dist/t.json"),
            r#"{
              "iconDefinitions": {
                "file": {"iconPath": "./../icons/file.svg"},
                "rust": {"iconPath": "./../icons/rust.svg"},
                "ts": {"iconPath": "./../icons/ts.svg"},
                "dts": {"iconPath": "./../icons/dts.svg"},
                "npm": {"iconPath": "./../icons/npm.svg"},
                "folder": {"iconPath": "./../icons/folder.svg"},
                "folder-open": {"iconPath": "./../icons/folder-open.svg"},
                "folder-src": {"iconPath": "./../icons/folder-src.svg"},
                "folder-src-open": {"iconPath": "./../icons/folder-src-open.svg"},
                "root": {"iconPath": "./../icons/root.svg"},
                "glyph": {"fontCharacter": "\\E001"}
              },
              "file": "file",
              "folder": "folder",
              "folderExpanded": "folder-open",
              "rootFolder": "root",
              "fileNames": {"Package.json": "npm"},
              "fileExtensions": {"rs": "rust", "ts": "ts", "d.ts": "dts"},
              "folderNames": {"src": "folder-src"},
              "folderNamesExpanded": {"src": "folder-src-open"}
            }"#,
        )
        .unwrap();
        Tmp(d)
    }

    fn file(name: &str) -> PathBuf {
        PathBuf::from(name)
    }

    #[test]
    fn files_resolve_by_name_then_longest_extension_then_the_plain_icon() {
        let t = theme_dir("files");
        let th = IconTheme::load(&t.0.join("dist/t.json")).unwrap();
        let name =
            |p: Option<PathBuf>| p.map(|p| p.file_name().unwrap().to_string_lossy().into_owned());
        assert_eq!(name(th.file_icon("package.json")), Some("npm.svg".into()));
        assert_eq!(name(th.file_icon("PACKAGE.JSON")), Some("npm.svg".into()));
        assert_eq!(name(th.file_icon("lib.rs")), Some("rust.svg".into()));
        assert_eq!(name(th.file_icon("types.d.ts")), Some("dts.svg".into()));
        assert_eq!(name(th.file_icon("app.test.ts")), Some("ts.svg".into()));
        assert_eq!(name(th.file_icon("notes.unknown")), Some("file.svg".into()));
        assert_eq!(name(th.file_icon("Makefile")), Some("file.svg".into()));
        assert_eq!(
            th.file_icon("lib.rs").unwrap(),
            t.0.join("dist").join("./../icons/rust.svg")
        );
        let _ = file("x");
    }

    #[test]
    fn folders_resolve_by_name_open_state_and_root() {
        let t = theme_dir("folders");
        let th = IconTheme::load(&t.0.join("dist/t.json")).unwrap();
        let name =
            |p: Option<PathBuf>| p.map(|p| p.file_name().unwrap().to_string_lossy().into_owned());
        assert_eq!(
            name(th.folder_icon("src", false, false)),
            Some("folder-src.svg".into())
        );
        assert_eq!(
            name(th.folder_icon("SRC", true, false)),
            Some("folder-src-open.svg".into())
        );
        assert_eq!(
            name(th.folder_icon("docs", false, false)),
            Some("folder.svg".into())
        );
        assert_eq!(
            name(th.folder_icon("docs", true, false)),
            Some("folder-open.svg".into())
        );
        assert_eq!(
            name(th.folder_icon("proj", false, true)),
            Some("root.svg".into())
        );
        assert_eq!(
            name(th.folder_icon("proj", true, true)),
            Some("folder-open.svg".into())
        );
    }

    #[test]
    fn glyph_only_themes_are_rejected() {
        let d = std::env::temp_dir().join(format!("nanti-icons-glyph-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let json = d.join("g.json");
        std::fs::write(
            &json,
            r#"{"iconDefinitions":{"a":{"fontCharacter":"\\E001"}}}"#,
        )
        .unwrap();
        assert!(IconTheme::load(&json).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_new_theme_is_switched_on_once_and_a_later_choice_sticks() {
        let a = ThemeEntry {
            key: "x.a-1/a".into(),
            label: "A".into(),
            json: PathBuf::new(),
        };
        let b = ThemeEntry {
            key: "y.b-1/b".into(),
            label: "B".into(),
            json: PathBuf::new(),
        };
        let mut c = Choice::default();
        assert_eq!(
            adopt_new(&mut c, std::slice::from_ref(&a)),
            Some("A".into())
        );
        assert_eq!(c.active.as_deref(), Some("x.a-1/a"));
        // The user goes back to the built-in icons; the same theme must not come back by itself.
        c.active = None;
        assert_eq!(adopt_new(&mut c, std::slice::from_ref(&a)), None);
        assert_eq!(c.active, None);
        // A theme installed later is new again.
        assert_eq!(adopt_new(&mut c, &[a, b]), Some("B".into()));
        assert_eq!(c.active.as_deref(), Some("y.b-1/b"));
    }

    #[test]
    fn the_choice_is_remembered_on_disk() {
        let t = theme_dir("choice");
        assert_eq!(load_choice(&t.0), Choice::default());
        let c = Choice {
            active: Some("p/q".into()),
            seen: vec!["p/q".into(), "r/s".into()],
        };
        save_choice(&t.0, &c).unwrap();
        assert_eq!(load_choice(&t.0), c);
    }

    #[test]
    fn themes_are_found_in_the_extension_manifests() {
        let t = theme_dir("discover");
        let ext_dir = t.0.join("PKief.demo-1.0.0");
        std::fs::create_dir_all(&ext_dir).unwrap();
        std::fs::write(
            ext_dir.join("package.json"),
            r#"{"name":"demo","contributes":{"iconThemes":[{"id":"demo-icons","label":"Demo Icons","path":"./dist/t.json"}]}}"#,
        )
        .unwrap();
        let ext = Extension {
            dir: ext_dir.clone(),
            id: "pkief.demo".into(),
            display_name: "Demo".into(),
            version: "1.0.0".into(),
            description: String::new(),
            engine: String::new(),
            view_type: None,
            commands: 0,
            publisher: "PKief".into(),
            icon: None,
            contributed_commands: Vec::new(),
        };
        let found = discover(&[ext]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, "PKief.demo-1.0.0/demo-icons");
        assert_eq!(found[0].label, "Demo Icons");
        assert_eq!(found[0].json, ext_dir.join("dist/t.json"));
    }

    // Reads the Material Icon Theme the user has installed; needs NANTI_TEST_ICON_THEME_JSON to point at its material-icons.json.
    #[test]
    fn real_material_icon_theme_resolves_common_files_and_folders() {
        let Some(json) = std::env::var_os("NANTI_TEST_ICON_THEME_JSON").map(PathBuf::from) else {
            return;
        };
        let th = IconTheme::load(&json).unwrap();
        let name = |p: Option<PathBuf>| {
            p.map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .unwrap()
        };
        assert_eq!(name(th.file_icon("main.rs")), "rust.svg");
        assert_eq!(name(th.file_icon("script.py")), "python.svg");
        assert_eq!(name(th.file_icon("package.json")), "nodejs.svg");
        assert_eq!(name(th.file_icon("index.html")), "html.svg");
        assert_eq!(name(th.file_icon("unknown.zzz")), "file.svg");
        assert_eq!(name(th.folder_icon("src", false, false)), "folder-src.svg");
        assert_eq!(
            name(th.folder_icon("src", true, false)),
            "folder-src-open.svg"
        );
        assert_eq!(name(th.folder_icon("random", false, false)), "folder.svg");
        for p in [th.file_icon("main.rs"), th.folder_icon("src", false, false)] {
            assert!(p.unwrap().is_file(), "the svg exists on disk");
        }
    }
}
