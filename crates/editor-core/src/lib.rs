//! Text buffer with undo/redo and dirty tracking (ED-04, CT-001).
//!
//! Indices are `char` offsets (Unicode scalar values), not bytes.

use ropey::Rope;

#[derive(Debug, Clone)]
enum Edit {
    Insert { at: usize, text: String },
    Delete { at: usize, text: String },
}

#[derive(Debug, Default)]
pub struct Buffer {
    rope: Rope,
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    /// Length of `undo` when the buffer was last saved; `None` if that state is unreachable.
    saved_at: Option<usize>,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            saved_at: Some(0),
            ..Self::default()
        }
    }

    pub fn from_text(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            saved_at: Some(0),
            ..Self::default()
        }
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn is_dirty(&self) -> bool {
        self.saved_at != Some(self.undo.len())
    }

    /// Mark current content as persisted to disk.
    pub fn mark_saved(&mut self) {
        self.saved_at = Some(self.undo.len());
    }

    pub fn insert(&mut self, at: usize, text: &str) {
        assert!(at <= self.len_chars(), "insert index out of bounds");
        if text.is_empty() {
            return;
        }
        self.rope.insert(at, text);
        self.push_edit(Edit::Insert {
            at,
            text: text.to_owned(),
        });
    }

    pub fn delete(&mut self, range: std::ops::Range<usize>) {
        assert!(
            range.start <= range.end && range.end <= self.len_chars(),
            "delete range out of bounds"
        );
        if range.is_empty() {
            return;
        }
        let removed = self.rope.slice(range.clone()).to_string();
        self.rope.remove(range.clone());
        self.push_edit(Edit::Delete {
            at: range.start,
            text: removed,
        });
    }

    pub fn undo(&mut self) -> bool {
        let Some(edit) = self.undo.pop() else {
            return false;
        };
        match &edit {
            Edit::Insert { at, text } => self.rope.remove(*at..*at + text.chars().count()),
            Edit::Delete { at, text } => self.rope.insert(*at, text),
        }
        self.redo.push(edit);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(edit) = self.redo.pop() else {
            return false;
        };
        match &edit {
            Edit::Insert { at, text } => self.rope.insert(*at, text),
            Edit::Delete { at, text } => self.rope.remove(*at..*at + text.chars().count()),
        }
        self.undo.push(edit);
        true
    }

    fn push_edit(&mut self, edit: Edit) {
        // A new edit drops the redo branch; a saved state living in it becomes unreachable.
        if !self.redo.is_empty() {
            self.redo.clear();
            if self.saved_at.is_some_and(|s| s > self.undo.len()) {
                self.saved_at = None;
            }
        }
        self.undo.push(edit);
    }
}

impl std::fmt::Display for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for chunk in self.rope.chunks() {
            f.write_str(chunk)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_delete_roundtrip_with_unicode() {
        let mut b = Buffer::from_text("héllo 🌍\r\nworld");
        b.insert(6, "日本");
        assert_eq!(b.to_string(), "héllo 日本🌍\r\nworld");
        b.delete(6..8);
        assert_eq!(b.to_string(), "héllo 🌍\r\nworld");
    }

    #[test]
    fn undo_redo_is_deterministic() {
        let mut b = Buffer::from_text("abc");
        b.insert(3, "def");
        b.delete(0..2);
        assert_eq!(b.to_string(), "cdef");
        assert!(b.undo());
        assert_eq!(b.to_string(), "abcdef");
        assert!(b.undo());
        assert_eq!(b.to_string(), "abc");
        assert!(!b.undo());
        assert!(b.redo());
        assert!(b.redo());
        assert_eq!(b.to_string(), "cdef");
        assert!(!b.redo());
    }

    #[test]
    fn dirty_tracks_saved_state() {
        let mut b = Buffer::from_text("x");
        assert!(!b.is_dirty());
        b.insert(1, "y");
        assert!(b.is_dirty());
        b.mark_saved();
        assert!(!b.is_dirty());
        b.undo();
        assert!(b.is_dirty());
        b.redo();
        assert!(!b.is_dirty());
    }

    #[test]
    fn saved_state_lost_after_branching_edit() {
        let mut b = Buffer::from_text("x");
        b.insert(1, "y");
        b.mark_saved();
        b.undo();
        b.insert(1, "z");
        b.undo();
        b.redo();
        assert!(b.is_dirty());
    }
}
