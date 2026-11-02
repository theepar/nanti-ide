use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use editor_core::find::{FindOptions, Finder};

use crate::palette::{TextField, list_files};

pub const MAX_FILE_BYTES: u64 = 1_000_000;
pub const MAX_HITS: usize = 2_000;
pub const MAX_FILES_WITH_HITS: usize = 200;
pub const TIME_BUDGET: Duration = Duration::from_secs(2);
const PREVIEW_CHARS: usize = 160;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub line: usize,
    pub col: usize,
    pub len: usize,
    pub preview: String,
    pub preview_range: std::ops::Range<usize>,
}

#[derive(Debug, Clone)]
pub struct FileHits {
    pub path: PathBuf,
    pub rel: String,
    pub hits: Vec<Hit>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchResults {
    pub files: Vec<FileHits>,
    pub total_hits: usize,
    pub files_scanned: usize,
    pub truncated: bool,
    pub error: Option<String>,
}

pub fn search_workspace(root: &Path, query: &str, opts: FindOptions) -> SearchResults {
    let mut out = SearchResults::default();
    let finder = match Finder::new(query, opts) {
        Ok(Some(f)) => f,
        Ok(None) => return out,
        Err(e) => {
            out.error = Some(e.to_string());
            return out;
        }
    };
    let started = Instant::now();
    for (rel, abs) in list_files(root) {
        if started.elapsed() > TIME_BUDGET
            || out.total_hits >= MAX_HITS
            || out.files.len() >= MAX_FILES_WITH_HITS
        {
            out.truncated = true;
            break;
        }
        let Some(text) = read_text(&abs) else {
            continue;
        };
        out.files_scanned += 1;
        let mut hits = Vec::new();
        for (line_no, line) in text.lines().enumerate() {
            for m in finder.find_all(line) {
                hits.push(make_hit(line_no, line, m));
                if out.total_hits + hits.len() >= MAX_HITS {
                    break;
                }
            }
            if out.total_hits + hits.len() >= MAX_HITS {
                out.truncated = true;
                break;
            }
        }
        if !hits.is_empty() {
            out.total_hits += hits.len();
            out.files.push(FileHits {
                path: abs,
                rel,
                hits,
            });
        }
    }
    out
}

fn read_text(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > MAX_FILE_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    if bytes.iter().take(8000).any(|&b| b == 0) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn make_hit(line_no: usize, line: &str, m: std::ops::Range<usize>) -> Hit {
    let lead = line.chars().take_while(|c| c.is_whitespace()).count();
    let chars: Vec<char> = line.chars().collect();
    let start = lead.min(m.start);
    let mut from = start;
    if m.end.saturating_sub(from) > PREVIEW_CHARS {
        from = m.start;
    }
    let to = (from + PREVIEW_CHARS)
        .min(chars.len())
        .max(m.end.min(chars.len()));
    let preview: String = chars[from..to].iter().collect();
    Hit {
        line: line_no,
        col: m.start,
        len: m.end - m.start,
        preview,
        preview_range: m.start - from..(m.end - from).min(to - from),
    }
}

pub struct SearchState {
    pub query: TextField,
    pub opts: FindOptions,
    pub results: Option<SearchResults>,
    pub collapsed: BTreeSet<PathBuf>,
    pub focused: bool,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: TextField::default(),
            opts: FindOptions::default(),
            results: None,
            collapsed: BTreeSet::new(),
            focused: false,
        }
    }

    pub fn run(&mut self, root: &Path) {
        self.collapsed.clear();
        self.results = Some(search_workspace(root, self.query.text(), self.opts));
    }

