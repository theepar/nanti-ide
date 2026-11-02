use std::path::PathBuf;

use app::host::ExtensionHost;
use serde_json::json;

fn main() {
    let entry = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extension-host/src/host.mjs");
    let node = std::env::var("NANTI_NODE").unwrap_or_else(|_| "node".into());
    match ExtensionHost::spawn(&node, &entry) {
        Ok(mut host) => {
            match host.request("ping", json!({ "hello": "nanti" })) {
                Ok(v) => println!("extension host ok: {v}"),
                Err(e) => eprintln!("ping failed: {e}"),
            }
            let _ = host.shutdown();
        }
        Err(e) => {
            eprintln!("cannot start extension host: {e}");
            std::process::exit(1);
        }
    }
}
