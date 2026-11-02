use std::path::PathBuf;

use app::host::{ExtensionHost, HostError};
use serde_json::json;

fn entry() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extension-host/src/host.mjs")
}

#[test]
fn handshake_and_ping_roundtrip() {
    let mut host = ExtensionHost::spawn("node", &entry()).expect("spawn host");
    let v = host.request("ping", json!({ "n": 1 })).expect("ping");
    assert_eq!(v["echo"], json!({ "n": 1 }));
    host.shutdown().expect("shutdown");
}

#[test]
fn unknown_method_returns_unsupported_capability() {
    let mut host = ExtensionHost::spawn("node", &entry()).expect("spawn host");
    let err = host.request("vscode.nope", json!({})).unwrap_err();
    match err {
        HostError::Remote { code, .. } => assert_eq!(code, "UnsupportedCapability"),
        other => panic!("unexpected error: {other}"),
    }
    host.shutdown().expect("shutdown");
}

fn mock_extension() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/mock-extension")
}

/// Gate B: a real extension (not a stub) loads, activates, runs commands and touches the workspace.
#[test]
fn mock_extension_end_to_end() {
    let ws = std::env::temp_dir().join(format!("nanti-ws-{}", std::process::id()));
    std::fs::create_dir_all(&ws).unwrap();
    std::fs::write(ws.join("a.txt"), "héllo").unwrap();

    let mut host =
        ExtensionHost::spawn_with_workspace("node", &entry(), &[ws.as_path()]).expect("spawn host");
    let id = host.load_extension(&mock_extension()).expect("load");
    assert_eq!(id, "nanti-test.mock");
    assert_eq!(host.activate_extension(&id).expect("activate"), "activated");

    let hello = host
        .execute_command("mock.hello", vec![json!("nanti")])
        .unwrap();
    assert_eq!(hello, json!("hello nanti"));
    let notes = host.take_notifications();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].method, "window.showInformationMessage");
    assert_eq!(notes[0].params["message"], "hello nanti");

    let text = host
        .execute_command("mock.readFile", vec![json!("a.txt")])
        .unwrap();
    assert_eq!(text, json!("héllo"));

    match host
        .execute_command("mock.unsupported", vec![])
        .unwrap_err()
    {
        HostError::Remote { code, message } => {
            assert_eq!(code, "UnsupportedCapability");
            assert!(message.contains("vscode.debug.startDebugging"), "{message}");
        }
        other => panic!("unexpected error: {other}"),
    }

    host.deactivate_extension(&id).expect("deactivate");
    host.shutdown().expect("shutdown");
    std::fs::remove_dir_all(&ws).unwrap();
}

fn mock_webview_extension() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/mock-webview-extension")
}

/// Wait for a notification with `method`, returning it; panics after a few seconds.
fn wait_for(host: &mut ExtensionHost, method: &str) -> app::host::Notification {
    for _ in 0..50 {
        if let Some(n) = host.recv_notification(std::time::Duration::from_millis(100)) {
            if n.method == method {
                return n;
            }
        }
    }
    panic!("no '{method}' notification");
}

/// VH-06/VH-07 over the real IPC: view provider, HTML, resource URI, async messages, URI callback.
#[test]
fn webview_view_and_uri_handler_end_to_end() {
    let mut host = ExtensionHost::spawn("node", &entry()).expect("spawn host");
    let id = host
        .load_extension(&mock_webview_extension())
        .expect("load");
    assert_eq!(host.activate_extension(&id).expect("activate"), "activated");

    let reg = wait_for(&mut host, "webview.viewProviderRegistered");
    assert_eq!(reg.params["viewType"], "mockweb.view");

    let handle = host.resolve_view("mockweb.view").expect("resolve");
    let html = wait_for(&mut host, "webview.setHtml");
    assert_eq!(html.params["handle"], json!(handle));
    assert!(html.params["html"]
        .as_str()
        .unwrap()
        .contains("https://nanti-resource.localhost/file/"));

    host.send_webview_message(&handle, json!({ "type": "ready" }))
        .unwrap();
    let ping = wait_for(&mut host, "webview.postMessage");
    assert_eq!(ping.params["message"], json!({ "type": "ping", "n": 1 }));

    host.send_webview_message(&handle, json!({ "type": "pong", "n": 1 }))
        .unwrap();
    let shown = wait_for(&mut host, "window.showInformationMessage");
    assert_eq!(shown.params["message"], "pong 1");

    host.handle_uri("nanti://nanti-test.mockweb/callback?code=abc")
        .unwrap();
    let shown = wait_for(&mut host, "window.showInformationMessage");
    assert_eq!(shown.params["message"], "uri /callback?code=abc");
    match host.handle_uri("nanti://someone.else/cb").unwrap_err() {
        HostError::Remote { code, .. } => assert_eq!(code, "NoUriHandler"),
        other => panic!("unexpected error: {other}"),
    }

    host.dispose_webview(&handle).unwrap();
    host.shutdown().expect("shutdown");
}

#[test]
fn killed_host_reports_error_not_hang() {
    let mut host = ExtensionHost::spawn("node", &entry()).expect("spawn host");
    host.kill().expect("kill");
    assert!(host.request("ping", json!({})).is_err());
}
