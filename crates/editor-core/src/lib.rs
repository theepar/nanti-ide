//! Text buffer with grouped undo/redo, dirty tracking, and an `Editor` (cursor + selection) on top
//! (ED-04, CT-001).
//!
//! Indices are `char` offsets (Unicode scalar values), not bytes. Cursor movement and deletion
//! work on grapheme clusters, so emoji sequences and `\r\n` behave as one unit.

use std::ops::Range;

mod edit_ops;
pub mod find;

use ropey::Rope;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone)]
enum Edit {
    Insert { at: usize, text: String },
    Delete { at: usize, text: String },
}

impl Edit {
    fn apply(&self, rope: &mut Rope) {
        match self {
            Edit::Insert { at, text } => rope.insert(*at, text),
            Edit::Delete { at, text } => rope.remove(*at..*at + text.chars().count()),
        }
    }

    fn apply_inverse(&self, rope: &mut Rope) {
        match self {
            Edit::Insert { at, text } => rope.remove(*at..*at + text.chars().count()),
            Edit::Delete { at, text } => rope.insert(*at, text),
        }
    }
}

/// A set of edits undone/redone together.
type Group = Vec<Edit>;

#[derive(Debug, Default)]
pub struct Buffer {
    rope: Rope,
    undo: Vec<Group>,
    redo: Vec<Group>,
    /// Edits of the transaction currently being recorded, if any.
    open: Option<Group>,
    /// Length of `undo` when the buffer was last saved; `None` if that state is unreachable.
    saved_at: Option<usize>,
    /// Bumped on every content change (insert, delete, undo, redo, composition rewind).
    version: u64,
}

