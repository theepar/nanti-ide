use std::time::Instant;

use syntax::{Language, SyntaxHighlighter};

fn rust_source(lines: usize) -> String {
    let unit = "pub fn compute(values: &[u32], factor: u32) -> Vec<u32> { values.iter().map(|v| v * factor).collect() } // note\n";
    unit.repeat(lines)
}

#[test]
#[ignore = "prints timings; run with --ignored --nocapture"]
fn highlight_cost_by_file_size() {
    let mut h = SyntaxHighlighter::new();
    let _ = h.highlight(Language::Rust, "fn warm() {}");
    for lines in [500, 2_000, 10_000, 40_000] {
        let src = rust_source(lines);
        let start = Instant::now();
        let spans = h.highlight(Language::Rust, &src).map(|l| l.len());
        println!(
            "{lines:>6} lines ({:>5} KiB): {:>8.1?} -> {spans:?} lines",
            src.len() / 1024,
            start.elapsed()
        );
    }
}
