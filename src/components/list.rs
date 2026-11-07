//! Scrollable, selectable list with a scrollbar. Rows are produced lazily, only for what's visible.

use super::hit;
use crate::theme::Skin;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Modifier;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

#[derive(Debug, PartialEq)]
pub enum ListAction {
    None,
    Moved,
    /// Enter, or click on the already-selected row.
    Activate,
}

#[derive(Default)]
pub struct ListView {
    pub selected: usize,
    offset: usize,
    len: usize,
    /// Keyboard moves keep the selection in view; wheel scrolling doesn't.
    follow: bool,
    pub outer: Rect,
    rows: Rect,
    /// Rows that can't be selected (section headers). Index-aligned; may be empty.
    pub inert: Vec<bool>,
    /// Read-only text (help): no selection; keys scroll.
    pub plain: bool,
}

impl ListView {
    pub fn reset(&mut self) {
        self.offset = 0;
        self.select(0);
    }

    fn is_inert(&self, i: usize) -> bool {
        self.inert.get(i).copied().unwrap_or(false)
    }

    /// Select row `i`, or the nearest selectable row after (else before) it.
    pub fn select(&mut self, i: usize) {
        self.follow = true;
        let n = self.inert.len().max(self.len).max(i + 1);
        self.selected = (i..n).chain((0..i).rev()).find(|&j| !self.is_inert(j)).unwrap_or(i);
    }

    /// Row index under a screen position, if any.
    pub fn row_at(&self, x: u16, y: u16) -> Option<usize> {
        let i = self.offset + y.checked_sub(self.rows.y)? as usize;
        (hit(self.rows, x, y) && i < self.len).then_some(i)
    }

    /// Left edge of the row content (after the selection marker).
    pub fn content_x(&self) -> u16 {
        self.rows.x + 1
    }

    fn page(&self) -> usize {
        (self.rows.height as usize).max(1)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ListAction {
        if self.len == 0 {
            return ListAction::None;
        }
        if self.plain {
            let max = self.len.saturating_sub(self.page());
            self.follow = false;
            self.offset = match key.code {
                KeyCode::Up => self.offset.saturating_sub(1),
                KeyCode::Down => (self.offset + 1).min(max),
                KeyCode::PageUp => self.offset.saturating_sub(self.page()),
                KeyCode::PageDown => (self.offset + self.page()).min(max),
                KeyCode::Home => 0,
                KeyCode::End => max,
                _ => return ListAction::None,
            };
            return ListAction::Moved;
        }
        let last = self.len - 1;
        let up = matches!(key.code, KeyCode::Up | KeyCode::PageUp | KeyCode::End);
        let mut to = match key.code {
            KeyCode::Up => self.selected.saturating_sub(1),
            KeyCode::Down => (self.selected + 1).min(last),
            KeyCode::PageUp => self.selected.saturating_sub(self.page()),
            KeyCode::PageDown => (self.selected + self.page()).min(last),
            KeyCode::Home => 0,
            KeyCode::End => last,
            KeyCode::Enter => return ListAction::Activate,
            _ => return ListAction::None,
        };
        // Skip section headers in the direction of travel.
        while self.is_inert(to) && ((up && to > 0) || (!up && to < last)) {
            to = if up { to - 1 } else { to + 1 };
        }
        if self.is_inert(to) {
            to = self.selected;
        }
        let moved = to != self.selected;
        self.select(to);
        if moved { ListAction::Moved } else { ListAction::None }
    }

    pub fn handle_mouse(&mut self, m: MouseEvent) -> ListAction {
        if !hit(self.outer, m.column, m.row) {
            return ListAction::None;
        }
        match m.kind {
            MouseEventKind::ScrollDown => {
                self.offset = (self.offset + 3).min(self.len.saturating_sub(self.page()));
                self.follow = false;
                ListAction::None
            }
            MouseEventKind::ScrollUp => {
                self.offset = self.offset.saturating_sub(3);
                self.follow = false;
                ListAction::None
            }
            MouseEventKind::Down(MouseButton::Left) if hit(self.rows, m.column, m.row) => {
                // Clicking the scrollbar column jumps proportionally.
                if m.column == self.rows.x + self.rows.width - 1 && self.len > self.page() {
                    let frac = (m.row - self.rows.y) as f32 / (self.rows.height.max(2) - 1) as f32;
                    self.offset = ((self.len - self.page()) as f32 * frac).round() as usize;
                    self.follow = false;
                    return ListAction::None;
                }
                let i = self.offset + (m.row - self.rows.y) as usize;
                if i >= self.len || self.is_inert(i) || self.plain {
                    return ListAction::None;
                }
                let again = i == self.selected;
                self.selected = i;
                if again { ListAction::Activate } else { ListAction::Moved }
            }
            _ => ListAction::None,
        }
    }

    /// `row(index, width)` builds one visible row. The selected row gets a "▌" marker and highlight.
    pub fn render(
        &mut self,
        f: &mut Frame,
        area: Rect,
        skin: &Skin,
        focused: bool,
        title: Line,
        len: usize,
        empty_msg: &str,
        row: impl Fn(usize, usize) -> Line<'static>,
    ) {
        self.outer = area;
        self.len = len;
        let block = skin.block(title, focused);
        let inner = block.inner(area);
        f.render_widget(block, area);
        self.rows = inner;
        if len == 0 {
            let y = inner.y + inner.height / 2;
            f.render_widget(
                Paragraph::new(empty_msg).style(skin.muted()).alignment(Alignment::Center),
                Rect { y, height: 1.min(inner.height), ..inner },
            );
            return;
        }
        if self.selected >= len || self.is_inert(self.selected) {
            let i = self.selected.min(len - 1);
            self.select(i);
        }
        let h = inner.height as usize;
        if self.follow {
            if self.selected < self.offset {
                self.offset = self.selected;
            } else if self.selected >= self.offset + h {
                self.offset = self.selected + 1 - h;
            }
            // Keep a section header visible above its first row.
            if self.offset == self.selected && self.selected > 0 && self.is_inert(self.selected - 1) {
                self.offset -= 1;
            }
        }
        self.offset = self.offset.min(len.saturating_sub(h));
        let scroll = len > h;
        let width = inner.width.saturating_sub(if scroll { 2 } else { 1 }) as usize;
        for (y, i) in (self.offset..len.min(self.offset + h)).enumerate() {
            let r = Rect { x: inner.x, y: inner.y + y as u16, width: inner.width - scroll as u16, height: 1 };
            let selected = i == self.selected && !self.plain;
            if selected {
                let mut style = skin.selected();
                if focused {
                    style = style.add_modifier(Modifier::BOLD);
                }
                f.buffer_mut().set_style(r, style);
                f.buffer_mut().set_string(r.x, r.y, "▌", if focused { skin.accent().bg(skin.t.sel_bg) } else { style });
            }
            let mut line = row(i, width);
            if selected {
                // Row colors would vanish against the highlight; use the selection colors.
                for s in &mut line.spans {
                    s.style = s.style.fg(skin.t.sel_fg).bg(skin.t.sel_bg);
                }
            }
            f.buffer_mut().set_line(r.x + 1, r.y, &line, width as u16);
        }
        if scroll {
            let mut state = ScrollbarState::new(len - h).position(self.offset).viewport_content_length(h);
            let bar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("│"))
                .thumb_symbol("┃")
                .track_style(skin.muted())
                .thumb_style(if focused { skin.accent() } else { skin.text() });
            f.render_stateful_widget(bar, inner, &mut state);
        }
    }
}
