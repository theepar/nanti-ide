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
