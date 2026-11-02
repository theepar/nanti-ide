use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionCommand {
    pub id: String,
    pub title: String,
    pub category: Option<String>,
}

impl ExtensionCommand {
    pub fn label(&self) -> String {
        if let Some(cat) = &self.category {
            format!("{cat}: {}", self.title)
        } else {
            self.title.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension {
    pub dir: PathBuf,
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub description: String,
    pub engine: String,
    pub view_type: Option<String>,
    pub commands: usize,
    pub publisher: String,
    pub icon: Option<PathBuf>,
    pub contributed_commands: Vec<ExtensionCommand>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MarketplaceExtension {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub display_name: String,
    pub version: String,
    pub description: String,
    pub download_url: String,
    pub icon_url: Option<String>,
    pub readme_url: Option<String>,
    pub downloads: u64,
    pub rating: Option<f64>,
    pub rating_count: u64,
    pub updated: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtDetail {
    pub id: String,
    pub display_name: String,
    pub publisher: String,
    pub version: String,
    pub description: String,
    pub download_url: Option<String>,
    pub downloads: Option<u64>,
    pub rating: Option<f64>,
    pub rating_count: u64,
    pub updated: Option<String>,
    pub license: Option<String>,
    pub categories: Vec<String>,
    pub readme: Readme,
    pub manifest: ManifestState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureGroup {
    pub title: &'static str,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ManifestInfo {
    pub features: Vec<FeatureGroup>,
    pub pack: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestState {
    Loading,
    Ready(ManifestInfo),
    Failed(String),
}

const FEATURE_ITEM_CAP: usize = 200;

fn readable(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty() && !(t.starts_with('%') && t.ends_with('%'))).then(|| t.to_owned())
}

fn text_of(v: &serde_json::Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).and_then(readable)
}

fn strings_of(v: Option<&serde_json::Value>) -> Vec<String> {
    v.and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_manifest_info(json: &serde_json::Value) -> ManifestInfo {
    let c = json.get("contributes");
    let list = |key: &str| -> Vec<&serde_json::Value> {
        c.and_then(|c| c.get(key))
            .and_then(|a| a.as_array())
            .map(|a| a.iter().collect())
            .unwrap_or_default()
    };
    let mut groups: Vec<FeatureGroup> = Vec::new();
    let mut add = |title: &'static str, mut items: Vec<String>| {
        items.truncate(FEATURE_ITEM_CAP);
        if !items.is_empty() {
            groups.push(FeatureGroup { title, items });
        }
    };

    add(
        "Commands",
        list("commands")
            .into_iter()
            .filter_map(|v| {
                let id = text_of(v, "command")?;
                Some(match (text_of(v, "category"), text_of(v, "title")) {
                    (Some(cat), Some(t)) => format!("{cat}: {t}"),
                    (None, Some(t)) => t,
                    _ => id,
                })
            })
            .collect(),
    );
    add(
        "Keyboard shortcuts",
        list("keybindings")
            .into_iter()
            .filter_map(|v| {
                let key = text_of(v, "key")?;
                let cmd = text_of(v, "command")?;
                Some(format!("{key}  ->  {cmd}"))
            })
            .collect(),
    );
    let mut settings = Vec::new();
    let config = c.and_then(|c| c.get("configuration"));
    let blocks: Vec<&serde_json::Value> = match config {
        Some(serde_json::Value::Array(a)) => a.iter().collect(),
        Some(v @ serde_json::Value::Object(_)) => vec![v],
        _ => Vec::new(),
    };
    for block in blocks {
        if let Some(props) = block.get("properties").and_then(|p| p.as_object()) {
            for (key, spec) in props {
                let desc = text_of(spec, "markdownDescription")
                    .or_else(|| text_of(spec, "description"))
                    .map(|d| {
                        d.lines()
                            .next()
                            .unwrap_or("")
                            .chars()
                            .take(110)
                            .collect::<String>()
                    });
                settings.push(match desc {
                    Some(d) if !d.is_empty() => format!("{key}  -  {d}"),
                    _ => key.clone(),
                });
            }
        }
    }
    add("Settings", settings);
    add(
        "Languages",
        list("languages")
            .into_iter()
            .filter_map(|v| {
                let id = text_of(v, "id")?;
                let aliases = strings_of(v.get("aliases"));
                Some(if aliases.is_empty() {
                    id
                } else {
                    format!("{id} ({})", aliases.join(", "))
                })
            })
            .collect(),
    );
    add(
        "Grammars",
        list("grammars")
            .into_iter()
            .filter_map(|v| text_of(v, "scopeName"))
            .collect(),
    );
    for (key, title) in [
        ("themes", "Color themes"),
        ("iconThemes", "File icon themes"),
        ("productIconThemes", "Product icon themes"),
    ] {
        add(
            title,
            list(key)
                .into_iter()
                .filter_map(|v| text_of(v, "label").or_else(|| text_of(v, "id")))
                .collect(),
        );
    }
    add(
        "Snippets",
        list("snippets")
            .into_iter()
            .filter_map(|v| text_of(v, "language"))
            .collect(),
    );
    let mut views = Vec::new();
    if let Some(map) = c.and_then(|c| c.get("views")).and_then(|v| v.as_object()) {
        for entries in map.values().filter_map(|e| e.as_array()) {
            for v in entries {
                if let Some(id) = text_of(v, "id") {
                    views.push(match text_of(v, "name") {
                        Some(n) => format!("{n} ({id})"),
                        None => id,
                    });
                }
            }
        }
    }
    add("Views", views);
    add(
        "Debuggers",
        list("debuggers")
            .into_iter()
            .filter_map(|v| text_of(v, "label").or_else(|| text_of(v, "type")))
            .collect(),
    );
    add(
        "Task types",
        list("taskDefinitions")
            .into_iter()
            .filter_map(|v| text_of(v, "type"))
            .collect(),
    );
    add(
        "Colors",
        list("colors")
            .into_iter()
            .filter_map(|v| text_of(v, "id"))
            .collect(),
    );

    ManifestInfo {
        features: groups,
        pack: strings_of(json.get("extensionPack")),
        dependencies: strings_of(json.get("extensionDependencies")),
    }
}

pub fn read_local_manifest_info(dir: &Path) -> Option<ManifestInfo> {
    let text = std::fs::read_to_string(dir.join("package.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    Some(parse_manifest_info(&json))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtExtra {
    pub manifest: Option<ManifestInfo>,
    pub readme: Option<String>,
    pub license: Option<String>,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Readme {
    None,
    Loading,
    Ready(String),
    Failed(String),
}

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
        .filter(|d| !d.starts_with('%'))
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
    let mut contributed_commands: Vec<ExtensionCommand> = j
        .pointer("/contributes/commands")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let id = v.get("command").and_then(|c| c.as_str())?.to_owned();
                    let title = v
                        .get("title")
                        .and_then(|t| t.as_str())
                        .map_or_else(|| id.clone(), str::to_owned);
                    let category = v
                        .get("category")
                        .and_then(|c| c.as_str())
                        .map(str::to_owned);
                    Some(ExtensionCommand {
                        id,
                        title,
                        category,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    if let Some(vt) = &view_type {
        let view_cmd_title = format!("Show {}", display);
        if !contributed_commands.iter().any(|c| c.id == *vt) {
            contributed_commands.push(ExtensionCommand {
                id: vt.clone(),
                title: view_cmd_title,
                category: Some("View".to_owned()),
            });
        }
    }
    Some(Extension {
        dir: dir.to_path_buf(),
        id: format!("{publisher}.{name}"),
        display_name: display,
        version,
        description,
        engine,
        view_type,
        commands,
        publisher: publisher.to_owned(),
        icon: j
            .get("icon")
            .and_then(|v| v.as_str())
            .map(|rel| dir.join(rel))
            .filter(|p| p.is_file()),
        contributed_commands,
    })
}

pub fn read_local_readme(dir: &Path) -> Option<String> {
    let entry = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .find(|e| {
            e.file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .starts_with("readme")
        })?;
    let bytes = std::fs::read(entry.path()).ok()?;
    Some(String::from_utf8_lossy(&bytes[..bytes.len().min(README_MAX)]).into_owned())
}

const README_MAX: usize = 400_000;
const ICON_MAX: u64 = 2_000_000;
const ICON_EXTS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "svg"];

pub fn icon_cache_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("cache").join("icons")
}

fn icon_stem(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn cached_icon(config_dir: &Path, id: &str) -> Option<PathBuf> {
    let dir = icon_cache_dir(config_dir);
    let stem = icon_stem(id);
    ICON_EXTS
        .iter()
        .map(|e| dir.join(format!("{stem}.{e}")))
        .find(|p| p.is_file())
}

pub fn download_icon(config_dir: &Path, id: &str, url: &str) -> Result<PathBuf, String> {
    if !url.starts_with("https://open-vsx.org/") {
        return Err("refusing to download an icon from an address that is not Open VSX".into());
    }
    let ext = url
        .rsplit('/')
        .next()
        .and_then(|f| f.rsplit_once('.'))
        .map(|(_, e)| e.to_ascii_lowercase())
        .filter(|e| ICON_EXTS.contains(&e.as_str()))
        .unwrap_or_else(|| "png".to_owned());
    let dir = icon_cache_dir(config_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(format!("{}.{ext}", icon_stem(id)));
    let part = dir.join(format!("{}.{ext}.part", icon_stem(id)));
    let mut cmd = std::process::Command::new("curl.exe");
    cmd.arg("-sSL")
        .arg("--max-time")
        .arg("15")
        .arg("-o")
        .arg(&part)
        .arg(url);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().map_err(|e| format!("cannot run curl: {e}"))?;
    let size = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    if !out.status.success() || size == 0 || size > ICON_MAX {
        let _ = std::fs::remove_file(&part);
        return Err("the icon could not be downloaded".into());
    }
    std::fs::rename(&part, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

pub fn download_readme(url: &str) -> Result<String, String> {
    fetch_text(url, README_MAX)
}

pub fn download_extra(publisher: &str, name: &str) -> Result<ExtExtra, String> {
    let plain = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    };
    if !plain(publisher) || !plain(name) {
        return Err("unexpected extension name".into());
    }
    let json = fetch_text(
        &format!("https://open-vsx.org/api/{publisher}/{name}"),
        4_000_000,
    )?;
    let v: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| format!("invalid response: {e}"))?;
    let readme_url = v
        .pointer("/files/readme")
        .and_then(|u| u.as_str())
        .map(str::to_owned);
    let manifest_url = v
        .pointer("/files/manifest")
        .and_then(|u| u.as_str())
        .map(str::to_owned);
    let (readme, manifest) = std::thread::scope(|s| {
        let manifest = s.spawn(|| {
            manifest_url
                .as_deref()
                .and_then(|u| fetch_text(u, 2_000_000).ok())
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .map(|j| parse_manifest_info(&j))
        });
        let readme = readme_url
            .as_deref()
            .and_then(|u| download_readme(u).ok())
            .filter(|t| !t.trim().is_empty());
        (readme, manifest.join().ok().flatten())
    });
    Ok(ExtExtra {
        manifest,
        readme,
        license: v
            .get("license")
            .and_then(|l| l.as_str())
            .filter(|l| !l.trim().is_empty())
            .map(str::to_owned),
        categories: v
            .get("categories")
            .and_then(|c| c.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|c| c.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn fetch_text(url: &str, cap: usize) -> Result<String, String> {
    if !url.starts_with("https://open-vsx.org/") {
        return Err("refusing to download from an address that is not Open VSX".into());
    }
    let mut cmd = std::process::Command::new("curl.exe");
    cmd.arg("-sSL").arg("--max-time").arg("15").arg(url);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().map_err(|e| format!("cannot run curl: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "could not load the description: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let bytes = &out.stdout[..out.stdout.len().min(cap)];
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

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

pub fn uninstall(config_dir: &Path, dir: &Path) -> Result<(), String> {
    let root = extensions_dir(config_dir);
    if dir.parent() != Some(root.as_path()) || !dir.is_dir() {
        return Err("only extensions installed by nanti. can be removed here".into());
    }
    std::fs::remove_dir_all(dir).map_err(|e| e.to_string())
}

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

pub fn search_marketplace(query: &str) -> Result<Vec<MarketplaceExtension>, String> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let encoded: String = url::form_urlencoded::byte_serialize(q.as_bytes()).collect();
    fetch_marketplace(&format!(
        "https://open-vsx.org/api/-/search?query={encoded}&size=20"
    ))
}

pub fn recommended_marketplace() -> Result<Vec<MarketplaceExtension>, String> {
    fetch_marketplace(
        "https://open-vsx.org/api/-/search?sortBy=downloadCount&sortOrder=desc&size=30",
    )
}

fn fetch_marketplace(url: &str) -> Result<Vec<MarketplaceExtension>, String> {
    let mut cmd = std::process::Command::new("curl.exe");
    cmd.arg("-sSL").arg("--max-time").arg("10").arg(url);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().map_err(|e| format!("cannot run curl: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "search request failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let val: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("invalid search response JSON: {e}"))?;

    let mut list = Vec::new();
    if let Some(items) = val.get("extensions").and_then(|e| e.as_array()) {
        for item in items {
            let namespace = item
                .get("namespace")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");
            let name = item
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown");
            let id = format!("{namespace}.{name}");
            let display_name = item
                .get("displayName")
                .and_then(|d| d.as_str())
                .unwrap_or(name);
            let version = item
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("0.0.0");
            let description = item
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");
            let download_url = item
                .pointer("/files/download")
                .and_then(|d| d.as_str())
                .map(str::to_owned)
                .or_else(|| {
                    item.get("downloadUrl")
                        .and_then(|d| d.as_str())
                        .map(str::to_owned)
                })
                .unwrap_or_default();

            if !download_url.is_empty() {
                list.push(MarketplaceExtension {
                    id,
                    name: name.to_owned(),
                    publisher: namespace.to_owned(),
                    display_name: display_name.to_owned(),
                    version: version.to_owned(),
                    description: description.to_owned(),
                    download_url,
                    icon_url: item
                        .pointer("/files/icon")
                        .and_then(|v| v.as_str())
                        .map(str::to_owned),
                    readme_url: item
                        .pointer("/files/readme")
                        .and_then(|v| v.as_str())
                        .map(str::to_owned),
                    downloads: item
                        .get("downloadCount")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                    rating: item.get("averageRating").and_then(|v| v.as_f64()),
                    rating_count: item
                        .get("ratingCount")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                    updated: item
                        .get("timestamp")
                        .and_then(|v| v.as_str())
                        .map(str::to_owned),
                });
            }
        }
    }
    Ok(list)
}

#[derive(Debug, Default)]
pub struct Progress {
    pub done: std::sync::atomic::AtomicU64,
    pub total: std::sync::atomic::AtomicU64,
}

fn target_platform() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "win32-arm64"
    } else {
        "win32-x64"
    }
}

pub fn resolve_package(
    publisher: &str,
    name: &str,
    listed_url: &str,
) -> Result<(String, u64), String> {
    let plain = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    };
    if !plain(publisher) || !plain(name) {
        return Ok((listed_url.to_owned(), 0));
    }
    let Ok(json) = fetch_text(
        &format!("https://open-vsx.org/api/{publisher}/{name}"),
        4_000_000,
    ) else {
        return Ok((listed_url.to_owned(), 0));
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) else {
        return Ok((listed_url.to_owned(), 0));
    };
    let builds = v
        .get("downloads")
        .and_then(|d| d.as_object())
        .filter(|d| !d.is_empty());
    let Some(builds) = builds else {
        return Ok((listed_url.to_owned(), 0));
    };
    for key in [target_platform(), "universal"] {
        if let Some(url) = builds.get(key).and_then(|u| u.as_str())
            && url.starts_with("https://open-vsx.org/")
        {
            let size = v
                .pointer(&format!("/downloadSizes/{key}"))
                .and_then(|n| n.as_u64())
                .unwrap_or(0);
            return Ok((url.to_owned(), size));
        }
    }
    Err(format!(
        "{publisher}.{name} has no build for this computer ({})",
        target_platform()
    ))
}

pub fn download_and_install(
    config_dir: &Path,
    publisher: &str,
    name: &str,
    listed_url: &str,
    progress: &Progress,
) -> Result<Extension, String> {
    use std::sync::atomic::Ordering;
    if !listed_url.starts_with("https://open-vsx.org/") {
        return Err("refusing to download from an address that is not Open VSX".into());
    }
    let (url, size) = resolve_package(publisher, name, listed_url)?;
    progress.done.store(0, Ordering::Relaxed);
    progress.total.store(size, Ordering::Relaxed);

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let tmp = std::env::temp_dir().join(format!("nanti-ext-{}-{stamp}.vsix", std::process::id()));
    let mut cmd = std::process::Command::new("curl.exe");
    // --fail: an HTTP error must not be saved as if it were the package.
    cmd.args([
        "-sSL",
        "--fail",
        "--connect-timeout",
        "15",
        "--max-time",
        "1800",
    ])
    .args(["--speed-limit", "2048", "--speed-time", "30"])
    .arg("-o")
    .arg(&tmp)
    .arg(&url)
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd.spawn().map_err(|e| format!("cannot run curl: {e}"))?;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                let got = std::fs::metadata(&tmp).map_or(0, |m| m.len());
                progress.done.store(got, Ordering::Relaxed);
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                return Err(format!("download failed: {e}"));
            }
        }
    };
    progress.done.store(
        std::fs::metadata(&tmp).map_or(0, |m| m.len()),
        Ordering::Relaxed,
    );
    if !status.success() {
        let mut why = String::new();
        if let Some(mut e) = child.stderr.take() {
            use std::io::Read;
            let _ = e.read_to_string(&mut why);
        }
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("download failed: {}", why.trim()));
    }
    let res = install_vsix(config_dir, &tmp);
    let _ = std::fs::remove_file(&tmp);
    res
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

    #[test]
    #[ignore = "needs the network and downloads real packages"]
    fn installs_real_extensions_from_open_vsx() {
        let wanted = std::env::var("NANTI_TEST_EXTS").unwrap_or_else(|_| {
            "usernamehw.errorlens,tamasfe.even-better-toml,esbenp.prettier-vscode,\
             dbaeumer.vscode-eslint,redhat.vscode-yaml,rust-lang.rust-analyzer"
                .to_owned()
        });
        let dir = tmp("real");
        let mut failures = Vec::new();
        for id in wanted.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let Some((publisher, name)) = id.split_once('.') else {
                continue;
            };
            let listed = search_marketplace(name)
                .ok()
                .and_then(|r| r.into_iter().find(|m| m.id.eq_ignore_ascii_case(id)));
            let Some(listed) = listed else {
                failures.push(format!("{id}: not found by the search"));
                continue;
            };
            let progress = Progress::default();
            let started = std::time::Instant::now();
            match download_and_install(&dir.0, publisher, name, &listed.download_url, &progress) {
                Ok(ext) => {
                    let mb = progress.done.load(std::sync::atomic::Ordering::Relaxed) / 1_000_000;
                    println!(
                        "OK  {id} {} ({mb} MB, {:.1}s) -> {}",
                        ext.version,
                        started.elapsed().as_secs_f32(),
                        ext.dir.display()
                    );
                    if !ext.id.eq_ignore_ascii_case(id) {
                        failures.push(format!("{id}: installed as {}", ext.id));
                    }
                }
                Err(e) => failures.push(format!("{id}: {e}")),
            }
        }
        assert!(
            failures.is_empty(),
            "failed installs:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn icon_and_readme_downloads_are_limited_to_open_vsx() {
        let dir = tmp("ic");
        for bad in [
            "https://evil.example/icon.png",
            "http://open-vsx.org/icon.png",
            "",
        ] {
            assert!(download_icon(&dir.0, "a.b", bad).is_err(), "{bad:?}");
            assert!(download_readme(bad).is_err(), "{bad:?}");
        }
        assert!(cached_icon(&dir.0, "a.b").is_none());
        std::fs::create_dir_all(icon_cache_dir(&dir.0)).unwrap();
        let p = icon_cache_dir(&dir.0).join("a.b.png");
        std::fs::write(&p, [1u8]).unwrap();
        assert_eq!(cached_icon(&dir.0, "a.b"), Some(p));
    }

    #[test]
    fn a_manifest_icon_is_used_only_if_the_file_exists() {
        let dir = tmp("mi");
        let ext = dir.0.join("x");
        write_manifest(
            &ext,
            r#"{"name":"n","publisher":"p","version":"1.0.0","icon":"icon.png"}"#,
        );
        assert!(read_manifest(&ext).unwrap().icon.is_none());
        std::fs::write(ext.join("icon.png"), [1u8]).unwrap();
        let m = read_manifest(&ext).unwrap();
        assert_eq!(m.icon, Some(ext.join("icon.png")));
        assert_eq!(m.publisher, "p");
    }

    #[test]
    fn local_readme_is_found_case_insensitively() {
        let dir = tmp("rd");
        assert!(read_local_readme(&dir.0).is_none());
        std::fs::write(dir.0.join("ReadMe.MD"), "# Hello").unwrap();
        assert_eq!(read_local_readme(&dir.0).as_deref(), Some("# Hello"));
    }

    #[test]
    fn downloads_are_limited_to_open_vsx() {
        let dir = tmp("dl");
        for bad in [
            "http://open-vsx.org/api/a/b/1/file/x.vsix",
            "https://open-vsx.org.evil.example/x.vsix",
            "https://evil.example/https://open-vsx.org/x.vsix",
            "file:///C:/x.vsix",
            "",
        ] {
            let err =
                download_and_install(&dir.0, "a", "b", bad, &Progress::default()).unwrap_err();
            assert!(err.contains("Open VSX"), "{bad:?}: {err}");
        }
    }

    fn write_manifest(dir: &Path, json: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("package.json"), json).unwrap();
    }

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
