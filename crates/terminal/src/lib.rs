use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Osc52, Term, TermMode};
use alacritty_terminal::tty;
use alacritty_terminal::vte::ansi::{Color, CursorShape as AlacrittyCursorShape, NamedColor, Rgb};

pub const DEFAULT_SCROLLBACK: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb8(pub u8, pub u8, pub u8);

impl Rgb8 {
    pub fn to_u32(self) -> u32 {
        (u32::from(self.0) << 16) | (u32::from(self.1) << 8) | u32::from(self.2)
    }
}

impl From<Rgb> for Rgb8 {
    fn from(c: Rgb) -> Self {
        Rgb8(c.r, c.g, c.b)
    }
}

pub const DEFAULT_FG: Rgb8 = Rgb8(0xcc, 0xcc, 0xcc);
pub const DEFAULT_BG: Rgb8 = Rgb8(0x18, 0x18, 0x18);

const ANSI: [Rgb8; 16] = [
    Rgb8(0x00, 0x00, 0x00),
    Rgb8(0xcd, 0x31, 0x31),
    Rgb8(0x0d, 0xbc, 0x79),
    Rgb8(0xe5, 0xe5, 0x10),
    Rgb8(0x24, 0x72, 0xc8),
    Rgb8(0xbc, 0x3f, 0xbc),
    Rgb8(0x11, 0xa8, 0xcd),
    Rgb8(0xe5, 0xe5, 0xe5),
    Rgb8(0x66, 0x66, 0x66),
    Rgb8(0xf1, 0x4c, 0x4c),
    Rgb8(0x23, 0xd1, 0x8b),
    Rgb8(0xf5, 0xf5, 0x43),
    Rgb8(0x3b, 0x8e, 0xea),
    Rgb8(0xd6, 0x70, 0xd6),
    Rgb8(0x29, 0xb8, 0xdb),
    Rgb8(0xe5, 0xe5, 0xe5),
];

pub fn indexed_color(i: u8) -> Rgb8 {
    match i {
        0..=15 => ANSI[i as usize],
        16..=231 => {
            let n = i - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            Rgb8(level(n / 36), level((n / 6) % 6), level(n % 6))
        }
        232..=255 => {
            let g = 8 + 10 * (i - 232);
            Rgb8(g, g, g)
        }
    }
}

fn resolve(color: Color, is_fg: bool) -> Rgb8 {
    match color {
        Color::Spec(c) => c.into(),
        Color::Indexed(i) => indexed_color(i),
        Color::Named(n) => match n {
            NamedColor::Foreground | NamedColor::BrightForeground => DEFAULT_FG,
            NamedColor::Background => DEFAULT_BG,
            NamedColor::Cursor => DEFAULT_FG,
            NamedColor::DimForeground => dim(DEFAULT_FG),
            other => {
                let idx = other as usize;
                if idx < 16 {
                    ANSI[idx]
                } else if (261..=268).contains(&idx) {
                    dim(ANSI[idx - 261])
                } else if is_fg {
                    DEFAULT_FG
                } else {
                    DEFAULT_BG
                }
            }
        },
    }
}

fn dim(c: Rgb8) -> Rgb8 {
    let f = |v: u8, bg: u8| ((u16::from(v) * 2 + u16::from(bg)) / 3) as u8;
    Rgb8(
        f(c.0, DEFAULT_BG.0),
        f(c.1, DEFAULT_BG.1),
        f(c.2, DEFAULT_BG.2),
    )
}

#[derive(Debug, Clone)]
pub struct TerminalOptions {
    pub shell: Option<(String, Vec<String>)>,
    pub cwd: PathBuf,
    pub cols: u16,
    pub rows: u16,
    pub cell_width: u16,
    pub cell_height: u16,
    pub scrollback: usize,
    pub env: Vec<(String, String)>,
}

impl TerminalOptions {
    pub fn new(cwd: PathBuf, cols: u16, rows: u16) -> Self {
        Self {
            shell: None,
            cwd,
            cols,
            rows,
            cell_width: 8,
            cell_height: 20,
            scrollback: DEFAULT_SCROLLBACK,
            env: Vec::new(),
        }
    }
}

