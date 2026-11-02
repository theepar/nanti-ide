use std::path::{Path, PathBuf};

use serde_json::{Value, json};

pub const SIDEBAR_MIN: f32 = 160.0;
pub const SIDEBAR_MAX: f32 = 700.0;
pub const SIDEBAR_DEFAULT: f32 = 260.0;
pub const PANEL_MIN: f32 = 80.0;
pub const PANEL_DEFAULT: f32 = 240.0;
pub const EDITOR_MIN_WIDTH: f32 = 240.0;
pub const EDITOR_MIN_HEIGHT: f32 = 120.0;
const ACTIVITY_BAR_WIDTH: f32 = 48.0;
pub const PANEL_WIDTH_MIN: f32 = 200.0;
pub const PANEL_WIDTH_MAX: f32 = 1200.0;
pub const PANEL_WIDTH_DEFAULT: f32 = 420.0;
const VERSION: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    fn name(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "left" => Some(Side::Left),
            "right" => Some(Side::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelPos {
    Bottom,
    Top,
    Left,
    Right,
}

impl PanelPos {
    pub fn is_side(self) -> bool {
        matches!(self, PanelPos::Left | PanelPos::Right)
    }

    fn name(self) -> &'static str {
        match self {
            PanelPos::Bottom => "bottom",
            PanelPos::Top => "top",
            PanelPos::Left => "left",
            PanelPos::Right => "right",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "bottom" => Some(PanelPos::Bottom),
            "top" => Some(PanelPos::Top),
            "left" => Some(PanelPos::Left),
            "right" => Some(PanelPos::Right),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub activity_bar: bool,
    pub sidebar: bool,
    pub sidebar_width: f32,
    pub sidebar_side: Side,
    pub panel: bool,
    pub panel_height: f32,
    pub panel_width: f32,
    pub panel_pos: PanelPos,
    pub panel_maximized: bool,
    pub status_bar: bool,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            activity_bar: true,
            sidebar: true,
            sidebar_width: SIDEBAR_DEFAULT,
            sidebar_side: Side::Left,
            panel: false,
            panel_height: PANEL_DEFAULT,
            panel_width: PANEL_WIDTH_DEFAULT,
            panel_pos: PanelPos::Bottom,
            panel_maximized: false,
            status_bar: true,
        }
    }
}

impl Layout {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn side_panel_width(&self) -> f32 {
        if self.panel && self.panel_pos.is_side() && !self.panel_maximized {
            self.panel_width
        } else {
            0.0
        }
    }

    pub fn set_panel_width(&mut self, w: f32, window_width: f32) {
        let room = window_width
            - EDITOR_MIN_WIDTH
            - if self.activity_bar {
                ACTIVITY_BAR_WIDTH
            } else {
                0.0
            }
            - if self.sidebar {
                self.sidebar_width
            } else {
                0.0
            };
        self.panel_width = w.clamp(
            PANEL_WIDTH_MIN,
            PANEL_WIDTH_MAX.min(room.max(PANEL_WIDTH_MIN)),
        );
    }

    pub fn effective_panel_width(&self, window_width: f32) -> f32 {
        let room = window_width
            - EDITOR_MIN_WIDTH
            - if self.activity_bar {
                ACTIVITY_BAR_WIDTH
            } else {
                0.0
            }
            - if self.sidebar {
                self.effective_sidebar(window_width)
            } else {
                0.0
            };
        self.panel_width.min(room.max(PANEL_WIDTH_MIN))
    }

    pub fn set_sidebar_width(&mut self, w: f32, window_width: f32) {
        let room = window_width
            - EDITOR_MIN_WIDTH
            - self.side_panel_width()
            - if self.activity_bar {
                ACTIVITY_BAR_WIDTH
            } else {
                0.0
            };
        self.sidebar_width = w.clamp(SIDEBAR_MIN, SIDEBAR_MAX.min(room.max(SIDEBAR_MIN)));
    }

    pub fn set_panel_height(&mut self, h: f32, window_height: f32) {
        let max = (window_height - EDITOR_MIN_HEIGHT).max(PANEL_MIN);
        self.panel_height = h.clamp(PANEL_MIN, max);
    }

    pub fn effective_sidebar(&self, window_width: f32) -> f32 {
        let room = window_width
            - EDITOR_MIN_WIDTH
            - self.side_panel_width()
            - if self.activity_bar {
                ACTIVITY_BAR_WIDTH
            } else {
                0.0
            };
        self.sidebar_width
            .min(room.max(SIDEBAR_MIN))
            .max(SIDEBAR_MIN.min(self.sidebar_width))
    }

    pub fn effective_panel(&self, column_height: f32) -> f32 {
        if self.panel_maximized {
            return column_height;
        }
        self.panel_height
            .min((column_height - EDITOR_MIN_HEIGHT).max(PANEL_MIN))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&json!({
            "version": VERSION,
            "activityBar": self.activity_bar,
            "sidebar": self.sidebar,
            "sidebarWidth": self.sidebar_width,
            "sidebarSide": self.sidebar_side.name(),
            "panel": self.panel,
            "panelHeight": self.panel_height,
            "panelWidth": self.panel_width,
            "panelPosition": self.panel_pos.name(),
            "panelMaximized": self.panel_maximized,
            "statusBar": self.status_bar,
        }))
        .unwrap_or_default()
    }

    pub fn from_json(text: &str) -> Self {
        let mut l = Self::default();
        let Ok(v) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) else {
            return l;
        };
        let flag = |k: &str, d: bool| v.get(k).and_then(Value::as_bool).unwrap_or(d);
        let num = |k: &str, d: f32, lo: f32, hi: f32| {
            v.get(k)
                .and_then(Value::as_f64)
                .filter(|n| n.is_finite())
                .map_or(d, |n| (n as f32).clamp(lo, hi))
        };
        l.activity_bar = flag("activityBar", l.activity_bar);
        l.sidebar = flag("sidebar", l.sidebar);
        l.sidebar_width = num("sidebarWidth", l.sidebar_width, SIDEBAR_MIN, SIDEBAR_MAX);
        l.sidebar_side = v
            .get("sidebarSide")
            .and_then(Value::as_str)
            .and_then(Side::parse)
            .unwrap_or(l.sidebar_side);
        l.panel = flag("panel", l.panel);
        l.panel_height = num("panelHeight", l.panel_height, PANEL_MIN, 4000.0);
        l.panel_width = num(
            "panelWidth",
            l.panel_width,
            PANEL_WIDTH_MIN,
            PANEL_WIDTH_MAX,
        );
        l.panel_pos = v
            .get("panelPosition")
            .and_then(Value::as_str)
            .and_then(PanelPos::parse)
            .unwrap_or(l.panel_pos);
        l.panel_maximized = flag("panelMaximized", l.panel_maximized);
        l.status_bar = flag("statusBar", l.status_bar);
        l
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .map(|t| Self::from_json(&t))
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, self.to_json())?;
        std::fs::rename(&tmp, path).inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
    }
}