fn is_line_break(c: char) -> bool {
    matches!(
        c,
        '\n' | '\r' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
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

    /// Changes whenever the text changes; use it to invalidate derived data (highlights, matches).
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Number of lines; a trailing line terminator starts one more (empty) line.
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn is_dirty(&self) -> bool {
        self.saved_at != Some(self.undo.len()) || self.open.as_ref().is_some_and(|g| !g.is_empty())
    }

    /// Mark current content as persisted to disk.
    pub fn mark_saved(&mut self) {
        self.saved_at = Some(self.undo.len());
    }

    /// Offset conversions for platform text services, which count UTF-16 code units.
    pub fn char_to_utf16(&self, char_idx: usize) -> usize {
        self.rope.char_to_utf16_cu(char_idx.min(self.len_chars()))
    }

    pub fn utf16_to_char(&self, utf16_idx: usize) -> usize {
        self.rope
            .utf16_cu_to_char(utf16_idx.min(self.rope.len_utf16_cu()))
    }

    /// Start an undo group that stays open across calls (an IME composition). Edits are recorded
    /// into it until `close_group`, which commits them as ONE undo step.
    pub fn open_group(&mut self) {
        if self.open.is_none() {
            self.open = Some(Vec::new());
        }
    }

    pub fn close_group(&mut self) {
        if let Some(group) = self.open.take() {
            if !group.is_empty() {
                self.commit(group);
            }
        }
    }

    /// Revert the edits recorded in the open group so far (keeping the group open). Used to redo a
    /// composition update from the pre-composition state without polluting undo history.
    pub fn rewind_open_group(&mut self) {
        if let Some(group) = self.open.as_mut() {
            for edit in group.drain(..).rev() {
                edit.apply_inverse(&mut self.rope);
                self.version += 1;
            }
        }
    }

    pub fn slice(&self, range: Range<usize>) -> String {
        self.rope.slice(range).to_string()
    }

    pub fn line_start(&self, line: usize) -> usize {
        self.rope.line_to_char(line)
    }

    /// Char index of the end of `line`, excluding its terminator.
    pub fn line_end(&self, line: usize) -> usize {
        let l = self.rope.line(line);
        let mut n = l.len_chars();
        if n > 0 && l.char(n - 1) == '\n' && n > 1 && l.char(n - 2) == '\r' {
            n -= 2;
        } else if n > 0 && is_line_break(l.char(n - 1)) {
            n -= 1;
        }
        self.rope.line_to_char(line) + n
    }

    pub fn line_of(&self, char_idx: usize) -> usize {
        self.rope.char_to_line(char_idx)
    }

    /// Text of `line` without its terminator.
    pub fn line_text(&self, line: usize) -> String {
        self.slice(self.line_start(line)..self.line_end(line))
    }

    pub fn insert(&mut self, at: usize, text: &str) {
        assert!(at <= self.len_chars(), "insert index out of bounds");
        if text.is_empty() {
            return;
        }
        self.rope.insert(at, text);
        self.version += 1;
        self.push_edit(Edit::Insert {
            at,
            text: text.to_owned(),
        });
    }

    /// Like `insert`, but extends the previous undo step when it was a single insert that ends
    /// exactly at `at` (so a run of typed characters undoes as one step). Never coalesces across a
    /// save point, which would make a modified buffer look clean.
    pub fn insert_coalescing(&mut self, at: usize, text: &str) {
        let can_merge = self.open.is_none()
            && self.redo.is_empty()
            && self.saved_at != Some(self.undo.len())
            && matches!(
                self.undo.last().map(Vec::as_slice),
                Some([Edit::Insert { at: a, text: t }]) if a + t.chars().count() == at
            );
        if !can_merge {
            return self.insert(at, text);
        }
        assert!(at <= self.len_chars(), "insert index out of bounds");
        self.rope.insert(at, text);
        self.version += 1;
        if let Some([Edit::Insert { text: t, .. }]) = self.undo.last_mut().map(Vec::as_mut_slice) {
            t.push_str(text);
        }
    }

    pub fn delete(&mut self, range: Range<usize>) {
        assert!(
            range.start <= range.end && range.end <= self.len_chars(),
            "delete range out of bounds"
        );
        if range.is_empty() {
            return;
        }
        let removed = self.slice(range.clone());
        self.rope.remove(range.clone());
        self.version += 1;
        self.push_edit(Edit::Delete {
            at: range.start,
            text: removed,
        });
    }

    /// Run `f`; every edit it makes becomes one undo step.
    pub fn transact<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let nested = self.open.is_some();
        if !nested {
            self.open = Some(Vec::new());
        }
        let r = f(self);
        if !nested {
            if let Some(group) = self.open.take() {
                if !group.is_empty() {
                    self.commit(group);
                }
            }
        }
        r
    }

    /// Undo one step. Returns where the cursor should go, or `None` if nothing to undo.
    pub fn undo(&mut self) -> Option<usize> {
        let group = self.undo.pop()?;
        let mut hint = 0;
        for edit in group.iter().rev() {
            edit.apply_inverse(&mut self.rope);
            hint = match edit {
                Edit::Insert { at, .. } => *at,
                Edit::Delete { at, text } => at + text.chars().count(),
            };
        }
        self.redo.push(group);
        self.version += 1;
        Some(hint)
    }

    /// Redo one step. Returns where the cursor should go, or `None` if nothing to redo.
    pub fn redo(&mut self) -> Option<usize> {
        let group = self.redo.pop()?;
        let mut hint = 0;
        for edit in &group {
            edit.apply(&mut self.rope);
            hint = match edit {
                Edit::Insert { at, text } => at + text.chars().count(),
                Edit::Delete { at, .. } => *at,
            };
        }
        self.undo.push(group);
        self.version += 1;
        Some(hint)
    }

    fn push_edit(&mut self, edit: Edit) {
        match &mut self.open {
            Some(group) => group.push(edit),
            None => self.commit(vec![edit]),
        }
    }

    fn commit(&mut self, group: Group) {
        // A new edit drops the redo branch; a saved state living in it becomes unreachable.
        if !self.redo.is_empty() {
            self.redo.clear();
            if self.saved_at.is_some_and(|s| s > self.undo.len()) {
                self.saved_at = None;
            }
        }
        self.undo.push(group);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eol {
    Lf,
    CrLf,
}

impl Eol {
    pub fn as_str(self) -> &'static str {
        match self {
            Eol::Lf => "\n",
            Eol::CrLf => "\r\n",
        }
    }
}

/// How many chars around the cursor are inspected to find grapheme boundaries.
const GRAPHEME_WINDOW: usize = 32;

/// An IME composition in progress: `origin` is what the composition replaced (restored on every
/// update, so intermediate states never reach the undo history), `marked` the pre-edit text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Composition {
    origin: Range<usize>,
    marked: Range<usize>,
}

/// Buffer + cursor + selection. The selection is `anchor..cursor` (either order).
#[derive(Debug)]
pub struct Editor {
    buffer: Buffer,
    cursor: usize,
    anchor: usize,
    /// Column to return to on vertical moves through shorter lines.
    goal_col: Option<usize>,
    eol: Eol,
    /// True while consecutive typing should merge into one undo step.
    typing_run: bool,
    composition: Option<Composition>,
}

