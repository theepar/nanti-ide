#![cfg(windows)]

use std::path::Path;

use syntax::{Language, SyntaxHighlighter};

#[repr(C)]
struct Counters {
    cb: u32,
    page_faults: u32,
    peak_working_set: usize,
    working_set: usize,
    quota_peak_paged: usize,
    quota_paged: usize,
    quota_peak_nonpaged: usize,
    quota_nonpaged: usize,
    private_bytes: usize,
    peak_private_bytes: usize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> isize;
    fn K32GetProcessMemoryInfo(process: isize, counters: *mut Counters, cb: u32) -> i32;
}

fn private_kib() -> usize {
    let mut c = Counters {
        cb: std::mem::size_of::<Counters>() as u32,
        page_faults: 0,
        peak_working_set: 0,
        working_set: 0,
        quota_peak_paged: 0,
        quota_paged: 0,
        quota_peak_nonpaged: 0,
        quota_nonpaged: 0,
        private_bytes: 0,
        peak_private_bytes: 0,
    };
    // SAFETY: the struct matches PROCESS_MEMORY_COUNTERS_EX and the pseudo handle is always valid.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) };
    assert_ne!(ok, 0);
    c.private_bytes / 1024
}

fn sample(lang: Language) -> String {
    let unit = match lang {
        Language::Rust => {
            "fn main() { let x: Vec<u32> = (0..10).collect(); println!(\"{x:?}\"); }\n"
        }
        Language::JavaScript | Language::TypeScript | Language::Tsx => {
            "export function f(a, b) { return a + b; }\nconst x = [1, 2, 3].map((n) => n * 2);\n"
        }
        Language::Go => "package main\nfunc main() { println(\"hi\") }\n",
        Language::Json => "{\"a\": [1, 2, {\"b\": null}], \"c\": \"text\"}\n",
        Language::Dart => "void main() { print('hi'); }\nclass A { final int x; A(this.x); }\n",
        Language::Markdown => "# Title\n\nSome *text* and `code`.\n\n- item\n",
        _ => "x = \"hi\" # note\nfoo(bar, 1)\n",
    };
    unit.repeat(2000)
}

#[test]
#[ignore = "prints memory numbers; run with --ignored --nocapture"]
fn memory_cost_of_each_language() {
    let all = Language::ALL;
    let mut h = SyntaxHighlighter::new();
    let base = private_kib();
    println!("baseline private memory: {base} KiB (all grammars are linked in, none loaded)");
    let mut prev = base;
    for lang in all {
        let text = sample(lang);
        let _ = h.highlight(lang, &text);
        let now = private_kib();
        println!(
            "{lang:?}: +{} KiB (first use builds the grammar and highlights {} KiB of text)",
            now.saturating_sub(prev),
            text.len() / 1024
        );
        prev = now;
    }
    println!(
        "total after using every language: +{} KiB",
        prev.saturating_sub(base)
    );
    let _ = Path::new(".");
}
