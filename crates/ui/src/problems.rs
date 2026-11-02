use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use syntax::{Diagnostic, Language};

use crate::model::{Tab, TabContent};

pub const DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemFile {
    pub path: PathBuf,
    pub title: String,
    pub items: Vec<Diagnostic>,
}

struct Entry {
    version: u64,
    seen: u64,
    seen_at: Instant,
    items: Vec<Diagnostic>,
}

#[derive(Default)]
pub struct Problems {
    cache: HashMap<PathBuf, Entry>,
    pub files: Vec<ProblemFile>,
}

impl Problems {
    pub fn total(&self) -> usize {
        self.files.iter().map(|f| f.items.len()).sum()
    }

    pub fn refresh(&mut self, tabs: &[Tab], now: Instant, debounce: Duration) -> bool {
        let mut files = Vec::new();
        let mut live: Vec<&PathBuf> = Vec::new();
        for t in tabs {
            let TabContent::Text(e) = &t.content else {
                continue;
            };
            let Some(lang) = Language::from_path(&t.path) else {
                continue;
            };
            live.push(&t.path);
            let version = e.buffer().version();
            let entry = self.cache.entry(t.path.clone()).or_insert_with(|| Entry {
                version: u64::MAX,
                seen: version,
                seen_at: now.checked_sub(debounce).unwrap_or(now),
                items: Vec::new(),
            });
            if entry.seen != version {
                entry.seen = version;
                entry.seen_at = now;
            }
            if entry.version != version && now.duration_since(entry.seen_at) >= debounce {
                entry.items = syntax::diagnostics(lang, &e.text());
                entry.version = version;
            }
            if !entry.items.is_empty() {
                files.push(ProblemFile {
                    path: t.path.clone(),
                    title: t.title.clone(),
                    items: entry.items.clone(),
                });
            }
        }
        self.cache.retain(|p, _| live.contains(&p));
        let changed = files != self.files;
        self.files = files;
        changed
    }

    pub fn update_lsp_diagnostics(
        &mut self,
        path: PathBuf,
        title: String,
        items: Vec<Diagnostic>,
    ) -> bool {
        let entry = self.cache.entry(path.clone()).or_insert_with(|| Entry {
            version: 0,
            seen: 0,
            seen_at: Instant::now(),
            items: Vec::new(),
        });
        entry.items = items.clone();
        if items.is_empty() {
            self.files.retain(|f| f.path != path);
        } else if let Some(f) = self.files.iter_mut().find(|f| f.path == path) {
            f.items = items;
        } else {
            self.files.push(ProblemFile { path, title, items });
        }
        true
    }
}
