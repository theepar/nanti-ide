use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::extensions::Extension;
use crate::icon_theme::ThemeEntry;
use crate::theme::Palette;

/// Dark colour themes that installed extensions contribute (`contributes.themes`).
/// Light themes are left out: parts of the interface put dark text on the accent colour.
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
        let Some(themes) = manifest["contributes"]["themes"].as_array() else {
            continue;
        };
        let folder = ext
            .dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        for t in themes {
            let (Some(label), Some(rel)) = (t["label"].as_str(), t["path"].as_str()) else {
                continue;
            };
            if !matches!(
                t["uiTheme"].as_str().unwrap_or("vs-dark"),
                "vs-dark" | "hc-black"
            ) {
                continue;
            }
            out.push(ThemeEntry {
                key: format!("{folder}/{}", rel.trim_start_matches("./")),
                label: label.to_owned(),
                json: ext.dir.join(rel.trim_start_matches("./")),
            });
        }
    }
    out
}

/// Removes comments and trailing commas, which VS Code theme files are allowed to have.
pub fn strip_jsonc(text: &str) -> String {
    let chars: Vec<char> = text.trim_start_matches('\u{feff}').chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    let mut in_string = false;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            out.push(c);
            if c == '\\' && i + 1 < chars.len() {
                out.push(chars[i + 1]);
                i += 1;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
            out.push(c);
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        } else if c == ',' {
            // A comma followed only by whitespace and a closing bracket is a trailing comma.
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if !matches!(chars.get(j), Some('}' | ']')) {
                out.push(c);
            }
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

/// "#rgb", "#rrggbb" or "#rrggbbaa" as (rgb, alpha 0-255).
fn parse_color(text: &str) -> Option<(u32, u8)> {
    let hex = text.trim().strip_prefix('#')?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let digit = |i: usize| u32::from_str_radix(&hex[i..=i], 16).ok();
    match hex.len() {
        3 => {
            let (r, g, b) = (digit(0)?, digit(1)?, digit(2)?);
            Some((((r * 17) << 16) | ((g * 17) << 8) | (b * 17), 255))
        }
        4 => {
            let (r, g, b, a) = (digit(0)?, digit(1)?, digit(2)?, digit(3)?);
            Some((
                ((r * 17) << 16) | ((g * 17) << 8) | (b * 17),
                (a * 17) as u8,
            ))
        }
        6 => Some((u32::from_str_radix(hex, 16).ok()?, 255)),
        8 => {
            let v = u32::from_str_radix(hex, 16).ok()?;
            Some((v >> 8, (v & 0xff) as u8))
        }
        _ => None,
    }
}

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let channel = |shift: u32| {
        let (x, y) = (((a >> shift) & 0xff) as f32, ((b >> shift) & 0xff) as f32);
        (x + (y - x) * t).round() as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

struct Source {
    colors: HashMap<String, String>,
    // (scope selector, foreground), in file order so a later rule wins a tie.
    rules: Vec<(String, u32)>,
}

fn load_source(json: &Path, depth: u8) -> Result<Source, String> {
    let text = std::fs::read_to_string(json).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&strip_jsonc(&text)).map_err(|e| e.to_string())?;
    let mut src = match v["include"].as_str() {
        Some(rel) if depth < 4 => {
            let base = json.parent().unwrap_or(Path::new("."));
            load_source(&base.join(rel), depth + 1)?
        }
        _ => Source {
            colors: HashMap::new(),
            rules: Vec::new(),
        },
    };
    if let Some(colors) = v["colors"].as_object() {
        for (k, c) in colors {
            if let Some(c) = c.as_str() {
                src.colors.insert(k.clone(), c.to_owned());
            }
        }
    }
    // Older themes call the syntax rules "settings" instead of "tokenColors".
    let rules = v["tokenColors"]
        .as_array()
        .or_else(|| v["settings"].as_array());
    for rule in rules.into_iter().flatten() {
        let Some((fg, _)) = rule["settings"]["foreground"]
            .as_str()
            .and_then(parse_color)
        else {
            continue;
        };
        let scopes: Vec<String> = match &rule["scope"] {
            Value::String(s) => s.split(',').map(|x| x.trim().to_owned()).collect(),
            Value::Array(a) => a
                .iter()
                .filter_map(Value::as_str)
                .flat_map(|s| {
                    s.split(',')
                        .map(|x| x.trim().to_owned())
                        .collect::<Vec<_>>()
                })
                .collect(),
            _ => continue,
        };
        for s in scopes
            .into_iter()
            .filter(|s| !s.is_empty() && !s.contains(' '))
        {
            src.rules.push((s, fg));
        }
    }
    Ok(src)
}

/// The rule whose selector is the longest dotted prefix of the scope; a later rule wins a tie.
fn resolve(rules: &[(String, u32)], scope: &str) -> Option<u32> {
    let mut best: Option<(usize, u32)> = None;
    for (selector, fg) in rules {
        let hit = scope == selector
            || scope
                .strip_prefix(selector.as_str())
                .is_some_and(|r| r.starts_with('.'));
        if hit && best.is_none_or(|(len, _)| selector.len() >= len) {
            best = Some((selector.len(), *fg));
        }
    }
    best.map(|(_, fg)| fg)
}

// The scopes tried, in order, for each token kind of the syntax highlighter (see `theme::TOKEN_ORDER`).
const TOKEN_SCOPES: [&[&str]; 14] = [
    &["keyword.control", "keyword", "storage.type", "storage"],
    &["string"],
    &["comment"],
    &["constant.numeric"],
    &["constant.language", "constant"],
    &[
        "entity.name.type",
        "entity.name.class",
        "support.type",
        "support.class",
    ],
    &["entity.name.function", "support.function"],
    &["entity.other.attribute-name", "variable.parameter"],
    &[
        "support.type.property-name",
        "variable.other.property",
        "meta.object-literal.key",
    ],
    &["keyword.operator"],
    &["entity.name.tag"],
    &["markup.heading", "entity.name.section"],
    &["markup.inline.raw", "markup.fenced_code", "markup.raw"],
    &["markup.underline.link", "string.other.link"],
];

pub fn load(json: &Path) -> Result<Palette, String> {
    let src = load_source(json, 0)?;
    let base = Palette::DARK;
    let color = |key: &str| src.colors.get(key).and_then(|c| parse_color(c));
    let bg = color("editor.background").map_or(base.editor, |(c, _)| c);
    let fg = color("editor.foreground").map_or(base.text, |(c, _)| c);
    // A colour with transparency is shown over the editor background.
    let opaque = |key: &str, over: u32| color(key).map(|(c, a)| mix(over, c, f32::from(a) / 255.0));
    let first = |keys: &[&str], over: u32| keys.iter().find_map(|k| opaque(k, over));
    let rgba =
        |key: &str, fallback: u32| color(key).map_or(fallback, |(c, a)| (c << 8) | u32::from(a));

    let sidebar = opaque("sideBar.background", bg).unwrap_or(bg);
    let activity_bar = opaque("activityBar.background", bg).unwrap_or(sidebar);
    let tab_bar = opaque("editorGroupHeader.tabsBackground", bg).unwrap_or(activity_bar);
    let text_dim = opaque("descriptionForeground", bg).unwrap_or_else(|| mix(fg, bg, 0.45));

    let mut tokens = base.tokens;
    for (i, scopes) in TOKEN_SCOPES.iter().enumerate() {
        if let Some(c) = scopes.iter().find_map(|s| resolve(&src.rules, s)) {
            tokens[i] = c;
        }
    }

    Ok(Palette {
        scroll_track: bg,
        scroll_thumb: opaque("scrollbarSlider.background", bg).unwrap_or_else(|| mix(bg, fg, 0.2)),
        activity_bar,
        sidebar,
        editor: bg,
        tab_bar,
        panel: opaque("panel.background", bg).unwrap_or(bg),
        status_bar: opaque("statusBar.background", bg).unwrap_or(activity_bar),
        border: first(
            &[
                "editorGroup.border",
                "panel.border",
                "sideBar.border",
                "contrastBorder",
            ],
            bg,
        )
        .unwrap_or_else(|| mix(bg, fg, 0.12)),
        text: fg,
        text_dim,
        icon: opaque("icon.foreground", bg).unwrap_or_else(|| mix(fg, bg, 0.15)),
        accent: base.accent,
        line_number: opaque("editorLineNumber.foreground", bg).unwrap_or(text_dim),
        line_number_active: opaque("editorLineNumber.activeForeground", bg).unwrap_or(fg),
        hover: opaque("list.hoverBackground", bg).unwrap_or_else(|| mix(bg, fg, 0.06)),
        selected: first(
            &[
                "list.activeSelectionBackground",
                "list.inactiveSelectionBackground",
            ],
            bg,
        )
        .unwrap_or_else(|| mix(bg, fg, 0.12)),
        hover_strong: opaque("list.focusBackground", bg).unwrap_or_else(|| mix(bg, fg, 0.16)),
        selection: rgba("editor.selectionBackground", base.selection),
        banner: mix(bg, 0xe5c07b, 0.12),
        warn: opaque("editorWarning.foreground", bg).unwrap_or(base.warn),
        find_match: rgba("editor.findMatchHighlightBackground", base.find_match),
        find_current: rgba("editor.findMatchBackground", base.find_current),
        menu_bg: first(
            &[
                "menu.background",
                "dropdown.background",
                "editorWidget.background",
            ],
            bg,
        )
        .unwrap_or_else(|| mix(sidebar, fg, 0.05)),
        tokens,
    })
}

pub fn entry_path(entries: &[ThemeEntry], key: &str) -> Option<PathBuf> {
    entries
        .iter()
        .find(|t| t.key == key)
        .map(|t| t.json.clone())
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

    fn dir(tag: &str) -> Tmp {
        let d = std::env::temp_dir().join(format!("nanti-colors-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Tmp(d)
    }

    #[test]
    fn comments_and_trailing_commas_are_stripped_but_not_inside_strings() {
        let text = "{\n // line\n \"a\": \"x // not a comment, /* nor this */\", /* block */\n \"b\": [1, 2,],\n}";
        let v: Value = serde_json::from_str(&strip_jsonc(text)).unwrap();
        assert_eq!(v["a"], "x // not a comment, /* nor this */");
        assert_eq!(v["b"], serde_json::json!([1, 2]));
    }

    #[test]
    fn colours_parse_in_all_hex_forms() {
        assert_eq!(parse_color("#fff"), Some((0xffffff, 255)));
        assert_eq!(parse_color("#1e1e1e"), Some((0x1e1e1e, 255)));
        assert_eq!(parse_color("#264f7880"), Some((0x264f78, 0x80)));
        assert_eq!(parse_color("#f008"), Some((0xff0000, 0x88)));
        assert_eq!(parse_color("red"), None);
        assert_eq!(parse_color("#12"), None);
    }

    #[test]
    fn a_theme_maps_to_the_palette_with_fallbacks_and_scope_resolution() {
        let t = dir("map");
        std::fs::write(
            t.0.join("base.json"),
            r##"{ "colors": { "editor.background": "#282a36", "editor.foreground": "#f8f8f2", "sideBar.background": "#21222c" },
                   "tokenColors": [ { "scope": "comment", "settings": { "foreground": "#6272a4" } } ] }"##,
        )
        .unwrap();
        std::fs::write(
            t.0.join("theme.json"),
            r##"{
              // a comment
              "include": "./base.json",
              "colors": { "activityBar.background": "#343746", "editor.selectionBackground": "#44475a", "list.hoverBackground": "#ffffff1a", },
              "tokenColors": [
                { "scope": ["string", "string.quoted.double"], "settings": { "foreground": "#f1fa8c" } },
                { "scope": "keyword, storage.type", "settings": { "foreground": "#ff79c6" } },
                { "scope": "keyword.control", "settings": { "foreground": "#ffb86c" } },
                { "scope": "comment", "settings": { "foreground": "#7082b6" } },
                { "scope": "entity.name.function", "settings": { "fontStyle": "italic" } }
              ]
            }"##,
        )
        .unwrap();
        let p = load(&t.0.join("theme.json")).unwrap();
        assert_eq!(p.editor, 0x282a36);
        assert_eq!(p.text, 0xf8f8f2);
        assert_eq!(p.sidebar, 0x21222c, "from the included file");
        assert_eq!(p.activity_bar, 0x343746);
        assert_eq!(p.tab_bar, 0x343746, "falls back to the activity bar");
        assert_eq!(p.status_bar, 0x343746);
        assert_eq!(p.selection, 0x44475aff);
        let idx = |name: &str| {
            crate::theme::TOKEN_ORDER
                .iter()
                .position(|n| *n == name)
                .unwrap()
        };
        assert_eq!(p.tokens[idx("String")], 0xf1fa8c);
        assert_eq!(
            p.tokens[idx("Keyword")],
            0xffb86c,
            "keyword.control is the longer match"
        );
        assert_eq!(
            p.tokens[idx("Comment")],
            0x7082b6,
            "a later rule wins a tie"
        );
        assert_eq!(
            p.tokens[idx("Function")],
            Palette::DARK.tokens[idx("Function")],
            "no foreground means the built-in colour stays"
        );
        // 0xffffff at 10% over the editor background.
        assert_eq!(p.hover, mix(0x282a36, 0xffffff, 0x1a as f32 / 255.0));
        assert_ne!(p.border, p.editor);
        assert_eq!(p.accent, Palette::DARK.accent);
    }

    #[test]
    fn only_dark_themes_are_offered() {
        let t = dir("discover");
        let ext_dir = t.0.join("pub.colors-1.0.0");
        std::fs::create_dir_all(&ext_dir).unwrap();
        std::fs::write(
            ext_dir.join("package.json"),
            r#"{"name":"colors","contributes":{"themes":[
                {"label":"Night","uiTheme":"vs-dark","path":"./themes/night.json"},
                {"label":"Day","uiTheme":"vs","path":"./themes/day.json"},
                {"label":"Contrast","uiTheme":"hc-black","path":"./themes/hc.json"}]}}"#,
        )
        .unwrap();
        let ext = Extension {
            dir: ext_dir.clone(),
            id: "pub.colors".into(),
            display_name: "Colors".into(),
            version: "1.0.0".into(),
            description: String::new(),
            engine: String::new(),
            view_type: None,
            commands: 0,
            publisher: "pub".into(),
            icon: None,
            contributed_commands: Vec::new(),
        };
        let found = discover(&[ext]);
        assert_eq!(
            found.iter().map(|t| t.label.as_str()).collect::<Vec<_>>(),
            ["Night", "Contrast"]
        );
        assert_eq!(found[0].key, "pub.colors-1.0.0/themes/night.json");
        assert_eq!(found[0].json, ext_dir.join("themes/night.json"));
    }

    // Reads a real theme file; needs NANTI_TEST_COLOR_THEME_JSON to point at it.
    #[test]
    fn real_theme_file_loads_into_a_palette() {
        let Some(json) = std::env::var_os("NANTI_TEST_COLOR_THEME_JSON").map(PathBuf::from) else {
            return;
        };
        let p = load(&json).unwrap();
        assert_ne!(p, Palette::DARK, "the theme changed something");
        eprintln!("{p:#?}");
    }
}
