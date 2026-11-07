//! Stage 3 for languages: two multi-line boxes. Type in either; the other gets the translation
//! after a short pause (or on Ctrl+R). Text is sent to the translator chosen in Settings.

use crate::app::{Action, App};
use crate::components::{hit, spinner, InputEvent};
use crate::translate;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn draw(f: &mut Frame, area: Rect, app: &mut App) {
    let skin = app.skin;
    let wide = area.width >= 80;
    let [io, status] = Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).areas(area);
    let (boxes, arrow) = if wide {
        let [a, b, c] = Layout::horizontal([Constraint::Fill(1), Constraint::Length(5), Constraint::Fill(1)]).areas(io);
        ([a, c], Rect { y: b.y + b.height.saturating_sub(1) / 2, height: 1, ..b })
    } else {
        let [a, b, c] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1), Constraint::Fill(1)]).areas(io);
        ([a, c], b)
    };
    app.tr.boxes = boxes;
    app.tr.swap_arrow = arrow;

    for side in 0..2 {
        let name = app.unit(side).name.clone();
        let title = match (&app.tr.detected, app.lang_code(side)) {
            (Some(d), "auto") => format!("{name} · {}", crate::catalog::lang::name_of(d).unwrap_or(d)),
            _ => name,
        };
        let focused = app.tr.focus == side;
        let input = &app.tr.sides[side];
        let rejected = input.rejected_at.is_some_and(|t| t.elapsed().as_millis() < 350);
        let fresh = side != app.tr.active && skin.animations && app.tr.changed_at.elapsed().as_millis() < 300;
        let mut block = skin.block(title, focused);
        if rejected {
            block = block.border_style(Style::new().fg(skin.t.err));
        } else if fresh {
            block = block.border_style(Style::new().fg(skin.t.accent2));
        }
        let count = input.text().chars().count();
        if count > 0 {
            block = block.title_bottom(Line::styled(format!(" {count} "), skin.muted()).alignment(Alignment::Right));
        }
        let inner = block.inner(boxes[side]);
        f.render_widget(block, boxes[side]);
        let inner = Rect { x: inner.x + 1, width: inner.width.saturating_sub(1), ..inner };
        app.tr.sides[side].render(f, inner, &skin, focused, Line::default());
    }

    let arrow_style = if hit(arrow, app.mouse_pos.0, app.mouse_pos.1) { Style::new().fg(skin.t.accent2).add_modifier(Modifier::BOLD) } else { skin.accent() };
    f.render_widget(Paragraph::new(Span::styled(if wide { "⇄" } else { "⇅ swap" }, arrow_style)).alignment(Alignment::Center), arrow);

    // Status: what's happening, then where the text goes.
    let state: Span = if app.translating() {
        Span::styled(format!("{} Translating…", spinner(app.tr.started)), skin.accent())
    } else if let Some(e) = &app.tr.error {
        Span::styled(format!("⚠ {e}"), Style::new().fg(skin.t.err))
    } else if !app.settings.translate_live {
        Span::styled("Press Ctrl+R to translate", skin.muted())
    } else {
        Span::styled("Translates as you type", skin.muted())
    };
    let p = app.settings.provider;
    let privacy = format!("Sent to {} ({}) only when translating · F2 to change", translate::provider_name(p), translate::host(p, &app.settings));
    let lines = vec![Line::from(vec![Span::raw(" "), state]), Line::styled(format!(" {privacy}"), skin.muted())];
    f.render_widget(Paragraph::new(lines), status);
}

pub fn on_key(app: &mut App, k: KeyEvent) {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    match k.code {
        KeyCode::Esc => return app.back(),
        KeyCode::Tab | KeyCode::BackTab => {
            app.tr.focus = 1 - app.tr.focus;
            return;
        }
        _ => {}
    }
    let side = app.tr.focus;
    match app.tr.sides[side].handle_key(k, &mut app.clipboard) {
        InputEvent::Changed => app.translator_typed(side),
        InputEvent::Rejected => app.reject_hint(app.unit(side).kind),
        InputEvent::Ignored if ctrl && k.code == KeyCode::Char('c') => app.run_action(Action::Copy),
        _ => {}
    }
}

pub fn on_mouse(app: &mut App, m: MouseEvent) {
    for side in 0..2 {
        if app.tr.sides[side].handle_mouse(m) {
            app.tr.focus = side;
            return;
        }
    }
    if m.kind == MouseEventKind::Down(MouseButton::Left) {
        if hit(app.tr.swap_arrow, m.column, m.row) {
            return app.run_action(Action::Swap);
        }
        if let Some(side) = (0..2).find(|&i| hit(app.tr.boxes[i], m.column, m.row)) {
            app.tr.focus = side;
        }
    }
}
