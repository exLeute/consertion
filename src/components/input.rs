//! Text input: cursor, selection (keyboard + mouse), word jumps, clipboard, undo and an optional
//! filter that rejects invalid edits. Single-line (scrolls sideways) or `multiline` (word-wraps,
//! scrolls vertically; Enter inserts a newline).

use super::hit;
use crate::clipboard::Clipboard;
use crate::theme::Skin;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;
use std::time::Instant;
use unicode_width::UnicodeWidthChar;

#[derive(Debug, PartialEq)]
pub enum InputEvent {
    /// Text changed.
    Changed,
    /// Key used (cursor moved, copied, …) but text unchanged.
    Handled,
    /// Edit refused by the filter.
    Rejected,
    /// Not an input key; the caller may use it.
    Ignored,
}

type Filter = Box<dyn Fn(&str) -> bool>;

/// What the last edit was, so a run of typed characters undoes as one step.
#[derive(Clone, Copy, PartialEq)]
enum Edit {
    None,
    Typing,
    Deleting,
    External,
    Other,
}

const UNDO_LIMIT: usize = 100;

pub struct TextInput {
    text: Vec<char>,
    cursor: usize,
    /// Selection is the range between `anchor` and `cursor`.
    anchor: Option<usize>,
    scroll: usize,
    pub placeholder: String,
    pub prefix: &'static str,
    filter: Option<Filter>,
    /// Applied to pasted text before filtering (e.g. strip thousands separators).
    pub paste_cleaner: Option<fn(&str) -> String>,
    /// Where the editable text was last drawn.
    rect: Rect,
    outer: Rect,
    pub rejected_at: Option<Instant>,
    last_click: Option<Instant>,
    dragging: bool,
    undo: Vec<(Vec<char>, usize)>,
    redo: Vec<(Vec<char>, usize)>,
    last_edit: Edit,
    pub multiline: bool,
    /// First visible visual line (multiline).
    vscroll: usize,
    /// Keep the cursor in view; wheel scrolling turns this off until the next key.
    follow: bool,
}

impl TextInput {
    pub fn new(placeholder: &str) -> Self {
        Self {
            text: Vec::new(),
            cursor: 0,
            anchor: None,
            scroll: 0,
            placeholder: placeholder.into(),
            prefix: "",
            filter: None,
            paste_cleaner: None,
            rect: Rect::default(),
            outer: Rect::default(),
            rejected_at: None,
            last_click: None,
            dragging: false,
            undo: Vec::new(),
            redo: Vec::new(),
            last_edit: Edit::None,
            multiline: false,
            vscroll: 0,
            follow: true,
        }
    }

    pub fn multiline(placeholder: &str) -> Self {
        Self { multiline: true, ..Self::new(placeholder) }
    }

    /// Also clears undo history: old states may not pass the new filter.
    pub fn set_filter(&mut self, f: Option<Filter>) {
        self.filter = f;
        self.undo.clear();
        self.redo.clear();
    }

    /// Snapshot the text before an edit; edits of the same kind in a row share one snapshot.
    fn record(&mut self, kind: Edit) {
        if kind == Edit::Other || kind != self.last_edit {
            if self.undo.len() >= UNDO_LIMIT {
                self.undo.remove(0);
            }
            self.undo.push((self.text.clone(), self.cursor));
        }
        self.redo.clear();
        self.last_edit = kind;
    }

    fn restore(&mut self, redo: bool) -> InputEvent {
        let (from, to) = if redo { (&mut self.redo, &mut self.undo) } else { (&mut self.undo, &mut self.redo) };
        let Some((text, cursor)) = from.pop() else { return InputEvent::Handled };
        to.push((std::mem::replace(&mut self.text, text), self.cursor));
        self.cursor = cursor.min(self.text.len());
        self.anchor = None;
        self.last_edit = Edit::None;
        InputEvent::Changed
    }

    pub fn undo(&mut self) -> InputEvent {
        self.restore(false)
    }

    pub fn redo(&mut self) -> InputEvent {
        self.restore(true)
    }

    /// Where the input (with its frame, if any) was last drawn.
    pub fn area(&self) -> Rect {
        self.outer
    }

    /// The selection, or the whole text when nothing is selected.
    pub fn copy_text(&self) -> String {
        self.selected_text().unwrap_or_else(|| self.text())
    }

    pub fn text(&self) -> String {
        self.text.iter().collect()
    }

