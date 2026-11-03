use std::path::{Path, PathBuf};

const HOST_FILES: [(&str, &str); 6] = [
    (
        "configuration.mjs",
        include_str!("../../../extension-host/src/configuration.mjs"),
    ),
    (
        "context.mjs",
        include_str!("../../../extension-host/src/context.mjs"),
    ),
    (
        "host.mjs",
        include_str!("../../../extension-host/src/host.mjs"),
    ),
    (
        "types.mjs",
        include_str!("../../../extension-host/src/types.mjs"),
    ),
    (
        "vscode-api.mjs",
        include_str!("../../../extension-host/src/vscode-api.mjs"),
    ),
    (
        "webview.mjs",
        include_str!("../../../extension-host/src/webview.mjs"),
    ),
];

fn beside_exe(relative: &str) -> Option<PathBuf> {
    let path = std::env::current_exe()
        .ok()?
        .parent()?
        .join("runtime")
        .join(relative);
    path.is_file().then_some(path)
}

/// The Node.js that runs extensions: `NANTI_NODE`, the one the installer puts in `runtime`, then `node` on PATH.
pub fn node() -> String {
    if let Some(chosen) = std::env::var_os("NANTI_NODE") {
        return chosen.to_string_lossy().into_owned();
    }
    beside_exe("node.exe")
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "node".into())
}

/// Writes the extension host next to the config so a single `nanti.exe` works on any machine.
pub fn host_entry() -> PathBuf {
    let dir = crate::layout::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("host");
    match write_host(&dir) {
        Ok(()) => dir.join("host.mjs"),
        Err(_) => dev_host_entry(),
    }
}

fn dev_host_entry() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extension-host/src/host.mjs")
}

fn write_host(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for (name, text) in HOST_FILES {
        let path = dir.join(name);
        if std::fs::read(&path).is_ok_and(|have| have == text.as_bytes()) {
            continue;
        }
        std::fs::write(path, text)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nanti-runtime-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn the_embedded_host_is_written_whole_and_refreshed() {
        let dir = temp("host");
        write_host(&dir).unwrap();
        for (name, text) in HOST_FILES {
            assert_eq!(std::fs::read_to_string(dir.join(name)).unwrap(), text);
        }
        std::fs::write(dir.join("host.mjs"), "stale").unwrap();
        write_host(&dir).unwrap();
        assert_ne!(
            std::fs::read_to_string(dir.join("host.mjs")).unwrap(),
            "stale"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_host_module_is_embedded() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extension-host/src");
        let mut on_disk: Vec<String> = std::fs::read_dir(src)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        on_disk.sort();
        let embedded: Vec<&str> = HOST_FILES.iter().map(|(n, _)| *n).collect();
        assert_eq!(on_disk, embedded);
    }

    #[test]
    fn the_host_entry_exists_and_node_falls_back_to_path() {
        assert!(host_entry().is_file());
        if std::env::var_os("NANTI_NODE").is_none() {
            assert_eq!(node(), "node");
        }
    }
}
