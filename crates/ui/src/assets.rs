use std::borrow::Cow;

use gpui::{AssetSource, Pixels, Result, SharedString, Styled, Svg, px, rgb, svg};

const ASSETS: &[(&str, &str)] = &[
    (
        "logo-mark.svg",
        include_str!("../../../assets/logo-mark.svg"),
    ),
    ("icons/file.svg", include_str!("../assets/icons/file.svg")),
    (
        "icons/folder.svg",
        include_str!("../assets/icons/folder.svg"),
    ),
    ("icons/close.svg", include_str!("../assets/icons/close.svg")),
    ("icons/error.svg", include_str!("../assets/icons/error.svg")),
    (
        "icons/warning.svg",
        include_str!("../assets/icons/warning.svg"),
    ),
    ("icons/plus.svg", include_str!("../assets/icons/plus.svg")),
    ("icons/trash.svg", include_str!("../assets/icons/trash.svg")),
    (
        "icons/maximize.svg",
        include_str!("../assets/icons/maximize.svg"),
    ),
    (
        "icons/restore.svg",
        include_str!("../assets/icons/restore.svg"),
    ),
    (
        "icons/layout-sidebar-on.svg",
        include_str!("../assets/icons/layout-sidebar-on.svg"),
    ),
    (
        "icons/layout-panel-on.svg",
        include_str!("../assets/icons/layout-panel-on.svg"),
    ),
    (
        "icons/minimize.svg",
        include_str!("../assets/icons/minimize.svg"),
    ),
    (
        "icons/settings.svg",
        include_str!("../assets/icons/settings.svg"),
    ),
    ("icons/files.svg", include_str!("../assets/icons/files.svg")),
    (
        "icons/search.svg",
        include_str!("../assets/icons/search.svg"),
    ),
    ("icons/git.svg", include_str!("../assets/icons/git.svg")),
    ("icons/run.svg", include_str!("../assets/icons/run.svg")),
    (
        "icons/extensions.svg",
        include_str!("../assets/icons/extensions.svg"),
    ),
    (
        "icons/chevron-right.svg",
        include_str!("../assets/icons/chevron-right.svg"),
    ),
    (
        "icons/chevron-down.svg",
        include_str!("../assets/icons/chevron-down.svg"),
    ),
    (
        "icons/folder-open.svg",
        include_str!("../assets/icons/folder-open.svg"),
    ),
    (
        "icons/file-plus.svg",
        include_str!("../assets/icons/file-plus.svg"),
    ),
    (
        "icons/folder-plus.svg",
        include_str!("../assets/icons/folder-plus.svg"),
    ),
    (
        "icons/person.svg",
        include_str!("../assets/icons/person.svg"),
    ),
    ("icons/clock.svg", include_str!("../assets/icons/clock.svg")),
    ("icons/copy.svg", include_str!("../assets/icons/copy.svg")),
    (
        "icons/external.svg",
        include_str!("../assets/icons/external.svg"),
    ),
    (
        "icons/commit.svg",
        include_str!("../assets/icons/commit.svg"),
    ),
    (
        "icons/refresh.svg",
        include_str!("../assets/icons/refresh.svg"),
    ),
    (
        "icons/collapse.svg",
        include_str!("../assets/icons/collapse.svg"),
    ),
    (
        "icons/layout-sidebar.svg",
        include_str!("../assets/icons/layout-sidebar.svg"),
    ),
    (
        "icons/layout-panel.svg",
        include_str!("../assets/icons/layout-panel.svg"),
    ),
    ("icons/file.svg", include_str!("../assets/icons/file.svg")),
    (
        "icons/folder.svg",
        include_str!("../assets/icons/folder.svg"),
    ),
    ("icons/close.svg", include_str!("../assets/icons/close.svg")),
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(ASSETS
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, data)| Cow::Borrowed(data.as_bytes())))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ASSETS
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, _)| SharedString::from(*p))
            .collect())
    }
}

pub fn icon(name: &str, size: impl Into<Pixels>, color: u32) -> Svg {
    let size = size.into();
    svg()
        .path(SharedString::from(format!("icons/{name}.svg")))
        .size(size)
        .flex_none()
        .text_color(rgb(color))
}

pub fn logo_mark(width: f32, color: u32) -> Svg {
    svg()
        .path("logo-mark.svg")
        .w(px(width))
        .h(px(width * 220. / 252.))
        .flex_none()
        .text_color(rgb(color))
}

pub fn file_badge(name: &str) -> Option<&'static str> {
    let ext = name.rsplit_once('.').map_or("", |(_, e)| e);
    Some(match ext.to_ascii_lowercase().as_str() {
        "rs" => "RS",
        "js" | "mjs" | "cjs" => "JS",
        "jsx" => "JSX",
        "ts" => "TS",
        "tsx" => "TSX",
        "json" | "jsonc" => "{}",
        "md" | "markdown" => "MD",
        "toml" => "TM",
        "yaml" | "yml" => "YML",
        "go" => "GO",
        "dart" => "DT",
        "py" => "PY",
        "html" | "htm" => "<>",
        "css" => "#",
        "scss" | "sass" => "SC",
        "sh" | "bash" | "zsh" => ">_",
        "ps1" | "bat" | "cmd" => ">_",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" => "C++",
        "cs" => "C#",
        "java" => "J",
        "kt" | "kts" => "KT",
        "php" => "PHP",
        "rb" => "RB",
        "swift" => "SW",
        "lua" => "LUA",
        "sql" => "SQL",
        "xml" => "XML",
        "lock" => "LK",
        "txt" => "TXT",
        _ => return None,
    })
}

pub fn file_color(name: &str) -> u32 {
    let ext = name.rsplit_once('.').map_or("", |(_, e)| e);
    match ext.to_ascii_lowercase().as_str() {
        "py" => 0x4b8bbe,
        "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" => 0x519aba,
        "cs" => 0x6fb26f,
        "java" | "kt" | "kts" => 0xe76f00,
        "php" => 0x8993be,
        "rb" => 0xcc3e44,
        "swift" => 0xf05138,
        "lua" => 0x51a0cf,
        "sql" | "xml" => 0xdad8d8,
        "rs" => 0xde7f57,
        "js" | "mjs" | "cjs" | "jsx" => 0xe5c07b,
        "ts" | "tsx" => 0x4f9fe8,
        "json" => 0xcbb26b,
        "md" | "markdown" => 0x6fb1e8,
        "toml" | "yaml" | "yml" | "lock" => 0x9aa5b1,
        "go" => 0x58c4dd,
        "dart" => 0x4ec9b0,
        "html" | "htm" => 0xe37933,
        "css" | "scss" => 0x9b7fe8,
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" => 0x81b88b,
        "sh" | "ps1" | "bat" | "cmd" => 0x8bc34a,
        _ => 0x858585,
    }
}
