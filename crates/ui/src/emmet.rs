use std::path::Path;

use editor_core::Editor;

const CURSOR: char = '\u{1}';

const TAGS: &[&str] = &[
    "a",
    "abbr",
    "article",
    "aside",
    "audio",
    "b",
    "blockquote",
    "body",
    "br",
    "button",
    "canvas",
    "code",
    "dd",
    "details",
    "div",
    "dl",
    "dt",
    "em",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "i",
    "iframe",
    "img",
    "input",
    "label",
    "legend",
    "li",
    "link",
    "main",
    "meta",
    "nav",
    "ol",
    "option",
    "p",
    "pre",
    "script",
    "section",
    "select",
    "small",
    "span",
    "strong",
    "style",
    "table",
    "tbody",
    "td",
    "textarea",
    "th",
    "thead",
    "tr",
    "ul",
    "title",
    "video",
    "summary",
    "dialog",
    "template",
    "picture",
    "source",
    "svg",
];

const VOID: &[&str] = &["br", "hr", "img", "input", "link", "meta", "area"];

pub fn applies_to(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_ascii_lowercase().as_str(),
            "html" | "htm" | "xhtml" | "vue" | "svelte"
        )
    })
}

#[derive(Debug, PartialEq, Eq)]
pub struct Expansion {
    pub text: String,
    // Where the cursor goes, counted in chars from the start of `text`.
    pub cursor: usize,
}

#[derive(Debug, Default)]
struct Node {
    tag: String,
    id: Option<String>,
    classes: Vec<String>,
    repeat: usize,
    children: Vec<Node>,
}

/// The abbreviation that ends `prefix`: the run of Emmet characters right before the cursor.
pub fn abbreviation_before(prefix: &str) -> Option<&str> {
    let start = prefix
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_ascii_alphanumeric() || "!.#>*+:_-".contains(*c))
        .last()
        .map(|(i, _)| i)?;
    let abbr = &prefix[start..];
    let before = prefix[..start].chars().next_back();
    // Inside a tag or an attribute value Tab must stay a Tab.
    (!matches!(before, Some('<' | '/' | '"' | '\'' | '=' | '&')) && !abbr.is_empty())
        .then_some(abbr)
}

pub fn expand(abbr: &str, indent: &str, unit: &str) -> Option<Expansion> {
    let body = if abbr == "!" || abbr == "html:5" {
        boilerplate(unit)
    } else {
        let nodes = parse(abbr)?;
        let known = nodes
            .first()
            .is_some_and(|n| n.tag.is_empty() || TAGS.contains(&n.tag.as_str()));
        if !known {
            return None;
        }
        let mut out = String::new();
        let mut first_empty = true;
        render(&nodes, "", "", unit, &mut out, &mut first_empty);
        out
    };
    let body = body.replace('\n', &format!("\n{indent}"));
    let cursor = body
        .chars()
        .position(|c| c == CURSOR)
        .unwrap_or_else(|| body.chars().count());
    Some(Expansion {
        text: body.replace(CURSOR, ""),
        cursor,
    })
}

fn boilerplate(unit: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n{unit}<meta charset=\"UTF-8\">\n{unit}<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n{unit}<title>Document</title>\n</head>\n<body>\n{unit}{CURSOR}\n</body>\n</html>"
    )
}

fn parse(s: &str) -> Option<Vec<Node>> {
    let (head, rest) = match s.split_once('>') {
        Some((h, r)) => (h, Some(r)),
        None => (s, None),
    };
    let mut siblings = head
        .split('+')
        .map(parse_node)
        .collect::<Option<Vec<_>>>()?;
    if let Some(rest) = rest {
        siblings.last_mut()?.children = parse(rest)?;
    }
    Some(siblings)
}

