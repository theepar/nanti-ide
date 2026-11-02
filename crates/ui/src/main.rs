// Release builds are a pure GUI app: no console window next to the IDE.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::time::Duration;

use gpui::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};
use ui::Workbench;
use ui::panel::{HostConfig, Panel, spawn_worker};

/// The extension shown in the right panel: `NANTI_EXTENSION` + `NANTI_VIEW` win; otherwise the one
/// chosen in the Extensions view (saved in the config folder). `NANTI_SELFTEST_LOG` makes the run
/// self-verifying against the mock extension.
fn panel_choice(config_dir: Option<&std::path::Path>) -> Option<(PathBuf, String)> {
    if let (Some(dir), Ok(view)) = (
        std::env::var_os("NANTI_EXTENSION"),
        std::env::var("NANTI_VIEW"),
    ) {
        return Some((PathBuf::from(dir), view));
    }
    ui::extensions::load_panel_choice(config_dir?)
}

fn host_config_from_env(config_dir: Option<&std::path::Path>) -> Option<HostConfig> {
    let (extension_dir, view_type) = panel_choice(config_dir)?;
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

/// Without a console the "windows" subsystem gives console children (the Node extension host) a
/// visible window of their own. Give the process a hidden console that they inherit instead.
#[cfg(all(windows, not(debug_assertions)))]
fn hide_console() {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    #[link(name = "user32")]
    unsafe extern "system" {
        fn AllocConsole() -> i32;
        fn GetConsoleWindow() -> *mut c_void;
        fn ShowWindow(hwnd: *mut c_void, cmd: i32) -> i32;
    }
    // SAFETY: plain Win32 calls with no pointers owned by us; a null window handle is checked.
    unsafe {
        if AllocConsole() != 0 {
            let hwnd = GetConsoleWindow();
            if !hwnd.is_null() {
                ShowWindow(hwnd, 0); // SW_HIDE
            }
        }
    }
}

fn main() {
    #[cfg(all(windows, not(debug_assertions)))]
    hide_console();
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
        let config_dir =
            ui::layout::layout_path().and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        let config = host_config_from_env(config_dir.as_deref());
        let panel_extension = config.as_ref().map(|c| c.extension_dir.clone());
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
                        if let Some(dir) = config_dir.clone() {
                            workbench.set_config_dir(dir, panel_extension.clone());
                        }
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
