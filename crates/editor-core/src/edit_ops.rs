use crate::Editor;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Word,
    Space,
    Punct,
    Newline,
}

fn class(c: char) -> Class {
    if c == '\n' || c == '\r' {
        Class::Newline
    } else if c.is_alphanumeric() || c == '_' {
        Class::Word
    } else if c.is_whitespace() {
        Class::Space
    } else {
        Class::Punct
    }
}

struct LineEdit {
    at: usize,
    remove: usize,
    insert: String,
}

fn map_through(edits: &[LineEdit], pos: usize, shift_at_equal: bool) -> usize {
    let mut shift: isize = 0;
    for e in edits {
        let ins = e.insert.chars().count() as isize;
        if pos > e.at + e.remove || (pos == e.at + e.remove && (shift_at_equal || e.remove > 0)) {
            shift += ins - e.remove as isize;
        } else if pos > e.at {
            return (e.at as isize + shift).max(0) as usize;
        } else {
            break;
        }
    }
    (pos as isize + shift).max(0) as usize
}

impl Editor {
    fn ch(&self, i: usize) -> char {
        self.buffer.slice(i..i + 1).chars().next().unwrap_or('\n')
    }

    fn eol_len(&self) -> usize {
        self.eol.as_str().chars().count()
    }

    fn word_left(&self, pos: usize) -> usize {
        let mut i = pos;
        if i > 0 && class(self.ch(i - 1)) == Class::Newline {
            i -= 1;
            if i > 0 && self.ch(i) == '\n' && self.ch(i - 1) == '\r' {
                i -= 1;
            }
            return i;
        }
        while i > 0 && class(self.ch(i - 1)) == Class::Space {
            i -= 1;
        }
        if i > 0 {
            let c = class(self.ch(i - 1));
            if c != Class::Newline {
                while i > 0 && class(self.ch(i - 1)) == c {
                    i -= 1;
                }
            }
        }
        i
    }

    fn word_right(&self, pos: usize) -> usize {
        let len = self.buffer.len_chars();
        let mut i = pos;
        if i >= len {
            return len;
        }
        if class(self.ch(i)) == Class::Newline {
            i += 1;
            if i < len && self.ch(i - 1) == '\r' && self.ch(i) == '\n' {
                i += 1;
            }
            return i;
        }
        while i < len && class(self.ch(i)) == Class::Space {
            i += 1;
        }
        if i < len {
            let c = class(self.ch(i));
            if c != Class::Newline {
                while i < len && class(self.ch(i)) == c {
                    i += 1;
                }
            }
        }
        i
    }

    pub fn move_word_left(&mut self, extend: bool) {
        self.goal_col = None;
        let to = if !extend && self.has_selection() {
            self.selection().start
        } else {
            self.word_left(self.cursor)
        };
        self.set_cursor(to, extend);
    }

    pub fn move_word_right(&mut self, extend: bool) {
        self.goal_col = None;
        let to = if !extend && self.has_selection() {
            self.selection().end
        } else {
            self.word_right(self.cursor)
        };
        self.set_cursor(to, extend);
    }

    pub fn delete_word_left(&mut self) {
        if self.has_selection() {
            return self.backspace();
        }
        let from = self.word_left(self.cursor);
        self.delete_span(from, self.cursor);
    }

    pub fn delete_word_right(&mut self) {
        if self.has_selection() {
            return self.delete_forward();
        }
        let to = self.word_right(self.cursor);
        self.delete_span(self.cursor, to);
    }

    fn delete_span(&mut self, from: usize, to: usize) {
        self.end_composition();
        self.typing_run = false;
        if from >= to {
            return;
        }
        self.buffer.delete(from..to);
        self.cursor = from;
        self.anchor = from;
        self.goal_col = None;
    }

    pub fn select_word_at(&mut self, pos: usize) {
        self.end_composition();
        self.typing_run = false;
        let len = self.buffer.len_chars();
        let pos = pos.min(len);
        let probe = if pos < len && class(self.ch(pos)) != Class::Newline {
            Some(pos)
        } else if pos > 0 && class(self.ch(pos - 1)) != Class::Newline {
            Some(pos - 1)
        } else {
            None
        };
        let Some(p) = probe else {
            return self.set_cursor(pos, false);
        };
        let c = class(self.ch(p));
        let (mut a, mut b) = (p, p + 1);
        while a > 0 && class(self.ch(a - 1)) == c {
            a -= 1;
        }
        while b < len && class(self.ch(b)) == c {
            b += 1;
        }
        self.anchor = a;
        self.cursor = b;
        self.goal_col = None;
    }