fn parse_node(s: &str) -> Option<Node> {
    let mut node = Node {
        repeat: 1,
        ..Node::default()
    };
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len()
        && (b[i].is_ascii_alphanumeric() || b[i] == b':' || b[i] == b'-')
        && (i > 0 || b[i].is_ascii_alphabetic())
    {
        i += 1;
    }
    node.tag = s[..i].to_ascii_lowercase();
    while i < b.len() {
        let sigil = b[i];
        let from = i + 1;
        i = from;
        while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'-' || b[i] == b'_') {
            i += 1;
        }
        let word = &s[from..i];
        if word.is_empty() {
            return None;
        }
        match sigil {
            b'#' => node.id = Some(word.to_owned()),
            b'.' => node.classes.push(word.to_owned()),
            b'*' => {
                node.repeat = word.parse().ok().filter(|n| (1..=100).contains(n))?;
                if i != b.len() {
                    return None;
                }
            }
            _ => return None,
        }
    }
    (!node.tag.is_empty() || node.id.is_some() || !node.classes.is_empty()).then_some(node)
}

fn render(
    nodes: &[Node],
    parent: &str,
    pad: &str,
    unit: &str,
    out: &mut String,
    first_empty: &mut bool,
) {
    let mut first_line = true;
    for node in nodes {
        for _ in 0..node.repeat {
            if !first_line || !out.is_empty() {
                out.push('\n');
                out.push_str(pad);
            }
            first_line = false;
            render_one(node, parent, pad, unit, out, first_empty);
        }
    }
}

fn render_one(
    node: &Node,
    parent: &str,
    pad: &str,
    unit: &str,
    out: &mut String,
    first_empty: &mut bool,
) {
    let tag = if node.tag.is_empty() {
        if matches!(parent, "ul" | "ol") {
            "li"
        } else {
            "div"
        }
    } else {
        node.tag.as_str()
    };
    let mut open = format!("<{tag}");
    if let Some(id) = &node.id {
        open.push_str(&format!(" id=\"{id}\""));
    }
    if !node.classes.is_empty() {
        open.push_str(&format!(" class=\"{}\"", node.classes.join(" ")));
    }
    match tag {
        "a" => open.push_str(" href=\"\""),
        "img" => open.push_str(" src=\"\" alt=\"\""),
        "input" => open.push_str(" type=\"text\""),
        "link" => open.push_str(" rel=\"stylesheet\" href=\"\""),
        _ => {}
    }
    open.push('>');
    if VOID.contains(&tag) {
        out.push_str(&open);
        return;
    }
    if node.children.is_empty() {
        out.push_str(&open);
        if std::mem::take(first_empty) {
            out.push(CURSOR);
        }
        out.push_str(&format!("</{tag}>"));
        return;
    }
    out.push_str(&open);
    let inner = format!("{pad}{unit}");
    out.push('\n');
    out.push_str(&inner);
    let mut body = String::new();
    render(&node.children, tag, &inner, unit, &mut body, first_empty);
    out.push_str(&body);
    out.push('\n');
    out.push_str(pad);
    out.push_str(&format!("</{tag}>"));
}