pub fn default_shell() -> (String, Vec<String>) {
    if let Ok(custom) = std::env::var("NANTI_SHELL") {
        let mut parts = custom.split_whitespace().map(str::to_owned);
        if let Some(program) = parts.next() {
            return (program, parts.collect());
        }
    }
    if cfg!(windows) {
        ("powershell.exe".to_owned(), vec!["-NoLogo".to_owned()])
    } else {
        (
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned()),
            Vec::new(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    Block,
    Underline,
    Beam,
    HollowBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub col: usize,
    pub cells: usize,
    pub text: String,
    pub fg: Rgb8,
    pub bg: Option<Rgb8>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikeout: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
    pub shape: CursorShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub cols: usize,
    pub rows: usize,
    pub lines: Vec<Vec<Run>>,
    pub cursor: Option<Cursor>,
    pub display_offset: usize,
    pub history: usize,
    pub alt_screen: bool,
}

impl Snapshot {
    pub fn text_lines(&self) -> Vec<String> {
        self.lines
            .iter()
            .map(|runs| {
                let mut s = String::new();
                let mut col = 0;
                for r in runs {
                    while col < r.col {
                        s.push(' ');
                        col += 1;
                    }
                    s.push_str(&r.text);
                    col = r.col + r.cells;
                }
                s.trim_end().to_owned()
            })
            .collect()
    }

    pub fn text(&self) -> String {
        self.text_lines().join("\n")
    }
}

#[derive(Default)]
struct Shared {
    dirty: AtomicBool,
    bell: AtomicBool,
    title: Mutex<String>,
    exit: Mutex<Option<i32>>,
}

#[derive(Clone)]
struct Listener {
    shared: Arc<Shared>,
    sender: Arc<OnceLock<EventLoopSender>>,
    size: Arc<Mutex<WindowSize>>,
}

impl Listener {
    fn reply(&self, text: String) {
        if let Some(tx) = self.sender.get() {
            let _ = tx.send(Msg::Input(text.into_bytes().into()));
        }
    }
}

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        match event {
            Event::Wakeup | Event::MouseCursorDirty | Event::CursorBlinkingChange => {
                self.shared.dirty.store(true, Ordering::Release);
            }
            Event::Title(t) => {
                *self.shared.title.lock().unwrap() = t;
                self.shared.dirty.store(true, Ordering::Release);
            }
            Event::ResetTitle => self.shared.title.lock().unwrap().clear(),
            Event::Bell => self.shared.bell.store(true, Ordering::Release),
            Event::PtyWrite(text) => self.reply(text),
            Event::TextAreaSizeRequest(f) => {
                let size = *self.size.lock().unwrap();
                self.reply(f(size));
            }
            Event::ColorRequest(index, f) => {
                let c = match index {
                    256 => DEFAULT_FG,
                    257 => DEFAULT_BG,
                    i if i < 256 => indexed_color(i as u8),
                    _ => DEFAULT_FG,
                };
                self.reply(f(Rgb {
                    r: c.0,
                    g: c.1,
                    b: c.2,
                }));
            }
            // Programs must not read or write the user's clipboard behind their back.
            Event::ClipboardStore(..) | Event::ClipboardLoad(..) => {}
            Event::ChildExit(status) => {
                *self.shared.exit.lock().unwrap() = Some(status.code().unwrap_or(-1));
                self.shared.dirty.store(true, Ordering::Release);
            }
            Event::Exit => self.shared.dirty.store(true, Ordering::Release),
        }
    }
}

struct Dim {
    cols: usize,
    rows: usize,
}

