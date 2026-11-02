use std::ops::Range;
use std::path::PathBuf;

use editor_core::Editor;
use editor_core::find::{
    FindOptions, Finder, MAX_MATCHES, index_of_selection, next_index, prev_index,
};

use crate::palette::TextField;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindFocus {
    Editor,
    Query,
    Replace,
}

pub struct FindState {
    pub query: TextField,
    pub replace: TextField,
    pub show_replace: bool,
    pub opts: FindOptions,
    pub focus: FindFocus,
    finder: Option<Finder>,
    pub error: Option<String>,
    matches: Vec<Range<usize>>,
    pub current: Option<usize>,
    pub truncated: bool,
    built_for: Option<(String, FindOptions)>,
    matched_for: Option<(Option<PathBuf>, u64)>,
}

impl FindState {
    pub fn new(show_replace: bool) -> Self {
        Self {
            query: TextField::default(),
            replace: TextField::default(),
            show_replace,
            opts: FindOptions::default(),
            focus: FindFocus::Query,
            finder: None,
            error: None,
            matches: Vec::new(),
            current: None,
            truncated: false,
            built_for: None,
            matched_for: None,
        }
    }

    pub fn matches(&self) -> &[Range<usize>] {
        &self.matches
    }

    pub fn toggle_case(&mut self) {
        self.opts.case_sensitive = !self.opts.case_sensitive;
    }

    pub fn toggle_whole_word(&mut self) {
        self.opts.whole_word = !self.opts.whole_word;
    }

    pub fn toggle_regex(&mut self) {
        self.opts.regex = !self.opts.regex;
    }

    pub fn refresh(&mut self, editor: &Editor, path: Option<&PathBuf>) {
        let key = (self.query.text().to_owned(), self.opts);
        let mut dirty = false;
        if self.built_for.as_ref() != Some(&key) {
            match Finder::new(&key.0, key.1) {
                Ok(f) => {
                    self.finder = f;
                    self.error = None;
                }
                Err(e) => {
                    self.finder = None;
                    self.error = Some(e.to_string());
                }
            }
            self.built_for = Some(key);
            dirty = true;
        }
        let stamp = (path.cloned(), editor.version());
        if dirty || self.matched_for.as_ref() != Some(&stamp) {
            self.matches = self
                .finder
                .as_ref()
                .map(|f| f.find_all(&editor.text()))
                .unwrap_or_default();
            self.truncated = self.matches.len() >= MAX_MATCHES;
            self.matched_for = Some(stamp);
        }
        self.current = index_of_selection(&self.matches, &editor.selection());
    }

    pub fn status(&self) -> String {
        if let Some(e) = &self.error {
            return e.clone();
        }
        if self.query.text().is_empty() {
            return String::new();
        }
        match (self.current, self.matches.len()) {
            (_, 0) => "No results".to_owned(),
            (Some(i), n) => format!("{} of {n}{}", i + 1, if self.truncated { "+" } else { "" }),
            (None, n) => format!("{n}{}", if self.truncated { "+" } else { "" }),
        }
    }

    fn select(&mut self, editor: &mut Editor, index: Option<usize>) {
        if let Some(i) = index {
            editor.select_range(self.matches[i].clone());
            self.current = Some(i);
        }
    }

    pub fn select_nearest(&mut self, editor: &mut Editor) {
        let from = editor.selection().start;
        let idx = next_index(&self.matches, from);
        self.select(editor, idx);
    }

    pub fn next(&mut self, editor: &mut Editor) {
        let pos = editor.selection().end.max(editor.cursor());
        // From a selected match, step past it; otherwise start at the cursor.
        let from = if self.current.is_some() {
            pos
        } else {
            editor.cursor()
        };
        let idx = next_index(&self.matches, from);
        self.select(editor, idx);
    }

    pub fn prev(&mut self, editor: &mut Editor) {
        let from = editor.selection().start;
        let idx = prev_index(&self.matches, from);
        self.select(editor, idx);
    }

    pub fn replace_current(&mut self, editor: &mut Editor) -> usize {
        let Some(finder) = self.finder.clone() else {
            return 0;
        };
        let sel = editor.selection();
        let n = if index_of_selection(&self.matches, &sel).is_some() {
            editor.replace_matches(&finder, self.replace.text(), Some(sel))
        } else {
            0
        };
        self.matched_for = None;
        self.refresh(editor, None);
        self.matched_for = None;
        let idx = next_index(&self.matches, editor.cursor());
        self.select(editor, idx);
        n
    }

    pub fn replace_all(&mut self, editor: &mut Editor) -> usize {
        let Some(finder) = self.finder.clone() else {
            return 0;
        };
        let n = editor.replace_matches(&finder, self.replace.text(), None);
        self.matched_for = None;
        n
    }
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;

    fn state(query: &str) -> FindState {
        let mut f = FindState::new(false);
        f.query.insert(query);
        f
    }