impl Editor {
    pub fn new(text: &str) -> Self {
        let eol = if text.contains("\r\n") {
            Eol::CrLf
        } else {
            Eol::Lf
        };
        Self {
            buffer: Buffer::from_text(text),
            cursor: 0,
            anchor: 0,
            goal_col: None,
            eol,
            typing_run: false,
            composition: None,
        }
    }

    // ---- IME composition -------------------------------------------------------------------

    /// Char range of the text being composed (underlined pre-edit text), if any.
    pub fn marked_range(&self) -> Option<Range<usize>> {
        self.composition.as_ref().map(|c| c.marked.clone())
    }

    pub fn is_composing(&self) -> bool {
        self.composition.is_some()
    }

    fn clamp_range(&self, r: Range<usize>) -> Range<usize> {
        let n = self.buffer.len_chars();
        r.start.min(n)..r.end.min(n)
    }

    /// Replace `range` (default: the composition's origin, else the selection) with pre-edit
    /// `text` and mark it as composing. `selected` is the cursor/selection inside `text`, in chars.
    /// The whole composition is a single undo step, committed when it ends.
    pub fn compose(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
    ) {
        self.typing_run = false;
        let text = self.normalize_eol(text);
        let target = match self.composition.take() {
            Some(c) => {
                self.buffer.rewind_open_group();
                c.origin
            }
            None => {
                self.buffer.open_group();
                match range {
                    Some(r) => self.clamp_range(r),
                    None => self.selection(),
                }
            }
        };
        let start = target.start;
        let len = text.chars().count();
        self.buffer.delete(target.clone());
        self.buffer.insert(start, &text);
        self.composition = Some(Composition {
            origin: target,
            marked: start..start + len,
        });
        let sel = selected.unwrap_or(len..len);
        self.anchor = start + sel.start.min(len);
        self.cursor = start + sel.end.min(len);
        self.goal_col = None;
    }

    /// Finish composing (the platform committed or abandoned the pre-edit text); text stays.
    pub fn end_composition(&mut self) {
        if self.composition.take().is_some() {
            self.buffer.close_group();
        }
    }

    /// Replace `range` (default: composition origin, else selection) with final `text`, ending any
    /// composition. This is what the IME calls with the committed result.
    pub fn commit_text(&mut self, range: Option<Range<usize>>, text: &str) {
        let text = self.normalize_eol(text);
        let target = match self.composition.take() {
            Some(c) => {
                self.buffer.rewind_open_group();
                c.origin
            }
            None => {
                self.buffer.open_group();
                match range {
                    Some(r) => self.clamp_range(r),
                    None => self.selection(),
                }
            }
        };
        let start = target.start;
        self.buffer.delete(target);
        self.buffer.insert(start, &text);
        self.buffer.close_group();
        self.cursor = start + text.chars().count();
        self.anchor = self.cursor;
        self.typing_run = false;
        self.goal_col = None;
    }

    /// UTF-16 selection `(range, reversed)` as platform text services expect it.
    pub fn selection_utf16(&self) -> (Range<usize>, bool) {
        (
            self.chars_range_to_utf16(self.selection()),
            self.cursor < self.anchor,
        )
    }

    pub fn marked_utf16(&self) -> Option<Range<usize>> {
        self.marked_range().map(|r| self.chars_range_to_utf16(r))
    }

    pub fn utf16_range_to_chars(&self, r: Range<usize>) -> Range<usize> {
        self.buffer.utf16_to_char(r.start)..self.buffer.utf16_to_char(r.end)
    }