impl Dimensions for Dim {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectKind {
    Char,
    Word,
    Line,
}

pub struct Terminal {
    term: Arc<FairMutex<Term<Listener>>>,
    sender: EventLoopSender,
    shared: Arc<Shared>,
    size: Arc<Mutex<WindowSize>>,
}

impl Terminal {
    pub fn spawn(opts: TerminalOptions) -> io::Result<Self> {
        let (program, args) = opts.shell.clone().unwrap_or_else(default_shell);
        let size = WindowSize {
            num_lines: opts.rows.max(1),
            num_cols: opts.cols.max(2),
            cell_width: opts.cell_width.max(1),
            cell_height: opts.cell_height.max(1),
        };
        let mut env = std::collections::HashMap::new();
        env.insert("TERM_PROGRAM".to_owned(), "nanti".to_owned());
        env.insert("COLORTERM".to_owned(), "truecolor".to_owned());
        env.extend(opts.env.iter().cloned());
        let pty_opts = tty::Options {
            shell: Some(tty::Shell::new(program, args)),
            working_directory: Some(opts.cwd.clone()),
            drain_on_exit: false,
            env,
            ..Default::default()
        };
        let pty = tty::new(&pty_opts, size, 0)?;

        let shared = Arc::new(Shared::default());
        let size_cell = Arc::new(Mutex::new(size));
        let slot = Arc::new(OnceLock::new());
        let listener = Listener {
            shared: shared.clone(),
            sender: slot.clone(),
            size: size_cell.clone(),
        };
        let config = Config {
            scrolling_history: opts.scrollback.min(100_000),
            osc52: Osc52::Disabled,
            ..Config::default()
        };
        let term = Arc::new(FairMutex::new(Term::new(
            config,
            &Dim {
                cols: size.num_cols as usize,
                rows: size.num_lines as usize,
            },
            listener.clone(),
        )));
        let event_loop = EventLoop::new(term.clone(), listener, pty, false, false)?;
        let sender = event_loop.channel();
        let _ = slot.set(event_loop.channel());
        let _ = event_loop.spawn();
        Ok(Self {
            term,
            sender,
            shared,
            size: size_cell,
        })
    }

    pub fn write(&self, bytes: impl Into<Vec<u8>>) {
        if self.is_alive() {
            let _ = self.sender.send(Msg::Input(bytes.into().into()));
        }
    }

    pub fn send_text(&self, text: &str) {
        self.write(text.as_bytes().to_vec());
    }

    pub fn paste(&self, text: &str) {
        let bracketed = self.term.lock().mode().contains(TermMode::BRACKETED_PASTE);
        self.write(paste_bytes(text, bracketed));
    }

    pub fn app_cursor(&self) -> bool {
        self.term.lock().mode().contains(TermMode::APP_CURSOR)
    }

    pub fn resize(&self, cols: u16, rows: u16, cell_width: u16, cell_height: u16) {
        let new = WindowSize {
            num_lines: rows.max(1),
            num_cols: cols.max(2),
            cell_width: cell_width.max(1),
            cell_height: cell_height.max(1),
        };
        {
            let mut cur = self.size.lock().unwrap();
            if cur.num_lines == new.num_lines && cur.num_cols == new.num_cols {
                *cur = new;
                return;
            }
            *cur = new;
        }
        self.term.lock().resize(Dim {
            cols: new.num_cols as usize,
            rows: new.num_lines as usize,
        });
        let _ = self.sender.send(Msg::Resize(new));
        self.shared.dirty.store(true, Ordering::Release);
    }

    pub fn size(&self) -> (usize, usize) {
        let t = self.term.lock();
        (t.columns(), t.screen_lines())
    }

    pub fn scroll_by(&self, lines: i32) {
        self.term.lock().scroll_display(Scroll::Delta(lines));
        self.shared.dirty.store(true, Ordering::Release);
    }

    pub fn scroll_to_bottom(&self) {
        self.term.lock().scroll_display(Scroll::Bottom);
        self.shared.dirty.store(true, Ordering::Release);
    }

    pub fn title(&self) -> String {
        self.shared.title.lock().unwrap().clone()
    }

    pub fn exit_code(&self) -> Option<i32> {
        *self.shared.exit.lock().unwrap()
    }

    pub fn is_alive(&self) -> bool {
        self.exit_code().is_none()
    }

    pub fn take_dirty(&self) -> bool {
        self.shared.dirty.swap(false, Ordering::AcqRel)
    }

    pub fn take_bell(&self) -> bool {
        self.shared.bell.swap(false, Ordering::AcqRel)
    }

    pub fn kill(&self) {
        let _ = self.sender.send(Msg::Shutdown);
    }

    fn grid_point(term: &Term<Listener>, row: usize, col: usize) -> Point {
        let offset = term.grid().display_offset() as i32;
        let row = row.min(term.screen_lines().saturating_sub(1)) as i32;
        let col = col.min(term.columns().saturating_sub(1));
        Point::new(Line(row - offset), Column(col))
    }