    /// Replace the text from outside (computed values, sample input). Undoable as one step.
    pub fn set_text(&mut self, s: &str) {
        let next: Vec<char> = s.chars().collect();
        if next != self.text {
            self.record(Edit::External);
        }
        self.text = next;
        self.cursor = self.text.len();
        self.anchor = None;
        self.scroll = 0;
        self.vscroll = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn selection(&self) -> Option<(usize, usize)> {
        let a = self.anchor?;
        (a != self.cursor).then(|| (a.min(self.cursor), a.max(self.cursor)))
    }

    pub fn selected_text(&self) -> Option<String> {
        self.selection().map(|(a, b)| self.text[a..b].iter().collect())
    }

    pub fn select_all(&mut self) {
        self.anchor = Some(0);
        self.cursor = self.text.len();
    }

    /// Replace the selection (or insert at the cursor) if the result passes the filter.
    pub fn insert(&mut self, s: &str) -> InputEvent {
        let (a, b) = self.selection().unwrap_or((self.cursor, self.cursor));
        let ins: Vec<char> = s.chars().filter(|&c| !c.is_control() || (self.multiline && c == '\n')).collect();
        let mut next = self.text[..a].to_vec();
        next.extend(&ins);
        next.extend(&self.text[b..]);
        if let Some(f) = &self.filter
            && !f(&next.iter().collect::<String>()) {
                self.rejected_at = Some(Instant::now());
                return InputEvent::Rejected;
            }
        let typing = ins.len() == 1 && !ins[0].is_whitespace();
        // Typing over a selection starts a new undo step that the following keystrokes join.
        self.record(if typing && a == b { Edit::Typing } else { Edit::Other });
        if typing {
            self.last_edit = Edit::Typing;
        }
        self.text = next;
        self.cursor = a + ins.len();
        self.anchor = None;
        InputEvent::Changed
    }

    pub fn paste(&mut self, s: &str) -> InputEvent {
        if self.multiline {
            return self.insert(&s.replace("\r\n", "\n").replace(['\r'], "\n").replace('\t', "    "));
        }
        let first_line = s.lines().next().unwrap_or("");
        let cleaned = self.paste_cleaner.map_or(first_line.to_string(), |f| f(first_line));
        self.insert(&cleaned)
    }

    fn delete_range(&mut self, a: usize, b: usize) -> InputEvent {
        if a == b {
            return InputEvent::Handled;
        }
        let mut next = self.text.clone();
        next.drain(a..b);
        // Deleting can still produce invalid input (e.g. "1e5" → "e5"), so check it too.
        if let Some(f) = &self.filter
            && !f(&next.iter().collect::<String>()) {
                self.rejected_at = Some(Instant::now());
                return InputEvent::Rejected;
            }
        self.record(if b - a == 1 { Edit::Deleting } else { Edit::Other });
        self.text = next;
        self.cursor = a;
        self.anchor = None;
        InputEvent::Changed
    }

    fn word_left(&self, mut i: usize) -> usize {
        while i > 0 && !self.text[i - 1].is_alphanumeric() {
            i -= 1;
        }
        while i > 0 && self.text[i - 1].is_alphanumeric() {
            i -= 1;
        }
        i
    }

    fn word_right(&self, mut i: usize) -> usize {
        let n = self.text.len();
        while i < n && !self.text[i].is_alphanumeric() {
            i += 1;
        }
        while i < n && self.text[i].is_alphanumeric() {
            i += 1;
        }
        i
    }

    fn move_to(&mut self, pos: usize, select: bool) {
        self.last_edit = Edit::None;
        if select {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = pos.min(self.text.len());
    }

    /// Visual lines as (start, end) char ranges at a wrap width; `end` excludes the newline.
    /// Lines wrap after the last space that fits, or mid-word when a word is longer than a line.
    fn lines(&self, width: usize) -> Vec<(usize, usize)> {
        let width = width.max(1);
        let mut out = Vec::new();
        let (mut start, mut w, mut space) = (0, 0, None);
        let mut i = 0;
        while i < self.text.len() {
            let c = self.text[i];
            if c == '\n' {
                out.push((start, i));
                (start, w, space) = (i + 1, 0, None);
                i += 1;
                continue;
            }
            let cw = c.width().unwrap_or(0);
            if w + cw > width && i > start {
                let br = match space {
                    Some(sp) if sp >= start => sp + 1,
                    _ => i,
                };
                out.push((start, br));
                start = br;
                w = self.text[start..i].iter().map(|c| c.width().unwrap_or(0)).sum();
                space = None;
                continue;
            }
            if c == ' ' {
                space = Some(i);
            }
            w += cw;
            i += 1;
        }
        out.push((start, self.text.len()));
        out
    }

    fn row_of(lines: &[(usize, usize)], i: usize) -> usize {
        lines.iter().rposition(|l| l.0 <= i).unwrap_or(0)
    }

    /// Last cursor position on visual line `r` (before the wrap point of a soft-wrapped line).
    fn line_end(lines: &[(usize, usize)], r: usize) -> usize {
        let (start, end) = lines[r];
        if lines.get(r + 1).is_some_and(|n| n.0 == end) { end.saturating_sub(1).max(start) } else { end }
    }

    fn index_in_line(&self, lines: &[(usize, usize)], r: usize, col: usize) -> usize {
        let (start, _) = lines[r];
        let end = Self::line_end(lines, r);
        let mut w = 0;
        for i in start..end {
            let cw = self.text[i].width().unwrap_or(0);
            if w + cw.div_ceil(2).max(1) > col {
                return i;
            }
            w += cw;
        }
        end
    }

    fn col_of(&self, lines: &[(usize, usize)], i: usize) -> usize {
        let start = lines[Self::row_of(lines, i)].0;
        self.text[start..i].iter().map(|c| c.width().unwrap_or(0)).sum()
    }

    fn wrap_width(&self) -> usize {
        (self.rect.width as usize).max(1)
    }

    /// Up/Down/Home/End within wrapped lines.
    fn vertical(&mut self, code: KeyCode, ctrl: bool, shift: bool) -> InputEvent {
        let lines = self.lines(self.wrap_width());
        let r = Self::row_of(&lines, self.cursor);
        let col = self.col_of(&lines, self.cursor);
        let to = match code {
            KeyCode::Home if ctrl => 0,
            KeyCode::End if ctrl => self.text.len(),
            KeyCode::Home => lines[r].0,
            KeyCode::End => Self::line_end(&lines, r),
            KeyCode::Up if r == 0 => 0,
            KeyCode::Up => self.index_in_line(&lines, r - 1, col),
            KeyCode::Down if r + 1 >= lines.len() => self.text.len(),
            KeyCode::Down => self.index_in_line(&lines, r + 1, col),
            KeyCode::PageUp => self.index_in_line(&lines, r.saturating_sub(self.rect.height as usize), col),
            KeyCode::PageDown => self.index_in_line(&lines, (r + self.rect.height as usize).min(lines.len() - 1), col),
            _ => return InputEvent::Ignored,
        };
        self.move_to(to, shift);
        InputEvent::Handled
    }

    pub fn handle_key(&mut self, key: KeyEvent, clip: &mut Clipboard) -> InputEvent {
        self.follow = true;
        if self.multiline {
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            match key.code {
                KeyCode::Enter => return self.insert("\n"),
                KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End | KeyCode::PageUp | KeyCode::PageDown => {
                    return self.vertical(key.code, ctrl, key.modifiers.contains(KeyModifiers::SHIFT));
                }
                _ => {}
            }
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let word = ctrl || alt;
        match key.code {
            KeyCode::Char(c) if !ctrl && !alt => self.insert(&c.to_string()),
            KeyCode::Char('z' | 'Z') if ctrl && shift => self.redo(),
            KeyCode::Char('z') if ctrl => self.undo(),
            KeyCode::Char('y') if ctrl => self.redo(),
            KeyCode::Char('a') if ctrl => {
                self.select_all();
                InputEvent::Handled
            }
            KeyCode::Char('c') if ctrl => match self.selected_text() {
                Some(t) => {
                    clip.set(&t);
                    InputEvent::Handled
                }
                None => InputEvent::Ignored,
            },
            KeyCode::Char('x') if ctrl => match self.selection() {
                Some((a, b)) => {
                    clip.set(&self.text[a..b].iter().collect::<String>());
                    self.delete_range(a, b)
                }
                None => InputEvent::Handled,
            },
            KeyCode::Char('v') if ctrl => {
                let s = clip.get();
                self.paste(&s)
            }
            KeyCode::Char('w') | KeyCode::Char('h') if ctrl => {
                let to = self.word_left(self.cursor);
                self.delete_range(to, self.cursor)
            }
            KeyCode::Char('u') if ctrl => self.delete_range(0, self.cursor),
            KeyCode::Char('k') if ctrl => self.delete_range(self.cursor, self.text.len()),
            KeyCode::Backspace => match self.selection() {
                Some((a, b)) => self.delete_range(a, b),
                None if word => self.delete_range(self.word_left(self.cursor), self.cursor),
                None => self.delete_range(self.cursor.saturating_sub(1), self.cursor),
            },
            KeyCode::Delete => match self.selection() {
                Some((a, b)) => self.delete_range(a, b),
                None if word => self.delete_range(self.cursor, self.word_right(self.cursor)),
                None => self.delete_range(self.cursor, (self.cursor + 1).min(self.text.len())),
            },
            KeyCode::Left => {
                let to = match self.selection() {
                    Some((a, _)) if !shift && !word => a,
                    _ if word => self.word_left(self.cursor),
                    _ => self.cursor.saturating_sub(1),
                };
                self.move_to(to, shift);
                InputEvent::Handled
            }
            KeyCode::Right => {
                let to = match self.selection() {
                    Some((_, b)) if !shift && !word => b,
                    _ if word => self.word_right(self.cursor),
                    _ => self.cursor + 1,
                };
                self.move_to(to, shift);
                InputEvent::Handled
            }
            KeyCode::Home => {
                self.move_to(0, shift);
                InputEvent::Handled
            }
            KeyCode::End => {
                self.move_to(self.text.len(), shift);
                InputEvent::Handled
            }
            _ => InputEvent::Ignored,
        }
    }

    fn index_at(&self, x: u16, y: u16) -> usize {
        if self.multiline {
            let lines = self.lines(self.wrap_width());
            let r = (self.vscroll + y.saturating_sub(self.rect.y) as usize).min(lines.len() - 1);
            return self.index_in_line(&lines, r, x.saturating_sub(self.rect.x) as usize);
        }
        let mut col = self.rect.x;
        for i in self.scroll..self.text.len() {
            let w = self.text[i].width().unwrap_or(0) as u16;
            if x < col + w.div_ceil(2).max(1) {
                return i;
            }
            col += w;
        }
        self.text.len()
    }

    /// Returns true if the event was inside the input (caller should focus it).
    pub fn handle_mouse(&mut self, m: MouseEvent) -> bool {
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) if hit(self.outer, m.column, m.row) => {
                let double = self.last_click.is_some_and(|t| t.elapsed().as_millis() < 400);
                self.last_click = Some(Instant::now());
                self.last_edit = Edit::None;
                if double {
                    self.select_all();
                } else {
                    self.cursor = self.index_at(m.column, m.row);
                    self.anchor = Some(self.cursor);
                    self.dragging = true;
                }
                true
            }
            MouseEventKind::Drag(MouseButton::Left) if self.dragging => {
                // Dragging past the edges scrolls the text.
                if m.column < self.rect.x && !self.multiline {
                    self.cursor = self.cursor.saturating_sub(1);
                } else {
                    self.cursor = self.index_at(m.column, m.row);
                }
                self.follow = true;
                true
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown if self.multiline && hit(self.outer, m.column, m.row) => {
                let n = self.lines(self.wrap_width()).len();
                let max = n.saturating_sub(self.rect.height as usize);
                self.vscroll = if m.kind == MouseEventKind::ScrollUp { self.vscroll.saturating_sub(3) } else { (self.vscroll + 3).min(max) };
                self.follow = false;
                true
            }
            MouseEventKind::Up(MouseButton::Left) if self.dragging => {
                self.dragging = false;
                true
            }
            _ => false,
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, skin: &Skin, focused: bool, title: Line) {
        if self.multiline {
            return self.render_multi(f, area, skin, focused);
        }
        self.outer = area;
        let rejected = self.rejected_at.is_some_and(|t| t.elapsed().as_millis() < 350);
        let inner = if area.height >= 3 {
            let mut block = skin.block(title, focused);
            if rejected {
                block = block.border_style(Style::new().fg(skin.t.err));
            }
            let inner = block.inner(area);
            f.render_widget(block, area);
            Rect { x: inner.x + 1, width: inner.width.saturating_sub(2), height: 1, ..inner }
        } else {
            area
        };
        let prefix_w = unicode_width::UnicodeWidthStr::width(self.prefix) as u16;
        if prefix_w > 0 {
            f.buffer_mut().set_string(inner.x, inner.y, self.prefix, if focused { skin.accent() } else { skin.muted() });
        }
        let text_rect = Rect { x: inner.x + prefix_w, width: inner.width.saturating_sub(prefix_w), ..inner };
        self.rect = text_rect;
        let width = text_rect.width as usize;
        if width == 0 {
            return;
        }

        // Keep the cursor visible.
        self.cursor = self.cursor.min(self.text.len());
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        }
        let w_between = |a: usize, b: usize| self.text[a..b].iter().map(|c| c.width().unwrap_or(0)).sum::<usize>();
        while self.scroll < self.cursor && w_between(self.scroll, self.cursor) + 1 > width {
            self.scroll += 1;
        }

        if self.text.is_empty() {
            let style = skin.muted().add_modifier(Modifier::ITALIC);
            f.buffer_mut().set_stringn(text_rect.x, text_rect.y, &self.placeholder, width, style);
        } else {
            let sel = self.selection();
            let mut spans = Vec::new();
            let mut used = 0;
            for (i, &c) in self.text.iter().enumerate().skip(self.scroll) {
                let cw = c.width().unwrap_or(0);
                if used + cw > width {
                    break;
                }
                used += cw;
                let in_sel = sel.is_some_and(|(a, b)| i >= a && i < b);
                let style = if in_sel { skin.selected() } else { skin.text() };
                spans.push(Span::styled(c.to_string(), style));
            }
            f.buffer_mut().set_line(text_rect.x, text_rect.y, &Line::from(spans), text_rect.width);
        }
        if focused {
            let col = w_between(self.scroll, self.cursor) as u16;
            f.set_cursor_position((text_rect.x + col.min(text_rect.width.saturating_sub(1)), text_rect.y));
        }
    }
}

impl TextInput {
    /// Multiline: fills `area` (the caller draws any frame), with a scrollbar when it overflows.
    fn render_multi(&mut self, f: &mut Frame, area: Rect, skin: &Skin, focused: bool) {
        self.outer = area;
        if area.width < 2 || area.height == 0 {
            self.rect = area;
            return;
        }
        self.cursor = self.cursor.min(self.text.len());
        let h = area.height as usize;
        let mut lines = self.lines(area.width as usize);
        let scroll = lines.len() > h;
        if scroll {
            lines = self.lines(area.width as usize - 1);
        }
        self.rect = Rect { width: area.width - scroll as u16, ..area };
        let row = Self::row_of(&lines, self.cursor);
        if self.follow {
            if row < self.vscroll {
                self.vscroll = row;
            } else if row >= self.vscroll + h {
                self.vscroll = row + 1 - h;
            }
        }
        self.vscroll = self.vscroll.min(lines.len().saturating_sub(h));

        if self.text.is_empty() {
            let style = skin.muted().add_modifier(Modifier::ITALIC);
            for (i, l) in super::wrap(&self.placeholder, self.rect.width as usize).iter().take(h).enumerate() {
                f.buffer_mut().set_stringn(area.x, area.y + i as u16, l, self.rect.width as usize, style);
            }
        }
        let sel = self.selection();
        for (y, &(a, b)) in lines.iter().enumerate().skip(self.vscroll).take(h) {
            let spans: Vec<Span> = (a..b)
                .map(|i| {
                    let in_sel = sel.is_some_and(|(s, e)| i >= s && i < e);
                    Span::styled(self.text[i].to_string(), if in_sel { skin.selected() } else { skin.text() })
                })
                .collect();
            f.buffer_mut().set_line(area.x, area.y + (y - self.vscroll) as u16, &Line::from(spans), self.rect.width);
        }
        if scroll {
            let mut state = ratatui::widgets::ScrollbarState::new(lines.len() - h).position(self.vscroll).viewport_content_length(h);
            let bar = ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("│"))
                .thumb_symbol("┃")
                .track_style(skin.muted())
                .thumb_style(if focused { skin.accent() } else { skin.text() });
            f.render_stateful_widget(bar, area, &mut state);
        }
        if focused && (self.vscroll..self.vscroll + h).contains(&row) {
            let col = self.col_of(&lines, self.cursor) as u16;
            f.set_cursor_position((area.x + col.min(self.rect.width.saturating_sub(1)), area.y + (row - self.vscroll) as u16));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEventState;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent { code, modifiers: mods, kind: ratatui::crossterm::event::KeyEventKind::Press, state: KeyEventState::NONE }
    }

    #[test]
    fn editing_and_filter() {
        let mut clip = Clipboard::new();
        let mut t = TextInput::new("");
        t.set_filter(Some(Box::new(crate::num::accepts_number_input)));
        for c in "12a.5".chars() {
            t.handle_key(key(KeyCode::Char(c), KeyModifiers::NONE), &mut clip);
        }
        assert_eq!(t.text(), "12.5", "letters must be rejected");
        assert_eq!(t.handle_key(key(KeyCode::Char('x'), KeyModifiers::NONE), &mut clip), InputEvent::Rejected);
        t.handle_key(key(KeyCode::Left, KeyModifiers::SHIFT), &mut clip);
        t.handle_key(key(KeyCode::Left, KeyModifiers::SHIFT), &mut clip);
        assert_eq!(t.selected_text().as_deref(), Some(".5"));
        t.handle_key(key(KeyCode::Char('9'), KeyModifiers::NONE), &mut clip);
        assert_eq!(t.text(), "129");
        t.handle_key(key(KeyCode::Home, KeyModifiers::NONE), &mut clip);
        t.handle_key(key(KeyCode::Char('-'), KeyModifiers::NONE), &mut clip);
        assert_eq!(t.text(), "-129");
        assert_eq!(t.paste("abc"), InputEvent::Rejected);
        assert_eq!(t.text(), "-129");
    }

    #[test]
    fn undo_redo() {
        let mut clip = Clipboard::new();
        let mut t = TextInput::new("");
        let k = |c| key(KeyCode::Char(c), KeyModifiers::NONE);
        for c in "hello world".chars() {
            t.handle_key(k(c), &mut clip);
        }
        let ctrl = |c| key(KeyCode::Char(c), KeyModifiers::CONTROL);
        t.handle_key(ctrl('z'), &mut clip);
        assert_eq!(t.text(), "hello ", "a typed word undoes as one step");
        t.handle_key(ctrl('z'), &mut clip);
        assert_eq!(t.text(), "hello");
        t.handle_key(ctrl('y'), &mut clip);
        t.handle_key(ctrl('y'), &mut clip);
        assert_eq!(t.text(), "hello world");
        t.set_text("computed");
        t.handle_key(ctrl('z'), &mut clip);
        assert_eq!(t.text(), "hello world", "outside changes undo too");
        t.handle_key(k('!'), &mut clip);
        assert_eq!(t.handle_key(ctrl('y'), &mut clip), InputEvent::Handled, "new edit clears redo");
        t.set_text("30");
        t.select_all();
        t.handle_key(k('1'), &mut clip);
        t.handle_key(k('5'), &mut clip);
        t.handle_key(ctrl('z'), &mut clip);
        assert_eq!(t.text(), "30", "typing over a selection undoes in one step");
    }

    #[test]
    fn multiline_wrap_and_moves() {
        let mut clip = Clipboard::new();
        let mut t = TextInput::multiline("");
        t.set_text("hello big world\nnext");
        assert_eq!(t.lines(10), [(0, 10), (10, 15), (16, 20)], "wraps after the space, splits at newline");
        assert_eq!(t.lines(3)[0], (0, 3), "long words split");
        t.rect.width = 10;
        t.move_to(2, false);
        t.handle_key(key(KeyCode::Down, KeyModifiers::NONE), &mut clip);
        assert_eq!(t.cursor, 12, "down keeps the column");
        t.handle_key(key(KeyCode::End, KeyModifiers::NONE), &mut clip);
        assert_eq!(t.cursor, 15);
        t.handle_key(key(KeyCode::Enter, KeyModifiers::NONE), &mut clip);
        assert_eq!(t.text(), "hello big world\n\nnext");
        assert_eq!(t.paste("a\r\nb\tc"), InputEvent::Changed);
        assert_eq!(t.text(), "hello big world\na\nb    c\nnext");
    }

    #[test]
    fn word_ops() {
        let mut clip = Clipboard::new();
        let mut t = TextInput::new("");
        t.set_text("hello big world");
        t.handle_key(key(KeyCode::Backspace, KeyModifiers::CONTROL), &mut clip);
        assert_eq!(t.text(), "hello big ");
        t.handle_key(key(KeyCode::Left, KeyModifiers::CONTROL), &mut clip);
        t.handle_key(key(KeyCode::Right, KeyModifiers::CONTROL | KeyModifiers::SHIFT), &mut clip);
        assert_eq!(t.selected_text().as_deref(), Some("big"));
    }
}
