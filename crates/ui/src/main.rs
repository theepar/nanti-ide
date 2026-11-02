use std::path::PathBuf;
use std::time::Duration;

use gpui::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};
use ui::Workbench;
use ui::panel::{HostConfig, Panel, spawn_worker};

/// `NANTI_EXTENSION` (extension dir) + `NANTI_VIEW` (webview view type) pick the extension shown in
/// the right panel. `NANTI_SELFTEST_LOG` makes the run self-verifying against the mock extension.
fn host_config_from_env() -> Option<HostConfig> {
    let extension_dir = PathBuf::from(std::env::var_os("NANTI_EXTENSION")?);
    let view_type = std::env::var("NANTI_VIEW").ok()?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    Some(HostConfig {
        node: std::env::var("NANTI_NODE").unwrap_or_else(|_| "node".into()),
        entry: root.join("extension-host/src/host.mjs"),
        extension_dir,
        view_type,
        workspace: std::env::var_os("NANTI_WORKSPACE").map(PathBuf::from),
        selftest_log: std::env::var_os("NANTI_SELFTEST_LOG").map(PathBuf::from),
    })
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let config = host_config_from_env();
        let root = std::env::var_os("NANTI_WORKSPACE")
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                move |window, cx| {
                    cx.new(|cx| {
                        let mut workbench = Workbench::new(root, cx);
                        workbench.enable_source_control();
                        // Visibility and sizes of every region are remembered between runs.
                        if let Some(path) = ui::layout::layout_path() {
                            workbench.set_layout_path(path);
                        }
                        if let Some(cfg) = config {
                            let diag_log = cfg.selftest_log.clone();
                            let (events, to_host, _shutdown) = spawn_worker(cfg);
                            match Panel::new(window, events, to_host, diag_log) {
                                Ok(panel) => workbench.attach_panel(panel),
                                Err(e) => eprintln!("cannot create WebView2 panel: {e}"),
                            }
                        }
                        workbench
                    })
                },
            )
            .unwrap();

        // Closing the window with unsaved tabs must ask first (CT-001).
        window
            .update(cx, |_, window, cx| {
                let entity = cx.entity();
                window.on_window_should_close(cx, move |_window, cx| {
                    entity.update(cx, |wb, cx| {
                        let ok = wb.allow_window_close();
                        cx.notify();
                        ok
                    })
                });
            })
            .unwrap();

        // Poll the extension host a few times per frame budget; redraw only when needed.
        cx.spawn(async move |cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(30))
                    .await;
                let alive = window
                    .update(cx, |workbench, _window, cx| {
                        if workbench.pump() {
                            cx.notify();
                        }
                    })
                    .is_ok();
                if !alive {
                    break;
                }
            }
        })
        .detach();
        cx.activate(true);
    });
}
