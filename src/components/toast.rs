//! Short notification that slides in at the bottom-right and disappears by itself.

use crate::theme::Skin;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::Frame;
use std::time::Instant;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, PartialEq)]
pub enum Level {
    Info,
    Ok,
    Warn,
    Error,
}

pub struct Toast {
    pub msg: String,
    pub level: Level,
    born: Instant,
}

const LIFE_MS: u128 = 2600;

impl Toast {
    pub fn new(msg: impl Into<String>, level: Level) -> Self {
        Self { msg: msg.into(), level, born: Instant::now() }
    }

    pub fn alive(&self) -> bool {
        self.born.elapsed().as_millis() < LIFE_MS + if self.level == Level::Error { 2000 } else { 0 }
    }

    pub fn render(&self, f: &mut Frame, screen: Rect, skin: &Skin) {
        let (icon, color): (&str, Color) = match self.level {
            Level::Info => ("ℹ", skin.t.accent),
            Level::Ok => ("✓", skin.t.ok),
            Level::Warn => ("!", skin.t.warn),
            Level::Error => ("✗", skin.t.err),
        };
        let max = screen.width.saturating_sub(4).min(60);
        let w = ((self.msg.width() + 6) as u16).min(max);
        if w < 8 || screen.height < 5 {
            return;
        }
        let slide = super::ease_out(self.born, 180, skin.animations);
        let x = screen.x + screen.width - 1 - (w as f32 * slide) as u16;
        let area = Rect { x, y: screen.y + screen.height - 4, width: (screen.x + screen.width - 1 - x).min(w), height: 3 };
        f.render_widget(Clear, area);
        let block = Block::bordered().border_type(skin.border).border_style(Style::new().fg(color));
        let line = Line::from(vec![Span::styled(format!("{icon} "), Style::new().fg(color)), Span::styled(self.msg.clone(), skin.text())]);
        f.render_widget(Paragraph::new(line).block(block), area);
    }
}
