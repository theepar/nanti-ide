
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub const MAX_LINES: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputLine {
    pub at: Duration,
    pub channel: String,
    pub text: String,
}

pub struct OutputLog {
    start: Instant,
    lines: VecDeque<OutputLine>,
}

impl Default for OutputLog {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputLog {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            lines: VecDeque::new(),
        }
    }

    pub fn push(&mut self, channel: &str, text: &str) {
        let at = self.start.elapsed();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            if self.lines.len() >= MAX_LINES {
                self.lines.pop_front();
            }
            self.lines.push_back(OutputLine {
                at,
                channel: channel.to_owned(),
                text: line.to_owned(),
            });
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn lines(&self) -> impl Iterator<Item = &OutputLine> {
        self.lines.iter()
    }

    pub fn stamp(line: &OutputLine) -> String {
        format!("{:>7.1}s", line.at.as_secs_f32())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_line_text_becomes_lines_and_blank_lines_are_skipped() {
        let mut l = OutputLog::new();
        l.push("Git", "first\n\nsecond\n   \nthird");
        assert_eq!(
            l.lines().map(|x| x.text.as_str()).collect::<Vec<_>>(),
            ["first", "second", "third"]
        );
        assert!(l.lines().all(|x| x.channel == "Git"));
    }

    #[test]
    fn the_log_is_bounded_and_keeps_the_newest_lines() {
        let mut l = OutputLog::new();
        for i in 0..MAX_LINES + 50 {
            l.push("c", &format!("line {i}"));
        }
        assert_eq!(l.len(), MAX_LINES);
        assert_eq!(l.lines().next().unwrap().text, "line 50");
        assert_eq!(
            l.lines().last().unwrap().text,
            format!("line {}", MAX_LINES + 49)
        );
        l.clear();
        assert!(l.is_empty());
    }
}