    pub fn select_line_at(&mut self, pos: usize) {
        self.end_composition();
        self.typing_run = false;
        let line = self.buffer.line_of(pos.min(self.buffer.len_chars()));
        let start = self.buffer.line_start(line);
        let end = if line + 1 < self.buffer.len_lines() {
            self.buffer.line_start(line + 1)
        } else {
            self.buffer.line_end(line)
        };
        self.anchor = start;
        self.cursor = end;
        self.goal_col = None;
    }

    fn line_span(&self) -> (usize, usize) {
        let sel = self.selection();
        let first = self.buffer.line_of(sel.start);
        let mut last = self.buffer.line_of(sel.end);
        if !sel.is_empty() && last > first && sel.end == self.buffer.line_start(last) {
            last -= 1;
        }
        (first, last)
    }

    pub fn has_multiline_selection(&self) -> bool {
        let sel = self.selection();
        !sel.is_empty() && self.buffer.line_of(sel.start) != self.buffer.line_of(sel.end)
    }

    fn apply_edits(&mut self, edits: &[LineEdit]) {
        self.end_composition();
        self.typing_run = false;
        if edits.is_empty() {
            return;
        }
        let (a, c) = (self.anchor, self.cursor);
        self.buffer.transact(|b| {
            for e in edits.iter().rev() {
                b.delete(e.at..e.at + e.remove);
                if !e.insert.is_empty() {
                    b.insert(e.at, &e.insert);
                }
            }
        });
        let empty = a == c;
        self.anchor = map_through(edits, a, empty || a > c).min(self.buffer.len_chars());
        self.cursor = map_through(edits, c, empty || c > a).min(self.buffer.len_chars());
        self.goal_col = None;
    }

    fn leading_ws(&self, line: usize) -> String {
        self.buffer
            .line_text(line)
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect()
    }

    pub fn indent_lines(&mut self, unit: &str) {
        let (first, last) = self.line_span();
        let edits: Vec<LineEdit> = (first..=last)
            .filter(|l| !self.buffer.line_text(*l).is_empty())
            .map(|l| LineEdit {
                at: self.buffer.line_start(l),
                remove: 0,
                insert: unit.to_owned(),
            })
            .collect();
        self.apply_edits(&edits);
    }

    pub fn outdent_lines(&mut self, unit: &str) {
        let width = unit.chars().count().max(1);
        let (first, last) = self.line_span();
        let mut edits = Vec::new();
        for l in first..=last {
            let text = self.buffer.line_text(l);
            let remove = if text.starts_with('\t') {
                1
            } else {
                text.chars().take(width).take_while(|c| *c == ' ').count()
            };
            if remove > 0 {
                edits.push(LineEdit {
                    at: self.buffer.line_start(l),
                    remove,
                    insert: String::new(),
                });
            }
        }
        self.apply_edits(&edits);
    }

    pub fn smart_newline(&mut self, unit: &str) {
        self.end_composition();
        self.typing_run = false;
        let sel = self.selection();
        let line = self.buffer.line_of(sel.start);
        let col = sel.start - self.buffer.line_start(line);
        let before: String = self.buffer.line_text(line).chars().take(col).collect();
        let indent: String = before
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let opener = before
            .trim_end()
            .chars()
            .last()
            .filter(|c| matches!(c, '{' | '(' | '[' | ':'));
        let next = (sel.end < self.buffer.len_chars()).then(|| self.ch(sel.end));
        let closes = matches!(
            (opener, next),
            (Some('{'), Some('}')) | (Some('('), Some(')')) | (Some('['), Some(']'))
        );
        let eol = self.eol.as_str().to_owned();
        let (text, cursor_back) = if closes {
            (
                format!("{eol}{indent}{unit}{eol}{indent}"),
                indent.chars().count() + self.eol_len(),
            )
        } else if matches!(opener, Some('{' | '(' | '[')) {
            (format!("{eol}{indent}{unit}"), 0)
        } else {
            (format!("{eol}{indent}"), 0)
        };
        let start = sel.start;
        let added = text.chars().count();
        self.buffer.transact(|b| {
            b.delete(sel);
            b.insert(start, &text);
        });
        self.cursor = start + added - cursor_back;
        self.anchor = self.cursor;
        self.goal_col = None;
    }