pub fn config_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("NANTI_CONFIG_DIR") {
        return Some(PathBuf::from(d));
    }
    if let Some(d) = std::env::var_os("APPDATA") {
        return Some(PathBuf::from(d).join("nanti"));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".nanti"))
}

pub fn layout_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("layout.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip_preserves_everything() {
        let l = Layout {
            activity_bar: false,
            sidebar: false,
            sidebar_width: 333.0,
            sidebar_side: Side::Right,
            panel: true,
            panel_height: 150.0,
            panel_width: 555.0,
            panel_pos: PanelPos::Left,
            panel_maximized: true,
            status_bar: false,
        };
        assert_eq!(Layout::from_json(&l.to_json()), l);
    }

    #[test]
    fn positions_default_to_the_classic_layout_and_unknown_names_fall_back() {
        let d = Layout::default();
        assert_eq!(
            (d.sidebar_side, d.panel_pos),
            (Side::Left, PanelPos::Bottom)
        );
        let l = Layout::from_json(r#"{"sidebarSide": "middle", "panelPosition": 7}"#);
        assert_eq!(
            (l.sidebar_side, l.panel_pos),
            (Side::Left, PanelPos::Bottom)
        );
        let l = Layout::from_json(r#"{"sidebarSide": "right", "panelPosition": "top"}"#);
        assert_eq!((l.sidebar_side, l.panel_pos), (Side::Right, PanelPos::Top));
    }

    #[test]
    fn a_docked_side_panel_is_clamped_and_leaves_room_for_the_editor() {
        let mut l = Layout {
            panel_pos: PanelPos::Left,
            ..Layout::default()
        };
        l.set_panel_width(5000.0, 1280.0);
        assert!(l.panel_width <= 1280.0 - EDITOR_MIN_WIDTH - 48.0 - l.sidebar_width);
        l.set_panel_width(1.0, 1280.0);
        assert_eq!(l.panel_width, PANEL_WIDTH_MIN);
        l.panel_width = 500.0;
        let with_panel = l.effective_sidebar(1280.0);
        l.panel_pos = PanelPos::Bottom;
        assert!(with_panel <= l.effective_sidebar(1280.0));
    }

    #[test]
    fn a_byte_order_mark_from_a_text_editor_is_ignored() {
        let l = Layout::from_json("\u{feff}{\"sidebarSide\": \"right\"}");
        assert_eq!(l.sidebar_side, Side::Right);
    }

    #[test]
    fn the_panel_starts_hidden() {
        assert!(!Layout::default().panel);
    }

    #[test]
    fn corrupt_or_hostile_files_fall_back_safely() {
        assert_eq!(Layout::from_json("not json"), Layout::default());
        assert_eq!(Layout::from_json(""), Layout::default());
        assert_eq!(Layout::from_json("[1,2,3]"), Layout::default());
        let hostile = Layout::from_json(
            r#"{"sidebarWidth": 99999, "panelHeight": "tall", "sidebar": "yes", "extra": 1}"#,
        );
        assert_eq!(hostile.sidebar_width, SIDEBAR_MAX);
        assert_eq!(
            hostile.panel_height, PANEL_DEFAULT,
            "wrong type uses the default"
        );
        assert!(hostile.sidebar, "wrong type uses the default");
        let partial = Layout::from_json(r#"{"panel": false}"#);
        assert!(!partial.panel && partial.sidebar);
    }

    #[test]
    fn dragging_is_clamped_and_never_squeezes_the_editor_out() {
        let mut l = Layout::default();
        l.set_sidebar_width(10.0, 1280.0);
        assert_eq!(l.sidebar_width, SIDEBAR_MIN);
        l.set_sidebar_width(5000.0, 1280.0);
        assert_eq!(l.sidebar_width, SIDEBAR_MAX);
        let mut tiny = Layout::default();
        tiny.set_sidebar_width(400.0, 300.0);
        assert_eq!(
            tiny.sidebar_width, SIDEBAR_MIN,
            "even in a tiny window the minimum holds"
        );
        l.set_panel_height(5.0, 800.0);
        assert_eq!(l.panel_height, PANEL_MIN);
        l.set_panel_height(9999.0, 800.0);
        assert_eq!(l.panel_height, 800.0 - EDITOR_MIN_HEIGHT);
    }

    #[test]
    fn effective_sizes_shrink_with_the_window_without_losing_the_stored_size() {
        let l = Layout::default();
        assert_eq!(l.effective_sidebar(1280.0), 260.0);
        assert!(l.effective_sidebar(700.0) < 260.0 + 1.0);
        assert_eq!(l.sidebar_width, 260.0, "stored size untouched");
        assert_eq!(l.effective_panel(800.0), 240.0);
        assert_eq!(
            l.effective_panel(250.0),
            PANEL_MIN.max(250.0 - EDITOR_MIN_HEIGHT)
        );
        let max = Layout {
            panel_maximized: true,
            ..Layout::default()
        };
        assert_eq!(max.effective_panel(500.0), 500.0);
    }

    #[test]
    fn save_and_load_through_a_file_atomically() {
        let dir = std::env::temp_dir().join(format!("nanti-layout-{}", std::process::id()));
        let path = dir.join("sub").join("layout.json");
        let l = Layout {
            sidebar_width: 301.0,
            sidebar: false,
            ..Layout::default()
        };
        l.save(&path).unwrap();
        assert_eq!(Layout::load(&path), l);
        assert!(!path.with_extension("json.tmp").exists());
        assert_eq!(Layout::load(&dir.join("missing.json")), Layout::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reset_restores_defaults() {
        let mut l = Layout {
            sidebar: false,
            panel_maximized: true,
            sidebar_width: 500.0,
            ..Layout::default()
        };
        l.reset();
        assert_eq!(l, Layout::default());
    }
}
