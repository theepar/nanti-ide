//! In-file find (ED-07): literal / regex search with case and whole-word options. Matches are
//! reported as `char` ranges (the unit the rest of the editor uses), never bytes.

use std::fmt;
use std::ops::Range;

use regex::{Regex, RegexBuilder};

/// Search never returns more than this many matches (the UI says so when it is hit).
pub const MAX_MATCHES: usize = 10_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FindOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindError(pub String);

impl fmt::Display for FindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid pattern: {}", self.0)
    }
}

impl std::error::Error for FindError {}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// A compiled query. Build once, then `find_all` on any text.
#[derive(Debug, Clone)]
pub struct Finder {
    re: Regex,
    regex_mode: bool,
}

impl Finder {
    /// `Ok(None)` for an empty query (nothing to search for), `Err` for an invalid regex.
    pub fn new(query: &str, opts: FindOptions) -> Result<Option<Self>, FindError> {
        if query.is_empty() {
            return Ok(None);
        }
        let mut pattern = if opts.regex {
            query.to_owned()
        } else {
            regex::escape(query)
        };
        if opts.whole_word {
            if opts.regex {
                pattern = format!(r"\b(?:{pattern})\b");
            } else {
                // Only demand a boundary where the query itself starts/ends with a word character.
                if query.chars().next().is_some_and(is_word) {
                    pattern = format!(r"\b{pattern}");
                }
                if query.chars().next_back().is_some_and(is_word) {
                    pattern = format!(r"{pattern}\b");
                }
            }
        }
        let re = RegexBuilder::new(&pattern)
            .case_insensitive(!opts.case_sensitive)
            .multi_line(true)
            .size_limit(2 << 20)
            .build()
            .map_err(|e| FindError(first_line(&e.to_string())))?;
        Ok(Some(Self {
            re,
            regex_mode: opts.regex,
        }))
    }

    /// Non-overlapping, non-empty matches in order, as char ranges. Capped at `MAX_MATCHES`.
    pub fn find_all(&self, text: &str) -> Vec<Range<usize>> {
        let mut out = Vec::new();
        let mut chars_before = 0usize;
        let mut byte_cursor = 0usize;
        for m in self.re.find_iter(text) {
            if m.start() == m.end() {
                continue;
            }
            chars_before += text[byte_cursor..m.start()].chars().count();
            let len = text[m.start()..m.end()].chars().count();
            out.push(chars_before..chars_before + len);
            chars_before += len;
            byte_cursor = m.end();
            if out.len() >= MAX_MATCHES {
                break;
            }
        }
        out
    }

    /// The text that replaces the match starting at byte `byte_start` of `text`. In regex mode
    /// `$1`, `${name}` and `$0` expand to capture groups; otherwise `replacement` is literal.
    pub fn replacement(&self, text: &str, byte_start: usize, replacement: &str) -> String {
        if !self.regex_mode {
            return replacement.to_owned();
        }
        let mut out = String::new();
        if let Some(caps) = self.re.captures_at(text, byte_start) {
            caps.expand(replacement, &mut out);
        } else {
            out.push_str(replacement);
        }
        out
    }
}

fn first_line(s: &str) -> String {
    s.lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(s)
        .trim()
        .trim_start_matches("error: ")
        .to_owned()
}

/// Index of the first match starting at or after `pos`, wrapping to the first one.
pub fn next_index(matches: &[Range<usize>], pos: usize) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    Some(matches.iter().position(|m| m.start >= pos).unwrap_or(0))
}

/// Index of the last match starting before `pos`, wrapping to the last one.
pub fn prev_index(matches: &[Range<usize>], pos: usize) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    Some(
        matches
            .iter()
            .rposition(|m| m.start < pos)
            .unwrap_or(matches.len() - 1),
    )
}

/// Index of the match exactly equal to `selection`, if any.
pub fn index_of_selection(matches: &[Range<usize>], selection: &Range<usize>) -> Option<usize> {
    matches.iter().position(|m| m == selection)
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)] // expected match lists legitimately have one range
mod tests {
    use super::*;

    fn all(q: &str, opts: FindOptions, text: &str) -> Vec<Range<usize>> {
        Finder::new(q, opts).unwrap().unwrap().find_all(text)
    }