    #[test]
    fn refresh_computes_matches_and_status() {
        let ed = Editor::new("foo bar Foo");
        let mut f = state("foo");
        f.refresh(&ed, None);
        assert_eq!(f.matches(), [0..3, 8..11]);
        assert_eq!(f.status(), "2");
        f.toggle_case();
        f.refresh(&ed, None);
        assert_eq!(f.matches(), [0..3]);
        let mut none = state("zzz");
        none.refresh(&ed, None);
        assert_eq!(none.status(), "No results");
        let mut empty = state("");
        empty.refresh(&ed, None);
        assert_eq!(empty.status(), "");
        assert!(empty.matches().is_empty());
    }

    #[test]
    fn invalid_regex_shows_the_error_and_no_matches() {
        let ed = Editor::new("a(b");
        let mut f = state("(");
        f.toggle_regex();
        f.refresh(&ed, None);
        assert!(f.error.is_some());
        assert!(f.matches().is_empty());
        assert_eq!(f.status(), f.error.clone().unwrap());
        f.toggle_regex();
        f.refresh(&ed, None);
        assert_eq!(f.matches(), [1..2]);
        assert!(f.error.is_none());
    }

    #[test]
    fn matches_follow_text_edits_via_the_version() {
        let mut ed = Editor::new("ab ab");
        let mut f = state("ab");
        f.refresh(&ed, None);
        assert_eq!(f.matches().len(), 2);
        ed.set_cursor(5, false);
        ed.insert(" ab");
        f.refresh(&ed, None);
        assert_eq!(f.matches().len(), 3);
        ed.undo();
        f.refresh(&ed, None);
        assert_eq!(f.matches().len(), 2);
    }

    #[test]
    fn next_and_prev_select_matches_and_wrap() {
        let mut ed = Editor::new("x a x b x");
        let mut f = state("x");
        f.refresh(&ed, None);
        f.next(&mut ed);
        assert_eq!(ed.selection(), 0..1);
        f.refresh(&ed, None);
        assert_eq!(f.status(), "1 of 3");
        f.next(&mut ed);
        assert_eq!(ed.selection(), 4..5);
        f.next(&mut ed);
        f.next(&mut ed);
        assert_eq!(ed.selection(), 0..1, "wrapped around");
        f.prev(&mut ed);
        assert_eq!(ed.selection(), 8..9, "previous wraps to the last");
        f.prev(&mut ed);
        assert_eq!(ed.selection(), 4..5);
    }

    #[test]
    fn first_next_starts_from_the_cursor() {
        let mut ed = Editor::new("x a x b x");
        ed.set_cursor(3, false);
        let mut f = state("x");
        f.refresh(&ed, None);
        f.next(&mut ed);
        assert_eq!(ed.selection(), 4..5);
    }

    #[test]
    fn replace_current_replaces_the_selected_match_then_advances() {
        let mut ed = Editor::new("a1 a2 a3");
        let mut f = state("a");
        f.replace.insert("Z");
        f.refresh(&ed, None);
        f.next(&mut ed);
        assert_eq!(f.replace_current(&mut ed), 1);
        assert_eq!(ed.text(), "Z1 a2 a3");
        assert_eq!(ed.selection(), 3..4, "now on the next match");
        assert_eq!(f.replace_current(&mut ed), 1);
        assert_eq!(f.replace_current(&mut ed), 1);
        assert_eq!(ed.text(), "Z1 Z2 Z3");
        let mut ed2 = Editor::new("q q");
        let mut f2 = state("q");
        f2.replace.insert("w");
        f2.refresh(&ed2, None);
        assert_eq!(f2.replace_current(&mut ed2), 0);
        assert_eq!(ed2.text(), "q q");
        assert_eq!(ed2.selection(), 0..1);
    }

    #[test]
    fn replace_all_is_one_undo_step_and_handles_regex_captures() {
        let mut ed = Editor::new("k1=v1 k2=v2");
        let mut f = state(r"(\w+)=(\w+)");
        f.toggle_regex();
        f.replace.insert("$2=$1");
        f.refresh(&ed, None);
        assert_eq!(f.replace_all(&mut ed), 2);
        assert_eq!(ed.text(), "v1=k1 v2=k2");
        assert!(ed.undo());
        assert_eq!(ed.text(), "k1=v1 k2=v2");
    }

    #[test]
    fn replace_without_a_valid_pattern_does_nothing() {
        let mut ed = Editor::new("abc");
        let mut f = state("(");
        f.toggle_regex();
        f.refresh(&ed, None);
        assert_eq!(f.replace_all(&mut ed), 0);
        assert_eq!(f.replace_current(&mut ed), 0);
        assert_eq!(ed.text(), "abc");
    }

    #[test]
    fn switching_files_recomputes_even_at_the_same_version() {
        let a = Editor::new("foo");
        let b = Editor::new("foo foo");
        let mut f = state("foo");
        f.refresh(&a, Some(&PathBuf::from("a")));
        assert_eq!(f.matches().len(), 1);
        f.refresh(&b, Some(&PathBuf::from("b")));
        assert_eq!(f.matches().len(), 2, "same version number, different file");
    }
}