/// Expands the abbreviation before the cursor in place; false when there is nothing to expand.
pub fn expand_at_cursor(e: &mut Editor, unit: &str) -> bool {
    if e.has_selection() {
        return false;
    }
    let cur = e.cursor();
    let line_start = e.buffer().line_start(e.buffer().line_of(cur));
    let prefix = e.buffer().slice(line_start..cur);
    let Some(abbr) = abbreviation_before(&prefix) else {
        return false;
    };
    let indent: String = prefix
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let Some(exp) = expand(abbr, &indent, unit) else {
        return false;
    };
    let start = cur - abbr.chars().count();
    e.select_range(start..cur);
    e.insert(&exp.text);
    e.set_cursor(start + exp.cursor, false);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ex(abbr: &str) -> String {
        let e = expand(abbr, "", "  ").unwrap();
        let mut t = e.text.chars().collect::<Vec<_>>();
        t.insert(e.cursor, '|');
        t.into_iter().collect()
    }

    #[test]
    fn bang_makes_the_html5_boilerplate_with_the_cursor_in_the_body() {
        assert_eq!(
            ex("!"),
            "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"UTF-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n  <title>Document</title>\n</head>\n<body>\n  |\n</body>\n</html>"
        );
        assert_eq!(expand("html:5", "", "  "), expand("!", "", "  "));
    }

    #[test]
    fn single_elements_ids_and_classes() {
        assert_eq!(ex("p"), "<p>|</p>");
        assert_eq!(ex("div.box"), "<div class=\"box\">|</div>");
        assert_eq!(ex(".a.b"), "<div class=\"a b\">|</div>");
        assert_eq!(
            ex("section#main.wide"),
            "<section id=\"main\" class=\"wide\">|</section>"
        );
        assert_eq!(ex("a"), "<a href=\"\">|</a>");
        assert_eq!(ex("img"), "<img src=\"\" alt=\"\">|");
    }

    #[test]
    fn nesting_siblings_and_repeats() {
        assert_eq!(
            ex("ul>li*3"),
            "<ul>\n  <li>|</li>\n  <li></li>\n  <li></li>\n</ul>"
        );
        assert_eq!(
            ex("nav>ul>.item"),
            "<nav>\n  <ul>\n    <li class=\"item\">|</li>\n  </ul>\n</nav>"
        );
        assert_eq!(ex("h1+p"), "<h1>|</h1>\n<p></p>");
    }

    #[test]
    fn the_current_indent_carries_over_to_every_later_line() {
        let e = expand("ul>li", "    ", "  ").unwrap();
        assert_eq!(e.text, "<ul>\n      <li></li>\n    </ul>");
    }

    #[test]
    fn plain_words_and_broken_abbreviations_are_left_alone() {
        assert!(expand("hello", "", "  ").is_none());
        assert!(expand("div>", "", "  ").is_none());
        assert!(expand("li*0", "", "  ").is_none());
        assert!(expand("div.", "", "  ").is_none());
    }

    #[test]
    fn the_abbreviation_is_the_run_before_the_cursor_and_never_inside_a_tag() {
        assert_eq!(abbreviation_before("    ul>li*3"), Some("ul>li*3"));
        assert_eq!(abbreviation_before("<p>text div.a"), Some("div.a"));
        assert_eq!(abbreviation_before("<div"), None);
        assert_eq!(abbreviation_before("a=\"b"), None);
        assert_eq!(abbreviation_before("  "), None);
    }

    #[test]
    fn expanding_in_an_editor_replaces_the_token_and_places_the_cursor() {
        let mut e = Editor::new("<body>\n  ul>li*2");
        e.move_doc_end(false);
        assert!(expand_at_cursor(&mut e, "  "));
        assert_eq!(
            e.text(),
            "<body>\n  <ul>\n    <li></li>\n    <li></li>\n  </ul>"
        );
        assert_eq!(e.line_col(), (2, 8));
        assert!(e.undo());
        assert_eq!(e.text(), "<body>\n  ul>li*2");

        let mut e = Editor::new("!");
        e.move_doc_end(false);
        assert!(expand_at_cursor(&mut e, "    "));
        assert!(e.text().starts_with("<!DOCTYPE html>\n<html lang=\"en\">"));
        assert_eq!(e.line_col(), (8, 4));

        let mut e = Editor::new("hello");
        e.move_doc_end(false);
        assert!(!expand_at_cursor(&mut e, "  "));
    }

    #[test]
    fn only_html_like_files_expand() {
        assert!(applies_to(Path::new("a/index.html")));
        assert!(applies_to(Path::new("A.HTM")));
        assert!(applies_to(Path::new("App.vue")));
        assert!(!applies_to(Path::new("main.rs")));
        assert!(!applies_to(Path::new("README")));
    }
}
