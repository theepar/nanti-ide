//! The terminal panel (TR-01..03): sessions, rendering of the emulator grid, keyboard/mouse/IME
//! input and process lifetime. The PTYs live here (not in `Model`) so the model stays testable
//! without spawning processes.

use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

use gpui::{
    AnyElement, Bounds, ClickEvent, ClipboardItem, Context, ElementInputHandler, FocusHandle,
    FontWeight, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, ScrollDelta, ScrollWheelEvent, SharedString, Window, canvas, div,
    point, prelude::*, px, rgb, rgba, size,
};
use terminal::{CursorShape, KeyMods, SelectKind, Terminal, TerminalOptions, encode_key};

use crate::model::{PanelTab, TerminalRequest};
use crate::palette::Command;
use crate::{FONT_FAMILY, FONT_SIZE, LINE_HEIGHT, Target, Workbench, theme, x_of_col};

/// Initial grid size before the panel has been measured.
const DEFAULT_COLS: u16 = 100;
const DEFAULT_ROWS: u16 = 10;
const SELECTION_BG: u32 = 0x264f78;

pub struct TerminalSession {
    pub term: Terminal,
    pub name: String,
    /// IME pre-edit text shown at the cursor; it only reaches the shell when committed.
    pub preedit: Option<String>,
    selecting: bool,
}

/// Where the terminal grid was last painted, shared with the paint closure.
#[derive(Clone)]
pub struct GridMetrics {
    pub area: Rc<Cell<Bounds<Pixels>>>,
    pub cell_width: Rc<Cell<f32>>,
}

impl Default for GridMetrics {
    fn default() -> Self {
        Self {
            area: Rc::new(Cell::new(Bounds::default())),
            cell_width: Rc::new(Cell::new(8.0)),
        }
    }
}

impl Workbench {
    // ---- inspection (used by tests and the status line) -------------------------------------

    pub fn terminal_count(&self) -> usize {
        self.terminals.len()
    }

    /// Visible text of the active terminal.
    pub fn terminal_text(&self) -> Option<String> {
        self.active_terminal().map(|s| s.term.snapshot().text())
    }

    pub fn terminal_exit_code(&self) -> Option<i32> {
        self.active_terminal().and_then(|s| s.term.exit_code())
    }

    pub fn terminal_alive(&self) -> Option<bool> {
        self.active_terminal().map(|s| s.term.is_alive())
    }

    pub fn terminal_selection(&self) -> Option<String> {
        self.active_terminal().and_then(|s| s.term.selection_text())
    }

    /// Plain-text snapshot of terminal `index`.
    pub fn terminal_text_of(&self, index: usize) -> Option<String> {
        self.terminals.get(index).map(|s| s.term.snapshot().text())
    }

    pub(crate) fn active_terminal(&self) -> Option<&TerminalSession> {
        self.terminals.get(self.active_terminal)
    }

    // ---- lifecycle ----------------------------------------------------------------------------

    fn grid_size(&self) -> (u16, u16) {
        let area = self.term_metrics.area.get();
        let cw = self.term_metrics.cell_width.get().max(1.0);
        let (w, h) = (f32::from(area.size.width), f32::from(area.size.height));
        if w < cw * 10.0 || h < LINE_HEIGHT * 2.0 {
            return (DEFAULT_COLS, DEFAULT_ROWS);
        }
        ((w / cw).floor() as u16, (h / LINE_HEIGHT).floor() as u16)
    }

    fn spawn_terminal(&mut self) {
        let (cols, rows) = self.grid_size();
        let mut opts = TerminalOptions::new(self.model.root.clone(), cols, rows);
        opts.cell_width = self.term_metrics.cell_width.get() as u16;
        opts.cell_height = LINE_HEIGHT as u16;
        match Terminal::spawn(opts) {
            Ok(term) => {
                self.term_counter += 1;
                self.terminals.push(TerminalSession {
                    term,
                    name: format!("terminal {}", self.term_counter),
                    preedit: None,
                    selecting: false,
                });
                self.active_terminal = self.terminals.len() - 1;
            }
            Err(e) => {
                self.model.message = Some(format!("Cannot start terminal: {e}"));
                self.model.terminal_focused = false;
            }
        }
    }

    /// Close terminal `index`; dropping the session kills its shell.
    pub fn close_terminal(&mut self, index: usize) {
        if index < self.terminals.len() {
            self.terminals.remove(index);
            self.active_terminal = self
                .active_terminal
                .saturating_sub(usize::from(
                    index <= self.active_terminal && self.active_terminal > 0,
                ))
                .min(self.terminals.len().saturating_sub(1));
        }
        if self.terminals.is_empty() {
            self.model.terminal_focused = false;
        }
    }