    pub fn insert_line(&mut self, below: bool, unit: &str) {
        self.end_composition();
        self.typing_run = false;
        let line = self.buffer.line_of(self.cursor);
        let indent = self.leading_ws(line);
        let eol = self.eol.as_str().to_owned();
        let opener = self
            .buffer
            .line_text(line)
            .trim_end()
            .ends_with(['{', '(', '[']);
        if below {
            let at = self.buffer.line_end(line);
            let extra = if opener { unit } else { "" };
            let text = format!("{eol}{indent}{extra}");
            let n = text.chars().count();
            self.buffer.insert(at, &text);
            self.cursor = at + n;
        } else {
            let at = self.buffer.line_start(line);
            let text = format!("{indent}{eol}");
            self.buffer.insert(at, &text);
            self.cursor = at + indent.chars().count();
        }
        self.anchor = self.cursor;
        self.goal_col = None;
    }

    pub fn toggle_line_comment(&mut self, prefix: &str) -> bool {
        let (first, last) = self.line_span();
        let lines: Vec<(usize, String)> = (first..=last)
            .map(|l| (l, self.buffer.line_text(l)))
            .filter(|(_, t)| !t.trim().is_empty())
            .collect();
        if lines.is_empty() {
            return false;
        }
        let all_commented = lines
            .iter()
            .all(|(_, t)| t.trim_start().starts_with(prefix));
        let before = self.buffer.version();
        let mut edits = Vec::new();
        if all_commented {
            for (l, t) in &lines {
                let indent = t.chars().take_while(|c| *c == ' ' || *c == '\t').count();
                let after = &t.trim_start()[prefix.len()..];
                let remove = prefix.chars().count() + usize::from(after.starts_with(' '));
                edits.push(LineEdit {
                    at: self.buffer.line_start(*l) + indent,
                    remove,
                    insert: String::new(),
                });
            }
        } else {
            let min_indent = lines
                .iter()
                .map(|(_, t)| t.chars().take_while(|c| *c == ' ' || *c == '\t').count())
                .min()
                .unwrap_or(0);
            for (l, _) in &lines {
                edits.push(LineEdit {
                    at: self.buffer.line_start(*l) + min_indent,
                    remove: 0,
                    insert: format!("{prefix} "),
                });
            }
        }
        self.apply_edits(&edits);
        self.buffer.version() != before
    }

    fn shift_selection(&mut self, delta: isize) {
        self.anchor = (self.anchor as isize + delta).max(0) as usize;
        self.cursor = (self.cursor as isize + delta).max(0) as usize;
    }

    pub fn move_lines(&mut self, down: bool) -> bool {
        self.end_composition();
        self.typing_run = false;
        let (a, b) = self.line_span();
        let eol = self.eol.as_str().to_owned();
        let block = self
            .buffer
            .slice(self.buffer.line_start(a)..self.buffer.line_end(b));
        let (range_start, range_end, replacement, delta) = if down {
            if b + 1 >= self.buffer.len_lines() {
                return false;
            }
            let next = self.buffer.line_text(b + 1);
            let delta = next.chars().count() + self.eol_len();
            (
                self.buffer.line_start(a),
                self.buffer.line_end(b + 1),
                format!("{next}{eol}{block}"),
                delta as isize,
            )
        } else {
            if a == 0 {
                return false;
            }
            let prev = self.buffer.line_text(a - 1);
            let delta = prev.chars().count() + self.eol_len();
            (
                self.buffer.line_start(a - 1),
                self.buffer.line_end(b),
                format!("{block}{eol}{prev}"),
                -(delta as isize),
            )
        };
        self.buffer.transact(|buf| {
            buf.delete(range_start..range_end);
            buf.insert(range_start, &replacement);
        });
        self.shift_selection(delta);
        self.goal_col = None;
        true
    }

    pub fn duplicate_lines(&mut self, down: bool) {
        self.end_composition();
        self.typing_run = false;
        let (a, b) = self.line_span();
        let eol = self.eol.as_str().to_owned();
        let block = self
            .buffer
            .slice(self.buffer.line_start(a)..self.buffer.line_end(b));
        let n = block.chars().count() + self.eol_len();
        if down {
            let at = self.buffer.line_end(b);
            self.buffer.insert(at, &format!("{eol}{block}"));
            self.shift_selection(n as isize);
        } else {
            let at = self.buffer.line_start(a);
            self.buffer.insert(at, &format!("{block}{eol}"));
        }
        self.goal_col = None;
    }

