use std::path::PathBuf;

use crate::panel::HostConfig;

pub fn panel_host_config(
    extension_dir: PathBuf,
    view_type: String,
    workspace: Option<PathBuf>,
) -> HostConfig {
    HostConfig {
        node: crate::runtime::node(),
        entry: crate::runtime::host_entry(),
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
        assert!(cfg.entry.ends_with("host.mjs"));
        assert!(cfg.entry.is_file());
        assert_eq!((cfg.view_type.as_str(), cfg.workspace), ("view", None));
    }
}