    /// Kill every shell (window close, app exit).
    pub fn kill_all_terminals(&mut self) {
        self.terminals.clear();
        self.model.terminal_focused = false;
    }

    /// Carry out what a command asked for (`Model` cannot spawn processes).
    pub(crate) fn process_terminal_request(&mut self) {
        match self.model.terminal_request.take() {
            Some(TerminalRequest::Ensure) => {
                if self.terminals.is_empty() {
                    self.spawn_terminal();
                }
            }
            Some(TerminalRequest::New) => self.spawn_terminal(),
            Some(TerminalRequest::KillActive) => {
                let i = self.active_terminal;
                self.close_terminal(i);
            }
            None => {}
        }
    }

    /// Poll for output/exit and keep every PTY the size of the panel. True = redraw needed.
    pub(crate) fn pump_terminals(&mut self) -> bool {
        if self.terminals.is_empty() {
            return false;
        }
        let (cols, rows) = self.grid_size();
        let cw = self.term_metrics.cell_width.get() as u16;
        let mut changed = false;
        for s in &self.terminals {
            let before = s.term.size();
            s.term.resize(cols, rows, cw, LINE_HEIGHT as u16);
            changed |= s.term.size() != before;
            changed |= s.term.take_dirty();
            let _ = s.term.take_bell();
        }
        changed
    }

    // ---- keyboard -----------------------------------------------------------------------------

    pub(crate) fn on_terminal_key(
        &mut self,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let k = &ev.keystroke;
        let m = k.modifiers;
        let key = k.key.as_str();
        let ctrl = m.control && !m.alt;

        // A few shortcuts keep working while the shell has the keyboard (as in VS Code's terminal).
        if ctrl {
            match (key, m.shift) {
                ("c", _) => return self.terminal_copy_or_interrupt(m.shift, cx),
                ("v", _) => return self.terminal_paste(cx),
                _ => {
                    if let Some(
                        c @ (Command::CommandPalette
                        | Command::QuickOpen
                        | Command::ToggleSidebar
                        | Command::ToggleTerminal
                        | Command::NewTerminal
                        | Command::TogglePanel
                        | Command::FindInFiles),
                    ) = Command::from_ctrl_key(key, m.shift)
                    {
                        return self.run_and_stop(c, window, cx);
                    }
                }
            }
        }
        let Some(session) = self.terminals.get(self.active_terminal) else {
            return;
        };
        let mods = KeyMods {
            ctrl,
            alt: m.alt && !m.control,
            shift: m.shift,
        };
        let Some(bytes) = encode_key(key, mods, session.term.app_cursor()) else {
            return; // printable text: delivered through the InputHandler
        };
        session.term.selection_clear();
        session.term.scroll_to_bottom();
        session.term.write(bytes);
        cx.stop_propagation();
        cx.notify();
    }