    const CI: FindOptions = FindOptions {
        case_sensitive: false,
        whole_word: false,
        regex: false,
    };

    #[test]
    fn literal_search_is_case_insensitive_by_default_and_escapes_metacharacters() {
        assert_eq!(all("foo", CI, "Foo foo FOO"), [0..3, 4..7, 8..11]);
        assert_eq!(
            all("a.b", CI, "a.b axb"),
            [0..3],
            "the dot is literal, not 'any char'"
        );
        let cs = FindOptions {
            case_sensitive: true,
            ..CI
        };
        assert_eq!(all("foo", cs, "Foo foo FOO"), [4..7]);
    }

    #[test]
    fn whole_word_respects_word_characters_only_at_query_edges() {
        let w = FindOptions {
            whole_word: true,
            ..CI
        };
        assert_eq!(all("cat", w, "cat concat cat_ cat."), [0..3, 16..19]);
        assert_eq!(
            all("@x", w, "a@x b@x"),
            [1..3, 5..7],
            "no boundary demanded before a non-word edge"
        );
    }

    #[test]
    fn regex_mode_with_anchors_per_line() {
        let r = FindOptions { regex: true, ..CI };
        assert_eq!(all(r"^\w+", r, "ab cd\nef gh"), [0..2, 6..8]);
        assert_eq!(all(r"\d+", r, "a1 b22 c333"), [1..2, 4..6, 8..11]);
        let wr = FindOptions {
            regex: true,
            whole_word: true,
            ..CI
        };
        assert_eq!(all("a|ab", wr, "ab a abc"), [0..2, 3..4]);
    }

    #[test]
    fn invalid_regex_reports_an_error_and_empty_query_is_none() {
        let r = FindOptions { regex: true, ..CI };
        let err = Finder::new("(unclosed", r).unwrap_err();
        assert!(!err.0.is_empty(), "{err}");
        assert!(Finder::new("", r).unwrap().is_none());
        assert!(
            Finder::new("(unclosed", CI).is_ok(),
            "fine as a literal query"
        );
    }

    #[test]
    fn offsets_are_chars_not_bytes() {
        // 'é' is 2 bytes, '🌍' is 4 bytes and 1 char
        assert_eq!(all("x", CI, "é🌍x日x"), [2..3, 4..5]);
        assert_eq!(all("🌍", CI, "a🌍b🌍"), [1..2, 3..4]);
    }

    #[test]
    fn matches_do_not_overlap_and_empty_matches_are_dropped() {
        assert_eq!(all("aa", CI, "aaaaa"), [0..2, 2..4]);
        let r = FindOptions { regex: true, ..CI };
        assert_eq!(all("x*", r, "abxxc"), [2..4], "zero-width matches skipped");
    }

    #[test]
    fn results_are_capped() {
        let text = "a".repeat(MAX_MATCHES + 50);
        assert_eq!(all("a", CI, &text).len(), MAX_MATCHES);
    }

    #[test]
    fn replacement_expands_captures_only_in_regex_mode() {
        let r = FindOptions { regex: true, ..CI };
        let f = Finder::new(r"(\w+)@(\w+)", r).unwrap().unwrap();
        let text = "me@host";
        assert_eq!(f.replacement(text, 0, "$2:$1"), "host:me");
        let lit = Finder::new("me", CI).unwrap().unwrap();
        assert_eq!(lit.replacement(text, 0, "$1 stays"), "$1 stays");
    }

    #[test]
    fn navigation_wraps_in_both_directions() {
        let m = [2..4, 10..12, 20..22];
        assert_eq!(next_index(&m, 0), Some(0));
        assert_eq!(next_index(&m, 5), Some(1));
        assert_eq!(next_index(&m, 21), Some(0), "wraps to first");
        assert_eq!(prev_index(&m, 11), Some(1));
        assert_eq!(prev_index(&m, 2), Some(2), "wraps to last");
        assert_eq!(next_index(&[], 0), None);
        assert_eq!(index_of_selection(&m, &(10..12)), Some(1));
        assert_eq!(index_of_selection(&m, &(10..11)), None);
    }
}
