//! Clickable button. Bordered when it has 3 rows, compact `[ label ]` otherwise.

use super::hit;
use crate::theme::Skin;
use ratatui::crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::time::Instant;
use unicode_width::UnicodeWidthStr;

pub struct Button {
    pub label: String,
    /// Shortcut shown dimmed after the label, e.g. "^S".
    pub hint: &'static str,
    pub rect: Rect,
    pressed_at: Option<Instant>,
}

impl Button {
    pub fn new(label: &str, hint: &'static str) -> Self {
        Self { label: label.into(), hint, rect: Rect::default(), pressed_at: None }
    }

    /// Width needed to show label + hint inside a border.
    pub fn width(&self) -> u16 {
        let hint = if self.hint.is_empty() { 0 } else { self.hint.width() + 1 };
        (self.label.width() + hint + 4) as u16
    }

    /// Visual press feedback; call when the button's action runs (by click or key).
    pub fn press(&mut self) {
        self.pressed_at = Some(Instant::now());
    }

    pub fn clicked(&self, m: &MouseEvent) -> bool {
        matches!(m.kind, MouseEventKind::Down(MouseButton::Left)) && hit(self.rect, m.column, m.row)
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, skin: &Skin, focused: bool, hover: (u16, u16)) {
        self.rect = area;
        if area.width == 0 || area.height == 0 {
            return;
        }
        let hovered = hit(area, hover.0, hover.1);
        let pressed = skin.animations && self.pressed_at.is_some_and(|t| t.elapsed().as_millis() < 160);
        let t = skin.t;
        let (fg, border) = match (focused, hovered) {
            (true, _) => (t.accent, t.accent),
            (false, true) => (t.accent2, t.accent2),
            _ => (t.text, t.border),
        };
        let mut style = Style::new().fg(fg);
        if focused || hovered {
            style = style.add_modifier(Modifier::BOLD);
        }
        if pressed {
            style = Style::new().fg(t.sel_fg).bg(t.accent).add_modifier(Modifier::BOLD);
        }
        let mut spans = vec![Span::styled(self.label.clone(), style)];
        if !self.hint.is_empty() && area.width >= self.width() {
            spans.push(Span::styled(format!(" {}", self.hint), if pressed { style } else { skin.muted() }));
        }
        if area.height >= 3 {
            let block = ratatui::widgets::Block::bordered()
                .border_type(skin.border)
                .border_style(Style::new().fg(border));
            let inner = block.inner(area);
            f.render_widget(block, area);
            if pressed {
                f.buffer_mut().set_style(inner, style);
            }
            let y = inner.y + inner.height.saturating_sub(1) / 2;
            f.render_widget(Paragraph::new(Line::from(spans)).alignment(Alignment::Center), Rect { y, height: 1, ..inner });
        } else {
            let mut line = vec![Span::styled("[ ", Style::new().fg(border))];
            line.extend(spans);
            line.push(Span::styled(" ]", Style::new().fg(border)));
            f.render_widget(Paragraph::new(Line::from(line)).alignment(Alignment::Center), area);
        }
    }
}