    fn terminal_copy_or_interrupt(&mut self, force_copy: bool, cx: &mut Context<Self>) {
        let Some(session) = self.terminals.get(self.active_terminal) else {
            return;
        };
        match session.term.selection_text() {
            Some(text) => {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                session.term.selection_clear();
            }
            // Ctrl+C is the interrupt key; Ctrl+Shift+C only ever copies.
            None if !force_copy => {
                session.term.scroll_to_bottom();
                session.term.write(vec![0x03]);
            }
            None => {}
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn terminal_paste(&mut self, cx: &mut Context<Self>) {
        if let (Some(s), Some(text)) = (
            self.terminals.get(self.active_terminal),
            cx.read_from_clipboard().and_then(|c| c.text()),
        ) {
            s.term.selection_clear();
            s.term.scroll_to_bottom();
            s.term.paste(&text);
        }
        cx.stop_propagation();
        cx.notify();
    }

    // ---- text input (typing and IME) ----------------------------------------------------------

    pub(crate) fn term_commit(&mut self, text: &str) {
        if let Some(s) = self.terminals.get_mut(self.active_terminal) {
            s.preedit = None;
            s.term.selection_clear();
            s.term.scroll_to_bottom();
            s.term.send_text(text);
        }
    }

    pub(crate) fn term_compose(&mut self, text: &str) {
        if let Some(s) = self.terminals.get_mut(self.active_terminal) {
            s.preedit = (!text.is_empty()).then(|| text.to_owned());
        }
    }

    pub(crate) fn term_unmark(&mut self) {
        if let Some(s) = self.terminals.get_mut(self.active_terminal) {
            s.preedit = None;
        }
    }

    pub(crate) fn term_marked_utf16(&self) -> Option<Range<usize>> {
        let p = self.active_terminal()?.preedit.as_ref()?;
        Some(0..p.encode_utf16().count())
    }

    pub(crate) fn term_cursor_bounds(&self) -> Option<Bounds<Pixels>> {
        let s = self.active_terminal()?;
        let cur = s.term.snapshot().cursor?;
        let area = self.term_metrics.area.get();
        let cw = self.term_metrics.cell_width.get();
        Some(Bounds {
            origin: point(
                area.origin.x + px(cw * cur.col as f32),
                area.origin.y + px(LINE_HEIGHT * cur.row as f32),
            ),
            size: size(px(cw), px(LINE_HEIGHT)),
        })
    }

    // ---- mouse --------------------------------------------------------------------------------

    /// Terminal cell (row, col, right half?) under a window position.
    fn term_cell_at(&self, p: Point<Pixels>) -> (usize, usize, bool) {
        let area = self.term_metrics.area.get();
        let cw = self.term_metrics.cell_width.get().max(1.0);
        let x = f32::from(p.x - area.origin.x).max(0.0);
        let y = f32::from(p.y - area.origin.y).max(0.0);
        let col = (x / cw).floor();
        (
            (y / LINE_HEIGHT).floor() as usize,
            col as usize,
            (x / cw - col) >= 0.5,
        )
    }

    fn on_term_mouse_down(
        &mut self,
        ev: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_target(Target::Terminal, window);
        if ev.button != MouseButton::Left {
            return cx.notify();
        }
        let (row, col, right) = self.term_cell_at(ev.position);
        if let Some(s) = self.terminals.get_mut(self.active_terminal) {
            let kind = match ev.click_count {
                1 => SelectKind::Char,
                2 => SelectKind::Word,
                _ => SelectKind::Line,
            };
            s.term.selection_start(row, col, right, kind);
            s.selecting = true;
        }
        cx.notify();
    }

    fn on_term_mouse_move(&mut self, ev: &MouseMoveEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if ev.pressed_button != Some(MouseButton::Left) {
            return;
        }
        let (row, col, right) = self.term_cell_at(ev.position);
        if let Some(s) = self.terminals.get(self.active_terminal)
            && s.selecting
        {
            s.term.selection_update(row, col, right);
            cx.notify();
        }
    }

    fn on_term_mouse_up(&mut self, _: &MouseUpEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some(s) = self.terminals.get_mut(self.active_terminal) {
            s.selecting = false;
        }
        cx.notify();
    }

    fn on_term_scroll(&mut self, ev: &ScrollWheelEvent, _w: &mut Window, cx: &mut Context<Self>) {
        let lines = match ev.delta {
            ScrollDelta::Lines(l) => l.y.round() as i32,
            ScrollDelta::Pixels(p) => (f32::from(p.y) / LINE_HEIGHT).round() as i32,
        };
        if lines != 0
            && let Some(s) = self.terminals.get(self.active_terminal)
        {
            s.term.scroll_by(lines);
            cx.notify();
        }
    }

    // ---- rendering ----------------------------------------------------------------------------

    /// Tabs of the open terminals plus a "+" button.
    pub(crate) fn terminal_strip(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap_1()
            .px_2()
            .h(px(24.))
            .text_size(px(12.))
            .children(self.terminals.iter().enumerate().map(|(i, s)| {
                let active = i == self.active_terminal;
                let label = {
                    let title = s.term.title();
                    let base = if title.is_empty() {
                        s.name.clone()
                    } else {
                        title
                    };
                    match s.term.exit_code() {
                        Some(c) => format!("{base} (exited {c})"),
                        None => base,
                    }
                };
                div()
                    .id(("term-tab", i))
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .rounded_sm()
                    .cursor_pointer()
                    .when(active, |d| {
                        d.bg(rgb(theme::SELECTED)).text_color(rgb(theme::TEXT))
                    })
                    .when(!active, |d| {
                        d.text_color(rgb(theme::TEXT_DIM))
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.active_terminal = i;
                        this.focus_target(Target::Terminal, window);
                        cx.notify();
                    }))
                    .child(SharedString::from(label))
                    .child(
                        div()
                            .id(("term-close", i))
                            .px_1()
                            .rounded_sm()
                            .hover(|s| s.bg(rgb(theme::HOVER)))
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                cx.stop_propagation();
                                this.close_terminal(i);
                                this.sync_focus(window);
                                cx.notify();
                            }))
                            .child("×"),
                    )
            }))
            .child(
                div()
                    .id("term-new")
                    .px_2()
                    .rounded_sm()
                    .cursor_pointer()
                    .text_color(rgb(theme::TEXT_DIM))
                    .hover(|s| s.bg(rgb(theme::HOVER)).text_color(rgb(theme::TEXT)))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.apply_command(Command::NewTerminal, window, cx);
                    }))
                    .child("+ New"),
            )
    }

    /// The emulator grid (or a hint when no terminal is open).
    pub(crate) fn terminal_view(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(session) = self.active_terminal() else {
            return div()
                .flex_1()
                .px_3()
                .py_2()
                .text_size(px(13.))
                .text_color(rgb(theme::TEXT_DIM))
                .child("No terminal open. Press Ctrl+` or click \"+ New\".")
                .into_any_element();
        };
        let snap = session.term.snapshot();
        let cw = x_of_col(window, &"m".repeat(40), 40) / 40.0;
        self.term_metrics.cell_width.set(f32::from(cw));
        let cw = f32::from(cw);
        let focused = self.target() == Target::Terminal;

        let rows: Vec<AnyElement> = snap
            .lines
            .iter()
            .enumerate()
            .map(|(r, runs)| {
                div()
                    .absolute()
                    .top(px(LINE_HEIGHT * r as f32))
                    .left_0()
                    .w_full()
                    .h(px(LINE_HEIGHT))
                    .children(runs.iter().map(|run| {
                        let (fg, bg) = if run.selected {
                            (0xffffff, Some(SELECTION_BG))
                        } else {
                            (run.fg.to_u32(), run.bg.map(|c| c.to_u32()))
                        };
                        div()
                            .absolute()
                            .top_0()
                            .left(px(cw * run.col as f32))
                            .w(px(cw * run.cells as f32))
                            .h(px(LINE_HEIGHT))
                            .when_some(bg, |d, c| d.bg(rgb(c)))
                            .text_color(rgb(fg))
                            .when(run.bold, |d| d.font_weight(FontWeight::BOLD))
                            .when(run.italic, |d| d.italic())
                            .when(run.underline, |d| d.underline())
                            .when(run.strikeout, |d| d.line_through())
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .child(SharedString::from(run.text.clone()))
                    }))
                    .into_any_element()
            })
            .collect();

        let cursor = snap.cursor.as_ref().map(|c| {
            let base = div()
                .absolute()
                .left(px(cw * c.col as f32))
                .top(px(LINE_HEIGHT * c.row as f32));
            match (c.shape, focused) {
                (CursorShape::Beam, true) => base.w(px(2.)).h(px(LINE_HEIGHT)).bg(rgb(theme::TEXT)),
                (CursorShape::Underline, true) => base
                    .w(px(cw))
                    .h(px(2.))
                    .mt(px(LINE_HEIGHT - 2.))
                    .bg(rgb(theme::TEXT)),
                (_, true) => base.w(px(cw)).h(px(LINE_HEIGHT)).bg(rgba(0xccccccaa)),
                (_, false) => base
                    .w(px(cw))
                    .h(px(LINE_HEIGHT))
                    .border_1()
                    .border_color(rgb(theme::TEXT_DIM)),
            }
        });
        let preedit = session
            .preedit
            .as_ref()
            .zip(snap.cursor.as_ref())
            .map(|(text, c)| {
                let w = x_of_col(window, text, text.chars().count());
                div()
                    .absolute()
                    .left(px(cw * c.col as f32))
                    .top(px(LINE_HEIGHT * c.row as f32))
                    .w(w)
                    .h(px(LINE_HEIGHT))
                    .bg(rgb(theme::PANEL))
                    .text_color(rgb(theme::TEXT))
                    .underline()
                    .whitespace_nowrap()
                    .child(SharedString::from(text.clone()))
            });

        // The canvas records where the grid is painted (for resizing and mouse mapping) and
        // registers the OS text input while the terminal has focus.
        let metrics = self.term_metrics.clone();
        let focus = self.terminal_focus.clone();
        let entity = cx.entity();
        let input_canvas = canvas(
            |_, _, _| (),
            move |bounds, (), window, cx| {
                metrics.area.set(bounds);
                window.handle_input(&focus, ElementInputHandler::new(bounds, entity), cx);
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        div()
            .id("terminal-grid")
            .track_focus(&self.terminal_focus)
            .relative()
            .flex_1()
            .min_h_0()
            .mx(px(8.))
            .overflow_hidden()
            .font_family(FONT_FAMILY)
            .text_size(px(FONT_SIZE))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_term_mouse_down))
            .on_mouse_move(cx.listener(Self::on_term_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_term_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_term_scroll))
            .children(rows)
            .children(cursor)
            .children(preedit)
            .child(input_canvas)
            .into_any_element()
    }

    /// Show the Terminal tab and give it the keyboard (used by the panel tab click).
    pub(crate) fn open_terminal_tab(&mut self, window: &mut Window) {
        self.model.panel_tab = PanelTab::Terminal;
        self.model.terminal_focused = true;
        self.model.terminal_request = Some(TerminalRequest::Ensure);
        self.process_terminal_request();
        self.sync_focus(window);
    }

    #[allow(dead_code)]
    pub(crate) fn terminal_focus_handle(&self) -> &FocusHandle {
        &self.terminal_focus
    }
}