    pub fn delete_lines(&mut self) {
        self.end_composition();
        self.typing_run = false;
        let (a, b) = self.line_span();
        let range = if b + 1 < self.buffer.len_lines() {
            self.buffer.line_start(a)..self.buffer.line_start(b + 1)
        } else if a > 0 {
            self.buffer.line_end(a - 1)..self.buffer.line_end(b)
        } else {
            0..self.buffer.line_end(b)
        };
        let start = range.start;
        self.buffer.delete(range);
        self.cursor = start;
        self.anchor = start;
        self.goal_col = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ed(text: &str, cursor: usize) -> Editor {
        let mut e = Editor::new(text);
        e.set_cursor(cursor, false);
        e
    }

    fn at_end(text: &str) -> Editor {
        let mut e = Editor::new(text);
        e.move_doc_end(false);
        e
    }

    #[test]
    fn word_movement_skips_runs_of_the_same_kind() {
        let mut e = ed("foo bar_baz  +=  qux", 0);
        let mut stops = vec![];
        for _ in 0..5 {
            e.move_word_right(false);
            stops.push(e.cursor());
        }
        assert_eq!(stops, vec![3, 11, 15, 20, 20]);
    }

    #[test]
    fn word_left_mirrors_word_right() {
        let mut e = at_end("foo bar_baz  +=  qux");
        let mut stops = vec![];
        for _ in 0..5 {
            e.move_word_left(false);
            stops.push(e.cursor());
        }
        assert_eq!(stops, vec![17, 13, 4, 0, 0]);
    }

    #[test]
    fn word_movement_steps_over_one_line_break_and_handles_crlf_and_unicode() {
        let mut e = ed("ab\r\ncd", 2);
        e.move_word_right(false);
        assert_eq!(e.cursor(), 4, "CRLF is one stop");
        e.move_word_left(false);
        assert_eq!(e.cursor(), 2);
        let mut u = ed("héllo wörld 日本語", 0);
        u.move_word_right(false);
        assert_eq!(u.cursor(), 5);
        u.move_word_right(false);
        assert_eq!(u.cursor(), 11);
        u.move_word_right(false);
        assert_eq!(u.cursor(), 15, "CJK letters are word characters");
    }

    #[test]
    fn shift_word_movement_extends_and_plain_movement_collapses() {
        let mut e = ed("one two three", 0);
        e.move_word_right(true);
        e.move_word_right(true);
        assert_eq!(e.selected_text(), "one two");
        e.move_word_left(false);
        assert_eq!(
            e.selection(),
            0..0,
            "collapses to the start of the selection"
        );
    }

    #[test]
    fn delete_word_left_and_right_are_single_undo_steps() {
        let mut e = at_end("let value = compute(x)");
        e.delete_word_left();
        assert_eq!(e.text(), "let value = compute(x");
        e.delete_word_left();
        assert_eq!(e.text(), "let value = compute(");
        e.delete_word_left();
        assert_eq!(
            e.text(),
            "let value = compute",
            "punctuation is its own run"
        );
        e.delete_word_left();
        assert_eq!(e.text(), "let value = ");
        assert!(e.undo());
        assert_eq!(e.text(), "let value = compute");
        let mut r = ed("alpha beta gamma", 5);
        r.delete_word_right();
        assert_eq!(r.text(), "alpha gamma");
        let mut s = ed("keep selected", 0);
        s.select_range(0..4);
        s.delete_word_left();
        assert_eq!(s.text(), " selected", "a selection is deleted as is");
    }

    #[test]
    fn double_click_selects_a_word_and_triple_click_a_line() {
        let mut e = ed("let foo_bar = 1;\nnext", 0);
        e.select_word_at(6);
        assert_eq!(e.selected_text(), "foo_bar");
        e.select_word_at(11);
        assert_eq!(e.selected_text(), " ", "a run of spaces");
        e.select_word_at(12);
        assert_eq!(e.selected_text(), "=");
        e.select_word_at(16);
        assert_eq!(
            e.selected_text(),
            ";",
            "at the end of a line it picks the word before the break"
        );
        e.select_line_at(4);
        assert_eq!(e.selected_text(), "let foo_bar = 1;\n");
        e.select_line_at(19);
        assert_eq!(
            e.selected_text(),
            "next",
            "the last line has no break to include"
        );
        let mut empty = ed("a\n\nb", 2);
        empty.select_word_at(2);
        assert!(empty.selection().is_empty());
    }

    #[test]
    fn enter_keeps_indentation_and_opens_blocks() {
        let mut e = at_end("    let x = 1;");
        e.smart_newline("    ");
        assert_eq!(e.text(), "    let x = 1;\n    ");
        let mut b = at_end("fn f() {");
        b.smart_newline("    ");
        assert_eq!(b.text(), "fn f() {\n    ");
        let mut c = ed("fn f() {}", 8);
        c.smart_newline("    ");
        assert_eq!(c.text(), "fn f() {\n    \n}");
        assert_eq!(c.cursor(), 13, "the caret is on the indented middle line");
        assert!(c.undo());
        assert_eq!(c.text(), "fn f() {}", "one undo step");
        let mut p = ed("\tif x []", 7);
        p.smart_newline("  ");
        assert!(
            p.text().starts_with("\tif x [\n\t  \n\t]"),
            "{:?}",
            p.text()
        );
        let mut sel = ed("  keep drop tail", 0);
        sel.select_range(6..11);
        sel.smart_newline("  ");
        assert_eq!(
            sel.text(),
            "  keep\n   tail",
            "a selection is replaced by the line break"
        );
    }

    #[test]
    fn enter_in_a_crlf_file_inserts_crlf_with_indent() {
        let mut e = at_end("  a\r\n  b");
        e.smart_newline("  ");
        assert_eq!(e.text(), "  a\r\n  b\r\n  ");
    }

    #[test]
    fn ctrl_enter_opens_lines_with_indentation() {
        let mut e = ed("    one\n    two", 5);
        e.insert_line(true, "    ");
        assert_eq!(e.text(), "    one\n    \n    two");
        assert_eq!(e.line_col(), (1, 4));
        let mut u = ed("  one", 3);
        u.insert_line(false, "  ");
        assert_eq!(u.text(), "  \n  one");
        assert_eq!(u.line_col(), (0, 2));
        let mut o = at_end("if (x) {");
        o.insert_line(true, "  ");
        assert_eq!(o.text(), "if (x) {\n  ");
    }

    #[test]
    fn indent_and_outdent_apply_to_all_selected_non_empty_lines() {
        let mut e = ed("a\n\nb\nc", 0);
        e.select_range(0..5);
        e.indent_lines("  ");
        assert_eq!(e.text(), "  a\n\n  b\nc");
        assert_eq!(
            e.selected_text(),
            "  a\n\n  b\n",
            "the selection follows the text and keeps its start"
        );
        e.outdent_lines("  ");
        assert_eq!(e.text(), "a\n\nb\nc");
        let mut t = ed("\ta\n b\nc", 0);
        t.select_all();
        t.outdent_lines("    ");
        assert_eq!(t.text(), "a\nb\nc");
        assert!(t.undo());
        assert_eq!(t.text(), "\ta\n b\nc", "outdent is one undo step");
        let mut s = ed("x\ny\nz", 0);
        s.select_range(0..4);
        s.indent_lines("--");
        assert_eq!(s.text(), "--x\n--y\nz");
        let mut c = ed("one\ntwo", 5);
        c.outdent_lines("  ");
        assert_eq!(c.text(), "one\ntwo");
        c.indent_lines("  ");
        assert_eq!(c.text(), "one\n  two");
    }

    #[test]
    fn toggle_comment_adds_at_the_minimum_indent_and_removes_when_all_are_commented() {
        let mut e = ed("fn f() {\n    let x = 1;\n\n    x\n}", 0);
        e.select_all();
        assert!(e.toggle_line_comment("//"));
        assert_eq!(
            e.text(),
            "// fn f() {\n    // let x = 1;\n\n    // x\n// }"
                .replace("    // let", "//     let")
                .replace("    // x", "//     x")
        );
    }

    #[test]
    fn toggle_comment_round_trips_and_keeps_blank_lines() {
        let src = "    a\n\n      b\n    c";
        let mut e = ed(src, 0);
        e.select_all();
        e.toggle_line_comment("#");
        assert_eq!(e.text(), "    # a\n\n    #   b\n    # c");
        e.toggle_line_comment("#");
        assert_eq!(e.text(), src, "second toggle restores the text exactly");
        assert!(e.undo());
        assert_eq!(
            e.text(),
            "    # a\n\n    #   b\n    # c",
            "one undo step each"
        );
    }

    #[test]
    fn toggle_comment_uncomments_only_when_every_line_is_commented() {
        let mut e = ed("// a\nb", 0);
        e.select_all();
        e.toggle_line_comment("//");
        assert_eq!(e.text(), "// // a\n// b", "mixed selection: comment all");
        let mut c = ed("//a\n//  b", 0);
        c.select_all();
        c.toggle_line_comment("//");
        assert_eq!(
            c.text(),
            "a\n b",
            "only one space after the marker is removed"
        );
        let mut blank = ed("\n\n", 0);
        blank.select_all();
        assert!(!blank.toggle_line_comment("//"), "nothing to comment");
    }

    #[test]
    fn toggle_comment_on_the_caret_line_keeps_the_caret_in_place() {
        let mut e = ed("abc\ndef", 5);
        e.toggle_line_comment("//");
        assert_eq!(e.text(), "abc\n// def");
        assert_eq!(e.cursor(), 8, "the caret stays on the same character");
        e.toggle_line_comment("//");
        assert_eq!(e.text(), "abc\ndef");
        assert_eq!(e.cursor(), 5);
    }

    #[test]
    fn move_lines_swaps_with_neighbours_and_carries_the_selection() {
        let mut e = ed("one\ntwo\nthree", 5);
        assert!(e.move_lines(false));
        assert_eq!(e.text(), "two\none\nthree");
        assert_eq!(e.line_col(), (0, 1), "caret stays on 'two'");
        assert!(!e.move_lines(false), "already first");
        e.move_lines(true);
        e.move_lines(true);
        assert_eq!(
            e.text(),
            "one\nthree\ntwo",
            "the last line has no terminator and none is invented"
        );
        assert!(!e.move_lines(true));
        let mut m = ed("a\nb\nc\nd", 0);
        m.select_range(2..5);
        m.move_lines(true);
        assert_eq!(m.text(), "a\nd\nb\nc");
        assert_eq!(m.selected_text(), "b\nc");
        assert!(m.undo());
        assert_eq!(m.text(), "a\nb\nc\nd", "one undo step");
    }

    #[test]
    fn move_lines_keeps_crlf() {
        let mut e = ed("a\r\nb\r\nc", 0);
        e.move_lines(true);
        assert_eq!(e.text(), "b\r\na\r\nc");
    }

    #[test]
    fn duplicate_lines_down_and_up() {
        let mut e = ed("one\ntwo", 1);
        e.duplicate_lines(true);
        assert_eq!(e.text(), "one\none\ntwo");
        assert_eq!(e.line_col(), (1, 1), "the caret moves to the copy");
        e.duplicate_lines(false);
        assert_eq!(e.text(), "one\none\none\ntwo");
        assert_eq!(e.line_col(), (1, 1));
        let mut last = at_end("a\nb");
        last.duplicate_lines(true);
        assert_eq!(last.text(), "a\nb\nb");
        let mut multi = ed("a\nb\nc", 0);
        multi.select_range(0..3);
        multi.duplicate_lines(true);
        assert_eq!(multi.text(), "a\nb\na\nb\nc");
    }

    #[test]
    fn delete_lines_handles_middle_last_and_only_lines() {
        let mut e = ed("one\ntwo\nthree", 5);
        e.delete_lines();
        assert_eq!(e.text(), "one\nthree");
        assert_eq!(e.line_col(), (1, 0));
        e.delete_lines();
        assert_eq!(
            e.text(),
            "one",
            "deleting the last line removes the break before it"
        );
        e.delete_lines();
        assert_eq!(e.text(), "");
        e.delete_lines();
        assert_eq!(e.text(), "");
        let mut m = ed("a\nb\nc\nd", 2);
        m.select_range(2..5);
        m.delete_lines();
        assert_eq!(m.text(), "a\nd");
        assert!(m.undo());
        assert_eq!(m.text(), "a\nb\nc\nd");
    }

    #[test]
    fn commands_never_panic_on_empty_and_tiny_documents() {
        for text in ["", "x", "\n", "\r\n", "  "] {
            let mut e = Editor::new(text);
            e.move_word_left(false);
            e.move_word_right(true);
            e.delete_word_left();
            e.delete_word_right();
            e.select_word_at(0);
            e.select_line_at(5);
            e.indent_lines("  ");
            e.outdent_lines("  ");
            e.smart_newline("  ");
            e.insert_line(true, "  ");
            e.insert_line(false, "  ");
            e.toggle_line_comment("//");
            e.move_lines(true);
            e.move_lines(false);
            e.duplicate_lines(true);
            e.duplicate_lines(false);
            e.delete_lines();
            assert!(e.cursor() <= e.buffer().len_chars());
        }
    }
}