    pub fn selection_start(&self, row: usize, col: usize, right_half: bool, kind: SelectKind) {
        let mut term = self.term.lock();
        let p = Self::grid_point(&term, row, col);
        let ty = match kind {
            SelectKind::Char => SelectionType::Simple,
            SelectKind::Word => SelectionType::Semantic,
            SelectKind::Line => SelectionType::Lines,
        };
        let side = if right_half { Side::Right } else { Side::Left };
        term.selection = Some(Selection::new(ty, p, side));
        self.shared.dirty.store(true, Ordering::Release);
    }

    pub fn selection_update(&self, row: usize, col: usize, right_half: bool) {
        let mut term = self.term.lock();
        let p = Self::grid_point(&term, row, col);
        let side = if right_half { Side::Right } else { Side::Left };
        if let Some(sel) = term.selection.as_mut() {
            sel.update(p, side);
        }
        self.shared.dirty.store(true, Ordering::Release);
    }

    pub fn selection_clear(&self) {
        self.term.lock().selection = None;
        self.shared.dirty.store(true, Ordering::Release);
    }

    pub fn selection_text(&self) -> Option<String> {
        self.term
            .lock()
            .selection_to_string()
            .filter(|s| !s.is_empty())
    }

    pub fn snapshot(&self) -> Snapshot {
        let term = self.term.lock();
        let content = term.renderable_content();
        let (cols, rows) = (term.columns(), term.screen_lines());
        let offset = content.display_offset;
        let mut lines: Vec<Vec<Run>> = vec![Vec::new(); rows];

        for indexed in content.display_iter {
            let cell = indexed.cell;
            if cell
                .flags
                .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            {
                continue;
            }
            let row = indexed.point.line.0 + offset as i32;
            if row < 0 || row as usize >= rows {
                continue;
            }
            let row = row as usize;
            let col = indexed.point.column.0;
            let wide = cell.flags.contains(Flags::WIDE_CHAR);
            let mut fg = resolve(cell.fg, true);
            let mut bg = resolve(cell.bg, false);
            if cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }
            if cell.flags.contains(Flags::DIM) {
                fg = dim(fg);
            }
            let ch = if cell.flags.contains(Flags::HIDDEN) || cell.c == '\0' {
                ' '
            } else {
                cell.c
            };
            let selected = content
                .selection
                .is_some_and(|s| s.contains(Point::new(indexed.point.line, indexed.point.column)));
            let bg_opt = (bg != DEFAULT_BG).then_some(bg);
            let width = if wide { 2 } else { 1 };
            let bold = cell.flags.intersects(Flags::BOLD);
            let italic = cell.flags.contains(Flags::ITALIC);
            let underline = cell.flags.intersects(Flags::ALL_UNDERLINES);
            let strike = cell.flags.contains(Flags::STRIKEOUT);

            let runs = &mut lines[row];
            if let Some(last) = runs.last_mut()
                && last.col + last.cells == col
                && last.fg == fg
                && last.bg == bg_opt
                && last.bold == bold
                && last.italic == italic
                && last.underline == underline
                && last.strikeout == strike
                && last.selected == selected
            {
                last.text.push(ch);
                last.cells += width;
                continue;
            }
            runs.push(Run {
                col,
                cells: width,
                text: ch.to_string(),
                fg,
                bg: bg_opt,
                bold,
                italic,
                underline,
                strikeout: strike,
                selected,
            });
        }
        for runs in &mut lines {
            while runs.last().is_some_and(|r| {
                r.bg.is_none() && !r.selected && !r.underline && r.text.trim().is_empty()
            }) {
                runs.pop();
            }
        }

