use std::path::PathBuf;

use crate::panel::HostConfig;

pub fn panel_host_config(
    extension_dir: PathBuf,
    view_type: String,
    workspace: Option<PathBuf>,
) -> HostConfig {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    HostConfig {
        node: std::env::var("NANTI_NODE").unwrap_or_else(|_| "node".into()),
        entry: root.join("extension-host/src/host.mjs"),
        extension_dir,
        view_type,
        workspace,
        selftest_log: std::env::var_os("NANTI_SELFTEST_LOG").map(PathBuf::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_entry_points_at_the_shipped_script() {
        let cfg = panel_host_config(PathBuf::from("ext"), "view".into(), None);
        assert!(cfg.entry.ends_with("extension-host/src/host.mjs"));
        assert!(cfg.entry.components().count() > 2);
        assert_eq!((cfg.view_type.as_str(), cfg.workspace), ("view", None));
    }
}
