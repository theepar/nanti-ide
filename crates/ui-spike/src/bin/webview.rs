//! Gate A spike: WebView2 as a child surface of a GPUI window, with a two-way message bridge.
//! Scope is extension panels only (e.g. Claude/Codex chat UI), never the main editor.
//!
//! Self-verifying: JS posts `hello-from-js`, Rust replies via `evaluate_script`, JS acks,
//! Rust logs PASS and exits. Log path: env `NANTI_SPIKE_LOG` (default `webview-spike.log`).

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use gpui::{
    App, Application, Bounds, Context, Window, WindowBounds, WindowOptions, div, prelude::*, px,
    rgb, size,
};
use wry::dpi::{LogicalPosition, LogicalSize};
use wry::{Rect, WebView, WebViewBuilder};

const HTML: &str = r#"<!doctype html><html><body style="background:#1e1e1e;color:#ddd;font-family:sans-serif">
<h3 id="s">webview ready</h3>
<button onclick="window.ipc.postMessage('click')">send</button>
<script>
window.__fromRust = (m) => {
  document.getElementById('s').textContent = 'rust says: ' + m;
  window.ipc.postMessage('ack:' + m);
};
window.ipc.postMessage('hello-from-js');
</script></body></html>"#;

fn log(line: &str) {
    let path = std::env::var("NANTI_SPIKE_LOG").unwrap_or_else(|_| "webview-spike.log".into());
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{line}");
    }
}

struct Root {
    _webview: Rc<RefCell<Option<WebView>>>,
}

impl Render for Root {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x252526))
            .text_color(rgb(0xffffff))
            .child("native gpui area (left) | WebView2 child (right)")
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(900.), px(600.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let slot: Rc<RefCell<Option<WebView>>> = Rc::new(RefCell::new(None));
                let handler_slot = slot.clone();
                let built = WebViewBuilder::new()
                    .with_bounds(Rect {
                        position: LogicalPosition::new(450, 0).into(),
                        size: LogicalSize::new(450, 600).into(),
                    })
                    .with_html(HTML)
                    .with_ipc_handler(move |req| {
                        let msg = req.body();
                        log(&format!("ipc <- js: {msg}"));
                        if msg == "hello-from-js" {
                            if let Some(wv) = handler_slot.borrow().as_ref() {
                                let r = wv.evaluate_script("window.__fromRust('ping-from-rust')");
                                log(&format!("evaluate_script -> {r:?}"));
                            }
                        } else if msg == "ack:ping-from-rust" {
                            log("PASS roundtrip js->rust->js");
                            std::process::exit(0);
                        }
                    })
                    .build_as_child(window);
                match built {
                    Ok(wv) => {
                        log("webview created");
                        *slot.borrow_mut() = Some(wv);
                    }
                    Err(e) => log(&format!("FAIL build_as_child: {e}")),
                }
                cx.new(|_| Root { _webview: slot })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
