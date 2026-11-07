//! A labelled "◀ value ▶" selector row (or a "value ✎" text row), used by settings and file options.

use super::hit;
use crate::theme::Skin;
use ratatui::crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

#[derive(Default, Clone, Copy)]
pub struct Choice {
    row: Rect,
    left: Rect,
    right: Rect,
}

pub enum ChoiceHit {
    Row,
    Step(i32),
}

impl Choice {
    pub fn render(&mut self, f: &mut Frame, area: Rect, skin: &Skin, label: &str, value: &str, selected: bool) {
        self.draw(f, area, skin, label, value, selected, true);
    }

    /// A free-text value: no arrows, the whole row is one click target.
    pub fn render_text(&mut self, f: &mut Frame, area: Rect, skin: &Skin, label: &str, value: &str, selected: bool) {
        self.draw(f, area, skin, label, value, selected, false);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw(&mut self, f: &mut Frame, area: Rect, skin: &Skin, label: &str, value: &str, selected: bool, arrows: bool) {
        self.row = area;
        let vw = value.width() as u16 + if arrows { 6 } else { 2 }; // "◀  value  ▶" or "value ✎"
        let vx = area.x + area.width.saturating_sub(vw + 1);
        self.left = Rect { x: vx, y: area.y, width: 3, height: 1 };
        self.right = Rect { x: vx + vw - 3, y: area.y, width: 3, height: 1 };
        let base = if selected { skin.selected().add_modifier(Modifier::BOLD) } else { skin.text() };
        if selected {
            f.buffer_mut().set_style(area, base);
            f.buffer_mut().set_string(area.x, area.y, "▌", skin.accent().bg(skin.t.sel_bg));
        }
        f.buffer_mut().set_stringn(area.x + 2, area.y, label, vx.saturating_sub(area.x + 3) as usize, base);
        let arrow = if selected { base } else { skin.accent() };
        let value_style = if selected { base } else { skin.accent().add_modifier(Modifier::BOLD) };
        let line = if arrows {
            Line::from(vec![Span::styled("◀  ", arrow), Span::styled(value.to_string(), value_style), Span::styled("  ▶", arrow)])
        } else {
            self.left = Rect::default();
            self.right = Rect::default();
            Line::from(vec![Span::styled(value.to_string(), value_style), Span::styled(" ✎", arrow)])
        };
        f.buffer_mut().set_line(vx, area.y, &line, vw);
    }

    pub fn handle_mouse(&self, m: &MouseEvent) -> Option<ChoiceHit> {
        if !matches!(m.kind, MouseEventKind::Down(MouseButton::Left)) {
            return None;
        }
        if hit(self.left, m.column, m.row) {
            Some(ChoiceHit::Step(-1))
        } else if hit(self.right, m.column, m.row) {
            Some(ChoiceHit::Step(1))
        } else if hit(self.row, m.column, m.row) {
            Some(ChoiceHit::Row)
        } else {
            None
        }
    }
}
