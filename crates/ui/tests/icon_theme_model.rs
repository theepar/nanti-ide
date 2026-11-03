use std::path::PathBuf;

use ui::model::Model;

struct Tmp(PathBuf);
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn config_with_theme(tag: &str, folder: &str, id: &str, label: &str) -> Tmp {
    let cfg = std::env::temp_dir().join(format!("nanti-icon-model-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cfg);
    add_theme(&cfg, folder, id, label);
    Tmp(cfg)
}

fn add_theme(cfg: &std::path::Path, folder: &str, id: &str, label: &str) {
    let ext = cfg.join("extensions").join(folder);
    std::fs::create_dir_all(ext.join("dist")).unwrap();
    std::fs::create_dir_all(ext.join("icons")).unwrap();
    std::fs::write(
        ext.join("icons/file.svg"),
        "<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
    )
    .unwrap();
    std::fs::write(
        ext.join("package.json"),
        format!(
            r#"{{"name":"{id}","publisher":"demo","version":"1.0.0","contributes":{{"iconThemes":[{{"id":"{id}","label":"{label}","path":"./dist/t.json"}}]}}}}"#
        ),
    )
    .unwrap();
    std::fs::write(
        ext.join("dist/t.json"),
        r#"{"iconDefinitions":{"file":{"iconPath":"./../icons/file.svg"}},"file":"file"}"#,
    )
    .unwrap();
}

fn model(cfg: &std::path::Path) -> Model {
    let mut m = Model::new(cfg.to_path_buf());
    m.config_dir = Some(cfg.to_path_buf());
    m.load_icon_settings();
    m.refresh_extensions();
    m
}

#[test]
fn a_freshly_installed_icon_theme_switches_on_once_and_the_users_choice_sticks() {
    let t = config_with_theme("flow", "demo.icons-1.0.0", "demo-icons", "Demo Icons");
    let mut m = model(&t.0);
    assert_eq!(m.icon_themes.len(), 1);
    assert_eq!(
        m.icon_choice.active.as_deref(),
        Some("demo.icons-1.0.0/demo-icons")
    );
    assert!(
        m.message
            .as_deref()
            .is_some_and(|s| s.contains("Demo Icons") && s.contains("Settings")),
        "{:?}",
        m.message
    );

    m.set_icon_theme(None);
    m.refresh_extensions();
    assert_eq!(
        m.icon_choice.active, None,
        "going back to the built-in icons stays"
    );

    let again = model(&t.0);
    assert_eq!(again.icon_choice.active, None, "and survives a restart");

    add_theme(&t.0, "demo.more-2.0.0", "more", "More Icons");
    let mut later = model(&t.0);
    assert_eq!(later.icon_themes.len(), 2);
    assert_eq!(
        later.icon_choice.active.as_deref(),
        Some("demo.more-2.0.0/more"),
        "a theme installed later is new again"
    );
    later.set_icon_theme(Some("demo.icons-1.0.0/demo-icons".into()));
    assert_eq!(
        model(&t.0).icon_choice.active.as_deref(),
        Some("demo.icons-1.0.0/demo-icons")
    );
}

#[test]
fn without_a_config_folder_nothing_is_switched_on() {
    let mut m = Model::new(std::env::temp_dir());
    m.refresh_extensions();
    assert!(m.icon_themes.is_empty());
    assert_eq!(m.icon_choice.active, None);
}

#[gpui::test]
fn the_settings_dropdown_renders_open_and_picking_an_entry_closes_it(
    cx: &mut gpui::TestAppContext,
) {
    use gpui::VisualTestContext;
    let t = config_with_theme("dropdown", "demo.icons-1.0.0", "demo-icons", "Demo Icons");
    let cfg = t.0.clone();
    let window = cx.add_window(move |_w, cx| {
        let mut wb = ui::Workbench::new(cfg.clone(), cx);
        wb.set_config_dir(cfg.clone(), None);
        wb.model_mut().settings_open = true;
        wb.model_mut().dropdown_open = Some("set-icon-theme");
        wb
    });
    let vcx = VisualTestContext::from_window(window.into(), cx).into_mut();
    vcx.run_until_parked();
    let (open, themes) = window
        .read_with(vcx, |wb, _| {
            (wb.model().dropdown_open, wb.model().icon_themes.len())
        })
        .unwrap();
    assert_eq!((open, themes), (Some("set-icon-theme"), 1));
    window
        .update(vcx, |wb, _, cx| {
            wb.pick_icon_theme(None);
            wb.model_mut().dropdown_open = None;
            cx.notify();
        })
        .unwrap();
    vcx.run_until_parked();
    let active = window
        .read_with(vcx, |wb, _| wb.model().icon_choice.active.clone())
        .unwrap();
    assert_eq!(active, None);
}

fn add_colors(cfg: &std::path::Path, folder: &str, label: &str, bg: &str) {
    let ext = cfg.join("extensions").join(folder);
    std::fs::create_dir_all(ext.join("themes")).unwrap();
    std::fs::write(
        ext.join("package.json"),
        format!(
            r#"{{"name":"{folder}","publisher":"demo","version":"1.0.0","contributes":{{"themes":[{{"label":"{label}","uiTheme":"vs-dark","path":"./themes/t.json"}}]}}}}"#
        ),
    )
    .unwrap();
    std::fs::write(
        ext.join("themes/t.json"),
        format!(r##"{{"colors":{{"editor.background":"{bg}","editor.foreground":"#eeeeee"}}}}"##),
    )
    .unwrap();
}

fn colour_model(cfg: &std::path::Path) -> Model {
    let mut m = Model::new(cfg.to_path_buf());
    m.config_dir = Some(cfg.to_path_buf());
    m.load_color_settings();
    m.refresh_extensions();
    m
}

#[test]
fn a_colour_theme_switches_on_once_applies_to_the_palette_and_the_choice_sticks() {
    let cfg = std::env::temp_dir().join(format!("nanti-color-model-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cfg);
    let _guard = Tmp(cfg.clone());
    add_colors(&cfg, "demo.night-1.0.0", "Night", "#101010");

    let mut m = colour_model(&cfg);
    assert_eq!(m.color_themes.len(), 1);
    assert_eq!(m.colors.editor, 0x101010);
    assert!(
        m.message
            .as_deref()
            .is_some_and(|s| s.contains("Night") && s.contains("Settings")),
        "{:?}",
        m.message
    );

    m.set_color_theme(None);
    assert_eq!(m.colors, ui::theme::Palette::DARK);
    assert_eq!(
        colour_model(&cfg).colors,
        ui::theme::Palette::DARK,
        "going back survives a restart"
    );

    add_colors(&cfg, "demo.dusk-2.0.0", "Dusk", "#202830");
    let later = colour_model(&cfg);
    assert_eq!(later.color_themes.len(), 2);
    assert_eq!(
        later.colors.editor, 0x202830,
        "a theme installed later is new again"
    );
}

#[test]
fn a_broken_colour_theme_falls_back_to_the_built_in_colours() {
    let cfg = std::env::temp_dir().join(format!("nanti-color-bad-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cfg);
    let _guard = Tmp(cfg.clone());
    add_colors(&cfg, "demo.bad-1.0.0", "Bad", "#101010");
    std::fs::write(
        cfg.join("extensions/demo.bad-1.0.0/themes/t.json"),
        "{ not json",
    )
    .unwrap();
    let m = colour_model(&cfg);
    assert_eq!(m.colors, ui::theme::Palette::DARK);
    assert!(
        m.message
            .as_deref()
            .is_some_and(|s| s.contains("Cannot read")),
        "{:?}",
        m.message
    );
}

#[gpui::test]
fn the_window_draws_with_the_active_colour_theme(cx: &mut gpui::TestAppContext) {
    use gpui::VisualTestContext;
    let cfg = std::env::temp_dir().join(format!("nanti-color-render-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cfg);
    let _guard = Tmp(cfg.clone());
    add_colors(&cfg, "demo.night-1.0.0", "Night", "#101010");
    let dir = cfg.clone();
    let window = cx.add_window(move |_w, cx| {
        let mut wb = ui::Workbench::new(dir.clone(), cx);
        wb.set_config_dir(dir.clone(), None);
        wb
    });
    let vcx = VisualTestContext::from_window(window.into(), cx).into_mut();
    vcx.run_until_parked();
    assert_eq!(ui::theme::current().editor, 0x101010);
}