    pub fn chars_range_to_utf16(&self, r: Range<usize>) -> Range<usize> {
        self.buffer.char_to_utf16(r.start)..self.buffer.char_to_utf16(r.end)
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn text(&self) -> String {
        self.buffer.to_string()
    }

    pub fn version(&self) -> u64 {
        self.buffer.version()
    }

    pub fn eol(&self) -> Eol {
        self.eol
    }

    pub fn is_dirty(&self) -> bool {
        self.buffer.is_dirty()
    }

    pub fn mark_saved(&mut self) {
        self.buffer.mark_saved();
        self.typing_run = false;
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn selection(&self) -> Range<usize> {
        self.cursor.min(self.anchor)..self.cursor.max(self.anchor)
    }

    pub fn has_selection(&self) -> bool {
        self.cursor != self.anchor
    }

    pub fn selected_text(&self) -> String {
        self.buffer.slice(self.selection())
    }

    /// `(line, column)` of the cursor, both 0-based, column in chars.
    pub fn line_col(&self) -> (usize, usize) {
        let line = self.buffer.line_of(self.cursor);
        (line, self.cursor - self.buffer.line_start(line))
    }

    // ---- editing -------------------------------------------------------------------------

    /// Replace the selection with `text` (line breaks are converted to the file's EOL).
    pub fn insert(&mut self, text: &str) {
        self.end_composition();
        let text = self.normalize_eol(text);
        if text.is_empty() {
            return;
        }
        let single_char =
            !self.has_selection() && text.chars().count() == 1 && !text.contains('\n');
        if single_char {
            let at = self.cursor;
            if self.typing_run {
                self.buffer.insert_coalescing(at, &text);
            } else {
                self.buffer.insert(at, &text);
            }
            self.typing_run = true;
            self.cursor = at + 1;
        } else {
            self.typing_run = false;
            let sel = self.selection();
            let start = sel.start;
            let added = text.chars().count();
            self.buffer.transact(|b| {
                b.delete(sel);
                b.insert(start, &text);
            });
            self.cursor = start + added;
        }
        self.anchor = self.cursor;
        self.goal_col = None;
    }

    pub fn newline(&mut self) {
        let eol = self.eol.as_str();
        self.insert(eol);
    }

    pub fn backspace(&mut self) {
        self.end_composition();
        self.typing_run = false;
        if self.has_selection() {
            return self.delete_selection();
        }
        if self.cursor == 0 {
            return;
        }
        let start = self.prev_boundary(self.cursor);
        self.buffer.delete(start..self.cursor);
        self.cursor = start;
        self.anchor = start;
        self.goal_col = None;
    }

    pub fn delete_forward(&mut self) {
        self.end_composition();
        self.typing_run = false;
        if self.has_selection() {
            return self.delete_selection();
        }
        if self.cursor >= self.buffer.len_chars() {
            return;
        }
        let end = self.next_boundary(self.cursor);
        self.buffer.delete(self.cursor..end);
        self.goal_col = None;
    }

    /// Delete the selection and return its text (for cut).
    pub fn cut(&mut self) -> String {
        self.end_composition();
        let text = self.selected_text();
        self.delete_selection();
        text
    }

    fn delete_selection(&mut self) {
        let sel = self.selection();
        if sel.is_empty() {
            return;
        }
        self.buffer.delete(sel.clone());
        self.cursor = sel.start;
        self.anchor = sel.start;
        self.goal_col = None;
    }

    pub fn undo(&mut self) -> bool {
        self.end_composition();
        self.typing_run = false;
        match self.buffer.undo() {
            Some(pos) => {
                self.cursor = pos.min(self.buffer.len_chars());
                self.anchor = self.cursor;
                self.goal_col = None;
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        self.end_composition();
        self.typing_run = false;
        match self.buffer.redo() {
            Some(pos) => {
                self.cursor = pos.min(self.buffer.len_chars());
                self.anchor = self.cursor;
                self.goal_col = None;
                true
            }
            None => false,
        }
    }

    /// Select `range` (anchor at its start) so it is highlighted and replaced by typing.
    pub fn select_range(&mut self, range: Range<usize>) {
        self.set_cursor(range.start, false);
        self.set_cursor(range.end, true);
    }

    /// Replace matches of `finder` with `replacement`: every match, or only the one equal to
    /// `only` (a char range). One undo step. Returns how many were replaced.
    pub fn replace_matches(
        &mut self,
        finder: &find::Finder,
        replacement: &str,
        only: Option<Range<usize>>,
    ) -> usize {
        self.end_composition();
        let text = self.text();
        let mut matches = finder.find_all(&text);
        if let Some(o) = &only {
            matches.retain(|m| m == o);
        }
        if matches.is_empty() {
            return 0;
        }
        // Walk the text once to get each match start as a byte offset (needed for capture expansion).
        let mut reps: Vec<(Range<usize>, String)> = Vec::with_capacity(matches.len());
        let (mut chars, mut bytes) = (0usize, 0usize);
        for m in &matches {
            for c in text[bytes..].chars().take(m.start - chars) {
                bytes += c.len_utf8();
            }
            chars = m.start;
            let rep = finder.replacement(&text, bytes, replacement);
            reps.push((m.clone(), self.normalize_eol(&rep)));
        }
        let mut shift: isize = 0;
        for (r, rep) in &reps[..reps.len() - 1] {
            shift += rep.chars().count() as isize - (r.end - r.start) as isize;
        }
        let (last_range, last_rep) = &reps[reps.len() - 1];
        let new_cursor = (last_range.start as isize + shift) as usize + last_rep.chars().count();
        self.buffer.transact(|b| {
            for (r, rep) in reps.iter().rev() {
                b.delete(r.clone());
                b.insert(r.start, rep);
            }
        });
        self.cursor = new_cursor.min(self.buffer.len_chars());
        self.anchor = self.cursor;
        self.typing_run = false;
        self.goal_col = None;
        reps.len()
    }

    // ---- movement ------------------------------------------------------------------------

    /// Move the cursor to `pos`; with `extend` the anchor stays so the selection grows.
    pub fn set_cursor(&mut self, pos: usize, extend: bool) {
        self.end_composition();
        self.typing_run = false;
        self.cursor = pos.min(self.buffer.len_chars());
        if !extend {
            self.anchor = self.cursor;
        }
    }

    pub fn move_left(&mut self, extend: bool) {
        self.goal_col = None;
        if !extend && self.has_selection() {
            return self.set_cursor(self.selection().start, false);
        }
        let to = if self.cursor == 0 {
            0
        } else {
            self.prev_boundary(self.cursor)
        };
        self.set_cursor(to, extend);
    }

    pub fn move_right(&mut self, extend: bool) {
        self.goal_col = None;
        if !extend && self.has_selection() {
            return self.set_cursor(self.selection().end, false);
        }
        let to = if self.cursor >= self.buffer.len_chars() {
            self.cursor
        } else {
            self.next_boundary(self.cursor)
        };
        self.set_cursor(to, extend);
    }

    pub fn move_up(&mut self, extend: bool) {
        self.move_vertical(-1, extend);
    }

    pub fn move_down(&mut self, extend: bool) {
        self.move_vertical(1, extend);
    }

    fn move_vertical(&mut self, delta: isize, extend: bool) {
        let (line, col) = self.line_col();
        let goal = *self.goal_col.get_or_insert(col);
        let target = line as isize + delta;
        if target < 0 {
            return self.set_cursor(0, extend);
        }
        let target = target as usize;
        if target >= self.buffer.len_lines() {
            return self.set_cursor(self.buffer.len_chars(), extend);
        }
        let start = self.buffer.line_start(target);
        let end = self.buffer.line_end(target);
        let goal_keep = self.goal_col;
        self.set_cursor((start + goal).min(end), extend);
        self.goal_col = goal_keep;
    }

    pub fn move_line_start(&mut self, extend: bool) {
        self.goal_col = None;
        let line = self.buffer.line_of(self.cursor);
        self.set_cursor(self.buffer.line_start(line), extend);
    }

    pub fn move_line_end(&mut self, extend: bool) {
        self.goal_col = None;
        let line = self.buffer.line_of(self.cursor);
        self.set_cursor(self.buffer.line_end(line), extend);
    }

    pub fn move_doc_start(&mut self, extend: bool) {
        self.goal_col = None;
        self.set_cursor(0, extend);
    }

    pub fn move_doc_end(&mut self, extend: bool) {
        self.goal_col = None;
        self.set_cursor(self.buffer.len_chars(), extend);
    }

    pub fn select_all(&mut self) {
        self.end_composition();
        self.typing_run = false;
        self.anchor = 0;
        self.cursor = self.buffer.len_chars();
    }

    // ---- helpers -------------------------------------------------------------------------

    fn normalize_eol(&self, text: &str) -> String {
        if !text.contains(['\r', '\n']) {
            return text.to_owned();
        }
        let unified = text.replace("\r\n", "\n").replace('\r', "\n");
        match self.eol {
            Eol::Lf => unified,
            Eol::CrLf => unified.replace('\n', "\r\n"),
        }
    }

    fn prev_boundary(&self, pos: usize) -> usize {
        let from = pos.saturating_sub(GRAPHEME_WINDOW);
        let window = self.buffer.slice(from..pos);
        let last = window
            .graphemes(true)
            .next_back()
            .map_or(1, |g| g.chars().count());
        pos - last
    }

    fn next_boundary(&self, pos: usize) -> usize {
        let to = (pos + GRAPHEME_WINDOW).min(self.buffer.len_chars());
        let window = self.buffer.slice(pos..to);
        let first = window
            .graphemes(true)
            .next()
            .map_or(1, |g| g.chars().count());
        pos + first
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Buffer ----

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
        assert!(b.undo().is_some());
        assert_eq!(b.to_string(), "abcdef");
        assert!(b.undo().is_some());
        assert_eq!(b.to_string(), "abc");
        assert!(b.undo().is_none());
        assert!(b.redo().is_some());
        assert!(b.redo().is_some());
        assert_eq!(b.to_string(), "cdef");
        assert!(b.redo().is_none());
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

    #[test]
    fn transaction_undoes_as_one_step() {
        let mut b = Buffer::from_text("hello world");
        b.transact(|b| {
            b.delete(0..5);
            b.insert(0, "bye");
        });
        assert_eq!(b.to_string(), "bye world");
        assert_eq!(b.undo(), Some(5)); // cursor lands after the restored text
        assert_eq!(b.to_string(), "hello world");
        assert!(b.undo().is_none());
        b.redo();
        assert_eq!(b.to_string(), "bye world");
    }

    #[test]
    fn coalescing_merges_typing_but_never_across_a_save() {
        let mut b = Buffer::from_text("");
        b.insert_coalescing(0, "a"); // no previous group: plain insert
        b.insert_coalescing(1, "b");
        b.insert_coalescing(2, "c");
        assert_eq!(b.to_string(), "abc");
        b.undo();
        assert_eq!(b.to_string(), "");
        b.redo();
        b.mark_saved();
        b.insert_coalescing(3, "d"); // must not merge into the saved step
        assert!(b.is_dirty());
        b.undo();
        assert_eq!(b.to_string(), "abc");
        assert!(!b.is_dirty());
    }

    #[test]
    fn line_helpers_exclude_terminators() {
        let b = Buffer::from_text("ab\r\ncde\nf");
        assert_eq!(b.len_lines(), 3);
        assert_eq!(b.line_text(0), "ab");
        assert_eq!(b.line_text(1), "cde");
        assert_eq!(b.line_text(2), "f");
        assert_eq!(b.line_end(0) - b.line_start(0), 2);
        assert_eq!(b.line_of(b.line_start(2)), 2);
    }

    // ---- Editor ----

    fn ed(text: &str, cursor: usize) -> Editor {
        let mut e = Editor::new(text);
        e.set_cursor(cursor, false);
        e
    }

    #[test]
    fn typing_advances_cursor_and_undoes_in_one_step() {
        let mut e = ed("", 0);
        for c in ["h", "i", "!"] {
            e.insert(c);
        }
        assert_eq!((e.text().as_str(), e.cursor()), ("hi!", 3));
        assert!(e.undo());
        assert_eq!((e.text().as_str(), e.cursor()), ("", 0));
        assert!(e.redo());
        assert_eq!((e.text().as_str(), e.cursor()), ("hi!", 3));
    }

    #[test]
    fn typing_over_selection_replaces_it_as_one_undo_step() {
        let mut e = ed("hello world", 0);
        e.set_cursor(5, true); // select "hello"
        e.insert("bye");
        assert_eq!(e.text(), "bye world");
        assert!(e.undo());
        assert_eq!(e.text(), "hello world");
        assert!(!e.undo());
    }

    #[test]
    fn backspace_and_delete_remove_whole_graphemes() {
        let family = "👨‍👩‍👧"; // 5 scalar values, one grapheme
        let mut e = ed(&format!("a{family}b"), 6);
        e.move_left(false); // jumps over the whole 5-scalar cluster
        assert_eq!(e.cursor(), 1);
        e.move_right(false);
        assert_eq!(e.cursor(), 6);
        e.backspace(); // removes the whole cluster at once
        assert_eq!(e.text(), "ab");
        let mut e = ed("e\u{301}x", 0); // 'e' + combining acute
        e.delete_forward();
        assert_eq!(e.text(), "x");
    }

    #[test]
    fn crlf_is_one_unit_and_newline_keeps_file_eol() {
        let mut e = ed("a\r\nb", 3);
        assert_eq!(e.eol(), Eol::CrLf);
        e.backspace(); // removes the whole \r\n
        assert_eq!(e.text(), "ab");
        e.newline();
        assert_eq!(e.text(), "a\r\nb");
        e.insert("x\ny"); // pasted LF becomes CRLF
        assert_eq!(e.text(), "a\r\nx\r\nyb");
        let mut lf = ed("a", 1);
        lf.newline();
        assert_eq!(lf.text(), "a\n");
    }

    #[test]
    fn emoji_and_cjk_roundtrip() {
        let mut e = ed("", 0);
        e.insert("日本語🌍");
        assert_eq!(e.cursor(), 4);
        e.move_left(false);
        e.backspace();
        assert_eq!(e.text(), "日本🌍");
    }

    #[test]
    fn vertical_movement_keeps_goal_column() {
        let mut e = ed("abcdef\nab\nabcdef", 5);
        e.move_down(false);
        assert_eq!(e.line_col(), (1, 2)); // clamped to the short line
        e.move_down(false);
        assert_eq!(e.line_col(), (2, 5)); // goal column restored
        e.move_up(false);
        e.move_up(false);
        assert_eq!(e.line_col(), (0, 5));
        e.move_up(false);
        assert_eq!(e.cursor(), 0);
    }

    #[test]
    fn selection_extends_and_collapses() {
        let mut e = ed("abcdef", 2);
        e.move_right(true);
        e.move_right(true);
        assert_eq!(e.selection(), 2..4);
        assert_eq!(e.selected_text(), "cd");
        e.move_left(false); // collapse to start
        assert_eq!((e.cursor(), e.has_selection()), (2, false));
        e.select_all();
        assert_eq!(e.selected_text(), "abcdef");
        assert_eq!(e.cut(), "abcdef");
        assert_eq!(e.text(), "");
        assert!(e.undo());
        assert_eq!(e.text(), "abcdef");
    }

    #[test]
    fn home_end_and_doc_bounds() {
        let mut e = ed("ab\ncd", 4);
        e.move_line_start(false);
        assert_eq!(e.cursor(), 3);
        e.move_line_end(false);
        assert_eq!(e.cursor(), 5);
        e.move_doc_start(true);
        assert_eq!(e.selection(), 0..5);
        e.move_doc_end(false);
        assert_eq!(e.cursor(), 5);
    }

    #[test]
    fn dirty_follows_edits_undo_and_save() {
        let mut e = ed("x", 1);
        assert!(!e.is_dirty());
        e.insert("a");
        assert!(e.is_dirty());
        e.mark_saved();
        assert!(!e.is_dirty());
        e.insert("b"); // typing right after a save must not merge into the saved step
        assert!(e.is_dirty());
        e.undo();
        assert!(!e.is_dirty());
        assert_eq!(e.text(), "xa");
    }

    #[test]
    fn edge_cases_do_not_panic() {
        let mut e = ed("", 0);
        e.backspace();
        e.delete_forward();
        e.move_left(false);
        e.move_right(false);
        e.move_up(false);
        e.move_down(false);
        e.insert("");
        assert!(!e.undo());
        assert!(!e.redo());
        assert_eq!(e.text(), "");
    }

    // ---- IME composition ----

    #[test]
    fn composition_updates_replace_each_other_and_commit_as_one_undo_step() {
        let mut e = ed("ab", 1); // a|b
        e.compose(None, "n", Some(1..1));
        assert_eq!((e.text().as_str(), e.marked_range()), ("anb", Some(1..2)));
        e.compose(None, "ni", Some(2..2));
        assert_eq!(e.text(), "anib");
        assert_eq!(e.cursor(), 3);
        e.compose(None, "你", Some(1..1));
        assert_eq!(e.text(), "a你b");
        assert!(e.is_dirty(), "text differs from disk while composing");
        e.commit_text(None, "你");
        assert_eq!((e.text().as_str(), e.cursor()), ("a你b", 2));
        assert!(!e.is_composing());
        // one undo reverts the whole composition, no intermediate states
        assert!(e.undo());
        assert_eq!(e.text(), "ab");
        assert!(!e.undo());
        assert!(e.redo());
        assert_eq!(e.text(), "a你b");
    }

    #[test]
    fn composition_over_a_selection_replaces_it_and_commit_restores_on_undo() {
        let mut e = ed("hello world", 0);
        e.set_cursor(5, true); // select "hello"
        e.compose(None, "こ", Some(1..1));
        assert_eq!(e.text(), "こ world");
        e.compose(None, "こん", Some(2..2));
        e.commit_text(None, "今");
        assert_eq!(e.text(), "今 world");
        assert!(e.undo());
        assert_eq!(e.text(), "hello world");
    }

    #[test]
    fn cancelled_composition_leaves_no_trace_in_history() {
        let mut e = ed("ab", 1);
        e.compose(None, "n", None);
        e.compose(None, "", None); // IME cancels: empty pre-edit
        e.end_composition();
        assert_eq!(e.text(), "ab");
        assert!(!e.is_dirty());
        assert!(!e.undo());
    }

    #[test]
    fn ending_composition_keeps_the_text_and_is_one_step() {
        let mut e = ed("", 0);
        e.compose(None, "ka", Some(2..2));
        e.end_composition(); // unmark without replace: pre-edit becomes real text
        assert_eq!(e.text(), "ka");
        assert!(e.undo());
        assert_eq!(e.text(), "");
    }

    #[test]
    fn selection_inside_marked_text_is_reported() {
        let mut e = ed("", 0);
        e.compose(None, "abcd", Some(1..3));
        assert_eq!(e.selection(), 1..3);
        assert_eq!(e.marked_range(), Some(0..4));
    }

    #[test]
    fn ordinary_edit_during_composition_ends_it_cleanly() {
        let mut e = ed("x", 1);
        e.compose(None, "ab", None);
        e.insert("!"); // should not happen with a real IME, but must stay consistent
        assert!(!e.is_composing());
        assert_eq!(e.text(), "xab!");
    }

    #[test]
    fn utf16_offsets_count_surrogate_pairs() {
        let e = ed("a🌍b", 0); // 🌍 is 2 UTF-16 units
        assert_eq!(e.chars_range_to_utf16(0..3), 0..4);
        assert_eq!(e.utf16_range_to_chars(1..3), 1..2);
        let mut e = ed("a🌍b", 2);
        e.set_cursor(2, true);
        assert_eq!(e.selection_utf16().0, 3..3);
        e.set_cursor(1, false);
        e.set_cursor(2, true);
        assert_eq!(e.selection_utf16(), (1..3, false));
    }

    // ---- find / replace ----

    use crate::find::{FindOptions, Finder};

    fn finder(q: &str, regex: bool) -> Finder {
        Finder::new(
            q,
            FindOptions {
                case_sensitive: true,
                whole_word: false,
                regex,
            },
        )
        .unwrap()
        .unwrap()
    }

    #[test]
    fn replace_all_is_one_undo_step_and_places_the_cursor_after_the_last_replacement() {
        let mut e = ed("foo bar foo baz foo", 0);
        let v0 = e.version();
        assert_eq!(e.replace_matches(&finder("foo", false), "X", None), 3);
        assert_eq!(e.text(), "X bar X baz X");
        assert_eq!(e.cursor(), e.text().chars().count());
        assert!(e.version() > v0);
        assert!(e.undo());
        assert_eq!(
            e.text(),
            "foo bar foo baz foo",
            "a single undo restores everything"
        );
        assert!(!e.undo());
    }

    #[test]
    fn replace_only_the_selected_match() {
        let mut e = ed("a-a-a", 0);
        let f = finder("a", false);
        assert_eq!(e.replace_matches(&f, "bb", Some(2..3)), 1);
        assert_eq!(e.text(), "a-bb-a");
        assert_eq!(e.cursor(), 4);
        assert_eq!(
            e.replace_matches(&f, "x", Some(99..100)),
            0,
            "no such match"
        );
    }

    #[test]
    fn regex_replacement_uses_capture_groups() {
        let mut e = ed("k1=v1; k2=v2", 0);
        assert_eq!(
            e.replace_matches(&finder(r"(\w+)=(\w+)", true), "$2=$1", None),
            2
        );
        assert_eq!(e.text(), "v1=k1; v2=k2");
    }

    #[test]
    fn replacement_follows_the_files_line_endings_and_handles_unicode() {
        let mut e = ed("a\r\nb x", 0);
        assert_eq!(e.eol(), Eol::CrLf);
        assert_eq!(e.replace_matches(&finder("x", false), "1\n2", None), 1);
        assert_eq!(
            e.text(),
            "a\r\nb 1\r\n2",
            "pasted LF becomes the file's CRLF"
        );
        let mut u = ed("é🌍é", 0);
        assert_eq!(u.replace_matches(&finder("é", false), "日", None), 2);
        assert_eq!(u.text(), "日🌍日");
        assert_eq!(u.cursor(), 3);
        assert_eq!(
            u.replace_matches(&finder("🌍", false), "", None),
            1,
            "replace with nothing"
        );
        assert_eq!(u.text(), "日日");
    }

    #[test]
    fn select_range_selects_for_typing_over() {
        let mut e = ed("hello world", 0);
        e.select_range(6..11);
        assert_eq!(e.selected_text(), "world");
        e.insert("there");
        assert_eq!(e.text(), "hello there");
    }

    #[test]
    fn version_changes_on_every_content_change_only() {
        let mut e = ed("ab", 1);
        let v = e.version();
        e.move_right(false);
        assert_eq!(e.version(), v, "moving does not change content");
        e.insert("x");
        let v1 = e.version();
        assert!(v1 > v);
        e.undo();
        assert!(e.version() > v1);
        e.compose(None, "n", None);
        let v2 = e.version();
        e.compose(None, "ni", None);
        assert!(e.version() > v2);
    }

    #[test]
    fn composition_converts_line_breaks_to_the_files_eol() {
        let mut e = ed("a\r\nb", 4);
        e.commit_text(None, "x\ny");
        assert_eq!(e.text(), "a\r\nbx\r\ny");
    }
}