    pub fn toggle_file(&mut self, path: &Path) {
        if !self.collapsed.remove(path) {
            self.collapsed.insert(path.to_path_buf());
        }
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tmp(PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn ws(tag: &str) -> Tmp {
        let d = std::env::temp_dir().join(format!("nanti-search-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::create_dir_all(d.join("node_modules/x")).unwrap();
        std::fs::write(d.join("a.txt"), "hello world\n  Hello again\nnothing\n").unwrap();
        std::fs::write(
            d.join("src/b.rs"),
            "fn hello() {}\r\n// héllo 日本 hello\r\n",
        )
        .unwrap();
        std::fs::write(d.join("node_modules/x/i.js"), "hello").unwrap();
        std::fs::write(d.join("bin.dat"), [b'h', b'e', 0, b'l', b'l', b'o']).unwrap();
        std::fs::write(d.join("latin1.txt"), [b'h', 0xe9, b'l', b'l', b'o']).unwrap();
        Tmp(d)
    }

    const CI: FindOptions = FindOptions {
        case_sensitive: false,
        whole_word: false,
        regex: false,
    };

    #[test]
    fn finds_hits_with_positions_skipping_binary_non_utf8_and_heavy_dirs() {
        let d = ws("basic");
        let r = search_workspace(&d.0, "hello", CI);
        let rels: Vec<&str> = r.files.iter().map(|f| f.rel.as_str()).collect();
        assert_eq!(
            rels,
            ["a.txt", "src/b.rs"],
            "no node_modules, no binary, no latin1"
        );
        let a = &r.files[0];
        assert_eq!(
            a.hits
                .iter()
                .map(|h| (h.line, h.col, h.len))
                .collect::<Vec<_>>(),
            [(0, 0, 5), (1, 2, 5)]
        );
        assert_eq!(a.hits[1].preview, "Hello again", "leading spaces trimmed");
        assert_eq!(a.hits[1].preview_range, 0..5);
        let b = &r.files[1];
        assert_eq!(
            b.hits.len(),
            2,
            "CRLF lines work; héllo does not match 'hello'"
        );
        assert_eq!(b.hits[0].line, 0);
        assert_eq!(r.total_hits, 4);
        assert!(!r.truncated);
    }

    #[test]
    fn options_regex_and_errors() {
        let d = ws("opts");
        let cs = FindOptions {
            case_sensitive: true,
            ..CI
        };
        assert_eq!(search_workspace(&d.0, "Hello", cs).total_hits, 1);
        let w = FindOptions {
            whole_word: true,
            ..CI
        };
        assert_eq!(search_workspace(&d.0, "ell", w).total_hits, 0);
        let re = FindOptions { regex: true, ..CI };
        assert_eq!(search_workspace(&d.0, r"h\w+o", re).total_hits, 5);
        let bad = search_workspace(&d.0, "(", re);
        assert!(bad.error.is_some() && bad.files.is_empty());
        assert!(search_workspace(&d.0, "", CI).files.is_empty());
    }

    #[test]
    fn unicode_columns_are_in_chars() {
        let d = ws("uni");
        std::fs::write(d.0.join("u.txt"), "日本語🌍 target\n").unwrap();
        let r = search_workspace(&d.0, "target", CI);
        let u = r.files.iter().find(|f| f.rel == "u.txt").unwrap();
        assert_eq!((u.hits[0].col, u.hits[0].len), (5, 6));
        assert_eq!(u.hits[0].preview, "日本語🌍 target");
    }

    #[test]
    fn hit_limit_truncates_and_says_so() {
        let d = ws("limit");
        std::fs::write(d.0.join("many.txt"), "x\n".repeat(MAX_HITS + 100)).unwrap();
        let r = search_workspace(&d.0, "x", CI);
        assert!(r.truncated);
        assert_eq!(r.total_hits, MAX_HITS);
    }

    #[test]
    fn very_long_lines_are_clipped_around_the_match() {
        let d = ws("long");
        let long = format!("{}needle{}", "a".repeat(500), "b".repeat(500));
        std::fs::write(d.0.join("long.txt"), long).unwrap();
        let r = search_workspace(&d.0, "needle", CI);
        let h = &r.files.iter().find(|f| f.rel == "long.txt").unwrap().hits[0];
        assert!(h.preview.chars().count() <= PREVIEW_CHARS + 6);
        assert_eq!(
            &h.preview.chars().collect::<Vec<_>>()[h.preview_range.clone()]
                .iter()
                .collect::<String>(),
            "needle"
        );
    }

    #[test]
    fn search_state_runs_and_toggles_files() {
        let d = ws("state");
        let mut s = SearchState::new();
        s.query.insert("hello");
        s.run(&d.0);
        let first = s.results.as_ref().unwrap().files[0].path.clone();
        s.toggle_file(&first);
        assert!(s.collapsed.contains(&first));
        s.toggle_file(&first);
        assert!(s.collapsed.is_empty());
        s.toggle_file(&first);
        s.run(&d.0);
        assert!(s.collapsed.is_empty(), "a new search resets collapsing");
    }
}