        let cursor_visible = content.mode.contains(TermMode::SHOW_CURSOR) && offset == 0;
        let cursor =
            (cursor_visible && content.cursor.shape != AlacrittyCursorShape::Hidden).then(|| {
                Cursor {
                    row: content.cursor.point.line.0.max(0) as usize,
                    col: content.cursor.point.column.0,
                    shape: match content.cursor.shape {
                        AlacrittyCursorShape::Underline => CursorShape::Underline,
                        AlacrittyCursorShape::Beam => CursorShape::Beam,
                        AlacrittyCursorShape::HollowBlock => CursorShape::HollowBlock,
                        _ => CursorShape::Block,
                    },
                }
            });
        Snapshot {
            cols,
            rows,
            lines,
            cursor,
            display_offset: offset,
            history: term.grid().history_size(),
            alt_screen: content.mode.contains(TermMode::ALT_SCREEN),
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.kill();
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyMods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl KeyMods {
    fn param(self) -> Option<u8> {
        let v = u8::from(self.shift) + 2 * u8::from(self.alt) + 4 * u8::from(self.ctrl);
        (v > 0).then_some(v + 1)
    }
}

pub fn encode_key(key: &str, mods: KeyMods, app_cursor: bool) -> Option<Vec<u8>> {
    let csi = |final_byte: char, num: Option<u8>| -> Vec<u8> {
        match (num, mods.param()) {
            (None, None) => format!("\x1b[{final_byte}"),
            (Some(n), None) => format!("\x1b[{n}{final_byte}"),
            (None, Some(m)) => format!("\x1b[1;{m}{final_byte}"),
            (Some(n), Some(m)) => format!("\x1b[{n};{m}{final_byte}"),
        }
        .into_bytes()
    };
    let arrow = |c: char| -> Vec<u8> {
        if mods.param().is_none() && app_cursor {
            format!("\x1bO{c}").into_bytes()
        } else {
            csi(c, None)
        }
    };
    let bytes = match key {
        "enter" => vec![b'\r'],
        "tab" if mods.shift => b"\x1b[Z".to_vec(),
        "tab" => vec![b'\t'],
        "backspace" if mods.ctrl => vec![0x08],
        "backspace" => vec![0x7f],
        "escape" => vec![0x1b],
        "up" => arrow('A'),
        "down" => arrow('B'),
        "right" => arrow('C'),
        "left" => arrow('D'),
        "home" => arrow('H'),
        "end" => arrow('F'),
        "insert" => csi('~', Some(2)),
        "delete" => csi('~', Some(3)),
        "pageup" => csi('~', Some(5)),
        "pagedown" => csi('~', Some(6)),
        "f1" | "f2" | "f3" | "f4" => {
            let c = ['P', 'Q', 'R', 'S'][(key.as_bytes()[1] - b'1') as usize];
            if mods.param().is_some() {
                csi(c, None)
            } else {
                format!("\x1bO{c}").into_bytes()
            }
        }
        k if k.starts_with('f') && k[1..].parse::<u8>().is_ok() => {
            let n = match k[1..].parse::<u8>().ok()? {
                5 => 15,
                6 => 17,
                7 => 18,
                8 => 19,
                9 => 20,
                10 => 21,
                11 => 23,
                12 => 24,
                _ => return None,
            };
            csi('~', Some(n))
        }
        "space" if mods.ctrl => vec![0],
        k if mods.ctrl && !mods.alt && k.chars().count() == 1 => {
            let c = k.chars().next()?.to_ascii_lowercase();
            match c {
                'a'..='z' => vec![c as u8 - b'a' + 1],
                '@' | '2' => vec![0],
                '[' => vec![0x1b],
                '\\' => vec![0x1c],
                ']' => vec![0x1d],
                '^' | '6' => vec![0x1e],
                '_' | '-' => vec![0x1f],
                _ => return None,
            }
        }
        k if mods.alt && !mods.ctrl && k.chars().count() == 1 => {
            let c = k.chars().next()?;
            let c = if mods.shift {
                c.to_ascii_uppercase()
            } else {
                c
            };
            let mut v = vec![0x1b];
            v.extend(c.to_string().into_bytes());
            v
        }
        _ => return None,
    };
    Some(bytes)
}

pub fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    if bracketed {
        let clean = text.replace("\x1b[201~", "").replace("\x1b[200~", "");
        let mut v = b"\x1b[200~".to_vec();
        v.extend(clean.into_bytes());
        v.extend(b"\x1b[201~");
        v
    } else {
        text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::{Duration, Instant};

    const NONE: KeyMods = KeyMods {
        ctrl: false,
        alt: false,
        shift: false,
    };
    const CTRL: KeyMods = KeyMods {
        ctrl: true,
        alt: false,
        shift: false,
    };
    const ALT: KeyMods = KeyMods {
        ctrl: false,
        alt: true,
        shift: false,
    };
    const SHIFT: KeyMods = KeyMods {
        ctrl: false,
        alt: false,
        shift: true,
    };

    #[test]
    fn keys_encode_like_xterm() {
        assert_eq!(encode_key("enter", NONE, false), Some(b"\r".to_vec()));
        assert_eq!(encode_key("backspace", NONE, false), Some(vec![0x7f]));
        assert_eq!(encode_key("tab", NONE, false), Some(b"\t".to_vec()));
        assert_eq!(encode_key("tab", SHIFT, false), Some(b"\x1b[Z".to_vec()));
        assert_eq!(encode_key("escape", NONE, false), Some(vec![0x1b]));
        assert_eq!(encode_key("up", NONE, false), Some(b"\x1b[A".to_vec()));
        assert_eq!(
            encode_key("up", NONE, true),
            Some(b"\x1bOA".to_vec()),
            "application cursor mode"
        );
        assert_eq!(
            encode_key("left", CTRL, true),
            Some(b"\x1b[1;5D".to_vec()),
            "modified arrows use CSI"
        );
        assert_eq!(encode_key("home", NONE, false), Some(b"\x1b[H".to_vec()));
        assert_eq!(encode_key("delete", NONE, false), Some(b"\x1b[3~".to_vec()));
        assert_eq!(encode_key("pageup", NONE, false), Some(b"\x1b[5~".to_vec()));
        assert_eq!(encode_key("f1", NONE, false), Some(b"\x1bOP".to_vec()));
        assert_eq!(encode_key("f5", NONE, false), Some(b"\x1b[15~".to_vec()));
        assert_eq!(
            encode_key("f12", SHIFT, false),
            Some(b"\x1b[24;2~".to_vec())
        );
    }

    #[test]
    fn control_and_alt_combinations() {
        assert_eq!(
            encode_key("c", CTRL, false),
            Some(vec![3]),
            "Ctrl+C = ETX (interrupt)"
        );
        assert_eq!(encode_key("a", CTRL, false), Some(vec![1]));
        assert_eq!(encode_key("z", CTRL, false), Some(vec![26]));
        assert_eq!(encode_key("[", CTRL, false), Some(vec![0x1b]));
        assert_eq!(encode_key("space", CTRL, false), Some(vec![0]));
        assert_eq!(encode_key("b", ALT, false), Some(b"\x1bb".to_vec()));
        assert_eq!(
            encode_key(
                "b",
                KeyMods {
                    alt: true,
                    shift: true,
                    ..NONE
                },
                false
            ),
            Some(b"\x1bB".to_vec())
        );
        assert_eq!(
            encode_key("a", NONE, false),
            None,
            "plain text goes through the OS text input"
        );
        assert_eq!(encode_key("é", NONE, false), None);
        assert_eq!(encode_key("f13", NONE, false), None);
    }

    #[test]
    fn paste_is_bracketed_when_asked_and_cannot_escape_the_brackets() {
        assert_eq!(paste_bytes("a\nb", false), b"a\rb".to_vec());
        assert_eq!(paste_bytes("a\r\nb\n", false), b"a\rb\r".to_vec());
        assert_eq!(paste_bytes("x", true), b"\x1b[200~x\x1b[201~".to_vec());
        let evil = "ls\x1b[201~; rm -rf /\x1b[200~";
        let out = String::from_utf8(paste_bytes(evil, true)).unwrap();
        assert_eq!(
            out, "\x1b[200~ls; rm -rf /\x1b[201~",
            "embedded markers are stripped"
        );
        assert_eq!(out.matches("\x1b[201~").count(), 1);
    }

    #[test]
    fn palette_matches_xterm() {
        assert_eq!(indexed_color(1), ANSI[1]);
        assert_eq!(indexed_color(16), Rgb8(0, 0, 0));
        assert_eq!(indexed_color(21), Rgb8(0, 0, 255));
        assert_eq!(indexed_color(196), Rgb8(255, 0, 0));
        assert_eq!(indexed_color(231), Rgb8(255, 255, 255));
        assert_eq!(indexed_color(232), Rgb8(8, 8, 8));
        assert_eq!(indexed_color(255), Rgb8(238, 238, 238));
        assert_eq!(Rgb8(1, 2, 3).to_u32(), 0x010203);
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nanti-term-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn spawn_in(dir: &Path) -> Terminal {
        Terminal::spawn(TerminalOptions::new(dir.to_path_buf(), 100, 24)).expect("spawn terminal")
    }

    fn wait_for(t: &Terminal, needle: &str, secs: u64) -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(secs) {
            if t.snapshot().text().contains(needle) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        false
    }

    fn wait_prompt(t: &Terminal) {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(30) {
            if t.snapshot()
                .text_lines()
                .iter()
                .any(|l| l.trim_end().ends_with('>'))
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        panic!("no prompt:\n{}", t.snapshot().text());
    }

    #[test]
    fn shell_starts_in_the_given_directory_and_runs_commands() {
        let dir = temp_dir("cwd");
        let t = spawn_in(&dir);
        wait_prompt(&t);
        let leaf = dir.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            t.snapshot().text().contains(&leaf),
            "prompt shows cwd:\n{}",
            t.snapshot().text()
        );
        t.send_text("echo (\"mark-\"+\"er-\"+(40+2))\r");
        assert!(wait_for(&t, "mark-er-42", 15), "{}", t.snapshot().text());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn exit_code_is_reported_and_input_after_exit_is_ignored() {
        let dir = temp_dir("exit");
        let t = spawn_in(&dir);
        wait_prompt(&t);
        assert!(t.is_alive());
        t.send_text("exit 7\r");
        let start = Instant::now();
        while t.is_alive() && start.elapsed() < Duration::from_secs(15) {
            std::thread::sleep(Duration::from_millis(30));
        }
        assert_eq!(t.exit_code(), Some(7));
        t.send_text("echo ignored\r"); // must not panic or hang
        assert!(t.take_dirty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unicode_output_and_wide_characters_render_without_spacer_cells() {
        let dir = temp_dir("uni");
        let t = spawn_in(&dir);
        wait_prompt(&t);
        t.send_text("echo ([char]0x65E5+[char]0x672C+[char]0x8A9E+'-ok')\r");
        assert!(
            wait_for(&t, "日本語-ok", 15),
            "wide chars appear contiguous:\n{}",
            t.snapshot().text()
        );
        let snap = t.snapshot();
        let run = snap
            .lines
            .iter()
            .flatten()
            .find(|r| r.text.contains("日本語"))
            .unwrap();
        assert_eq!(
            run.cells,
            run.text
                .chars()
                .map(|c| if c == '日' || c == '本' || c == '語' {
                    2
                } else {
                    1
                })
                .sum::<usize>()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn typed_unicode_reaches_the_shell_intact() {
        let dir = temp_dir("typed");
        let t = spawn_in(&dir);
        wait_prompt(&t);
        t.send_text("echo ('é'+'日'+'x')\r");
        assert!(wait_for(&t, "é日x", 15), "{}", t.snapshot().text());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resize_changes_the_grid_and_the_shell_sees_it() {
        let dir = temp_dir("resize");
        let t = spawn_in(&dir);
        wait_prompt(&t);
        assert_eq!(t.size(), (100, 24));
        t.resize(60, 10, 8, 20);
        assert_eq!(t.size(), (60, 10));
        t.send_text("echo ('w='+$Host.UI.RawUI.WindowSize.Width)\r");
        assert!(wait_for(&t, "w=60", 15), "{}", t.snapshot().text());
        let snap = t.snapshot();
        assert_eq!((snap.cols, snap.rows, snap.lines.len()), (60, 10, 10));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn large_output_does_not_hang_and_scrollback_is_capped_and_scrollable() {
        let dir = temp_dir("big");
        let mut opts = TerminalOptions::new(dir.clone(), 100, 24);
        opts.scrollback = 500;
        let t = Terminal::spawn(opts).unwrap();
        wait_prompt(&t);
        t.send_text("1..5000 | % { \"row $_\" }; echo (\"big-\"+\"done\")\r");
        let start = Instant::now();
        let mut snaps = 0;
        while !t.snapshot().text().contains("big-done") {
            snaps += 1;
            assert!(
                start.elapsed() < Duration::from_secs(60),
                "output flood timed out"
            );
            std::thread::sleep(Duration::from_millis(15));
        }
        assert!(snaps > 0);
        let snap = t.snapshot();
        assert_eq!(snap.history, 500, "scrollback is capped");
        t.scroll_by(100);
        let up = t.snapshot();
        assert_eq!(up.display_offset, 100);
        assert!(up.cursor.is_none(), "cursor hidden while scrolled back");
        assert!(up.text().contains("row 4"), "{}", up.text());
        t.scroll_to_bottom();
        assert_eq!(t.snapshot().display_offset, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ctrl_c_interrupts_a_running_command() {
        let dir = temp_dir("ctrlc");
        let t = spawn_in(&dir);
        wait_prompt(&t);
        let prompts = |t: &Terminal| {
            t.snapshot()
                .text_lines()
                .iter()
                .filter(|l| l.trim_end().ends_with('>'))
                .count()
        };
        t.send_text("echo (\"busy-\"+\"now\"); Start-Sleep 60\r");
        assert!(wait_for(&t, "busy-now", 30), "the command started");
        std::thread::sleep(Duration::from_millis(300));
        let before = prompts(&t);
        t.write(encode_key("c", CTRL, false).unwrap());
        let start = Instant::now();
        while prompts(&t) <= before {
            assert!(
                start.elapsed() < Duration::from_secs(15),
                "no prompt after Ctrl+C:
{}",
                t.snapshot().text()
            );
            std::thread::sleep(Duration::from_millis(30));
        }
        t.send_text("echo (\"after-\"+\"int\")\r");
        assert!(
            wait_for(&t, "after-int", 15),
            "shell is usable again:
{}",
            t.snapshot().text()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn shell_pid(t: &Terminal) -> u32 {
        t.send_text("echo (\"pid=\"+$PID+\"=\")\r");
        let start = Instant::now();
        loop {
            let text = t.snapshot().text();
            let found = text
                .lines()
                .filter_map(|l| l.split("pid=").nth(1))
                .filter_map(|r| r.split('=').next())
                .find_map(|n| n.trim().parse::<u32>().ok());
            if let Some(pid) = found {
                return pid;
            }
            assert!(
                start.elapsed() < Duration::from_secs(15),
                "no pid in output:
{text}"
            );
            std::thread::sleep(Duration::from_millis(30));
        }
    }

    fn wait_gone(pid: u32) -> bool {
        let start = Instant::now();
        while process_alive(pid) && start.elapsed() < Duration::from_secs(15) {
            std::thread::sleep(Duration::from_millis(100));
        }
        !process_alive(pid)
    }

    #[test]
    fn kill_ends_the_child_and_leaves_no_process_behind() {
        let dir = temp_dir("kill");
        let t = spawn_in(&dir);
        wait_prompt(&t);
        let pid = shell_pid(&t);
        assert!(process_alive(pid), "shell {pid} is running");
        t.kill();
        assert!(wait_gone(pid), "shell {pid} was terminated (no zombie)");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dropping_the_terminal_also_kills_the_shell() {
        let dir = temp_dir("drop");
        let t = spawn_in(&dir);
        wait_prompt(&t);
        let pid = shell_pid(&t);
        drop(t);
        assert!(wait_gone(pid), "shell {pid} outlived the Terminal");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn selection_copies_the_selected_text() {
        let dir = temp_dir("sel");
        let t = spawn_in(&dir);
        wait_prompt(&t);
        t.send_text("echo (\"sel-\"+\"text-\"+\"123\")\r");
        assert!(wait_for(&t, "sel-text-123", 15));
        let snap = t.snapshot();
        let (row, line) = snap
            .text_lines()
            .into_iter()
            .enumerate()
            .find(|(_, l)| l.trim() == "sel-text-123")
            .expect("output row");
        let start = line.find("sel").unwrap();
        t.selection_start(row, start, false, SelectKind::Char);
        t.selection_update(row, start + 7, true);
        assert_eq!(t.selection_text().as_deref(), Some("sel-text"));
        assert!(t.snapshot().lines[row].iter().any(|r| r.selected));
        t.selection_start(row, start + 2, false, SelectKind::Word);
        assert!(t.selection_text().unwrap().contains("sel"));
        t.selection_clear();
        assert!(t.selection_text().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_directory_is_an_error_not_a_hang() {
        let r = Terminal::spawn(TerminalOptions::new(
            PathBuf::from("Z:\\definitely\\missing"),
            80,
            24,
        ));
        if let Ok(t) = r {
            let start = Instant::now();
            while t.is_alive() && start.elapsed() < Duration::from_secs(15) {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    fn process_alive(pid: u32) -> bool {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
}
