use std::path::{Path, PathBuf};

pub const ORIGIN: &str = "https://nanti-resource.localhost";

#[derive(Default, Debug, Clone)]
pub struct ResourceState {
    pub html: String,
    pub roots: Vec<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Served {
    pub status: u16,
    pub mime: &'static str,
    pub body: Vec<u8>,
}

fn plain(status: u16, msg: &str) -> Served {
    Served {
        status,
        mime: "text/plain; charset=utf-8",
        body: msg.as_bytes().to_vec(),
    }
}

pub fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

pub fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json" | "map") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("wasm") => "application/wasm",
        Some("txt" | "md") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub fn serve(state: &ResourceState, path: &str) -> Served {
    if path == "/html" {
        return Served {
            status: 200,
            mime: "text/html; charset=utf-8",
            body: state.html.clone().into_bytes(),
        };
    }
    let Some(encoded) = path.strip_prefix("/file/") else {
        return plain(404, "not found");
    };
    let Some(decoded) = percent_decode(encoded) else {
        return plain(400, "bad path encoding");
    };
    let Ok(target) = Path::new(&decoded).canonicalize() else {
        return plain(404, "not found");
    };
    let allowed = state
        .roots
        .iter()
        .filter_map(|r| r.canonicalize().ok())
        .any(|root| target.starts_with(root));
    if !allowed {
        return plain(403, "outside localResourceRoots");
    }
    if !target.is_file() {
        return plain(404, "not found");
    }
    match std::fs::read(&target) {
        Ok(body) => Served {
            status: 200,
            mime: mime_for(&target),
            body,
        },
        Err(_) => plain(404, "not found"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_tree() -> (PathBuf, PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "nanti-res-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("media");
        std::fs::create_dir_all(&root).unwrap();
        let inside = root.join("main.js");
        std::fs::write(&inside, "console.log(1)").unwrap();
        let outside = base.join("secret.txt");
        std::fs::write(&outside, "secret").unwrap();
        (root, inside, outside)
    }

    fn enc(p: &Path) -> String {
        let mut s = String::from("/file/");
        for b in p.to_string_lossy().bytes() {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                s.push(b as char);
            } else {
                s.push_str(&format!("%{b:02X}"));
            }
        }
        s
    }

    #[test]
    fn serves_file_inside_root_with_mime() {
        let (root, inside, _) = temp_tree();
        let st = ResourceState {
            html: String::new(),
            roots: vec![root],
        };
        let r = serve(&st, &enc(&inside));
        assert_eq!(r.status, 200);
        assert!(r.mime.starts_with("text/javascript"));
        assert_eq!(r.body, b"console.log(1)");
    }

    #[test]
    fn denies_file_outside_root() {
        let (root, _, outside) = temp_tree();
        let st = ResourceState {
            html: String::new(),
            roots: vec![root],
        };
        assert_eq!(serve(&st, &enc(&outside)).status, 403);
    }

    #[test]
    fn denies_dotdot_traversal() {
        let (root, _, outside) = temp_tree();
        let sneaky = root.join("..").join(outside.file_name().unwrap());
        let st = ResourceState {
            html: String::new(),
            roots: vec![root],
        };
        assert_eq!(serve(&st, &enc(&sneaky)).status, 403);
    }

    #[test]
    fn denies_everything_without_roots() {
        let (_, inside, _) = temp_tree();
        let st = ResourceState::default();
        assert_eq!(serve(&st, &enc(&inside)).status, 403);
    }

    #[test]
    fn serves_html_and_rejects_unknown_paths() {
        let st = ResourceState {
            html: "<p>hi</p>".into(),
            roots: vec![],
        };
        let r = serve(&st, "/html");
        assert_eq!(
            (r.status, r.body.as_slice()),
            (200, b"<p>hi</p>".as_slice())
        );
        assert_eq!(serve(&st, "/other").status, 404);
        assert_eq!(serve(&st, "/file/%ZZ").status, 400);
        assert_eq!(
            serve(&st, &enc(Path::new("Z:/definitely/missing.txt"))).status,
            404
        );
    }

    #[test]
    fn percent_decode_handles_utf8() {
        assert_eq!(percent_decode("a%20b%C3%A9").as_deref(), Some("a bé"));
        assert_eq!(percent_decode("%4"), None);
    }
}
