//! Calculators (percentages, Ohm's law, loans…): one to four inputs, one live result.

use crate::app::{Action, App};
use crate::catalog::Kind;
use crate::components::{hit, InputEvent};
use crate::convert;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn draw(f: &mut Frame, area: Rect, app: &mut App) {
    let skin = app.skin;
    let def = app.calc_def();
    let n = def.inputs.len();
    let box_h: u16 = if area.height >= 18 { 5 } else { 3 };
    // As many columns as fit at ~28 cells each; the rest wrap onto more rows.
    let cols = (area.width as usize / 28).clamp(1, n);
    let rows = n.div_ceil(cols) as u16;
    let [inputs, result, formula] =
        Layout::vertical([Constraint::Length(rows * box_h), Constraint::Length(box_h), Constraint::Length(2)]).areas(area);

    let mut boxes = Vec::with_capacity(n);
    for r in 0..rows as usize {
        let in_row = (n - r * cols).min(cols);
        let row = Rect { y: inputs.y + r as u16 * box_h, height: box_h, ..inputs };
        boxes.extend(Layout::horizontal(vec![Constraint::Fill(1); in_row]).split(row).iter().copied());
    }

    for (i, &b) in boxes.iter().enumerate() {
        let focused = app.calc.focus == i;
        let kind = def.inputs[i].kind;
        let input = &mut app.calc.inputs[i];
        let rejected = input.rejected_at.is_some_and(|t| t.elapsed().as_millis() < 350);
        let mut block = skin.block(def.inputs[i].label, focused);
        if rejected {
            block = block.border_style(Style::new().fg(skin.t.err));
        }
        let inner = block.inner(b);
        f.render_widget(block, b);
        let inner = Rect { x: inner.x + 1, width: inner.width.saturating_sub(2), ..inner };
        let row = Rect { y: inner.y + inner.height.saturating_sub(1) / 2, height: 1, ..inner };
        input.render(f, row, &skin, focused, Line::default());
        // Dates get their weekday spelled out underneath, so typos are easy to spot.
        if let (Kind::Date(_), true) = (kind, inner.height >= 3)
            && let Ok(Some(v)) = convert::parse_quantity(kind, &input.text())
        {
            let text = crate::catalog::time::DateFmt::Readable.format(&v).unwrap_or_default();
            f.render_widget(Paragraph::new(Span::styled(text, skin.muted())), Rect { y: inner.y + inner.height - 1, height: 1, ..inner });
        }
    }
    app.calc.boxes = boxes;

    // ── Result ──
    let fresh = skin.animations && app.calc.changed_at.elapsed().as_millis() < 300;
    let mut block = skin.block(def.result, false);
    if fresh {
        block = block.border_style(Style::new().fg(skin.t.accent2));
    }
    let inner = block.inner(result);
    f.render_widget(block, result);
    app.calc.result_box = result;
    let row = Rect { x: inner.x + 1, width: inner.width.saturating_sub(2), y: inner.y + inner.height.saturating_sub(1) / 2, height: 1 };
    let value_style = Style::new().fg(if fresh { skin.t.accent2 } else { skin.t.accent }).add_modifier(Modifier::BOLD);
    let line = match &app.calc.result {
        Ok(None) => Line::styled("—", skin.muted()),
        Ok(Some(q)) => {
            let (s, exact) = app.fmt(def.out, q);
            let s = if def.out.groupable() { app.grouped(&s) } else { s };
            Line::from(vec![
                Span::styled(s, value_style),
                Span::styled(def.suffix, value_style),
                Span::styled(if exact { "   exact" } else { "   ≈ rounded" }, skin.muted()),
            ])
        }
        Err(e) => Line::styled(format!("⚠ {e}"), Style::new().fg(skin.t.err)),
    };
    f.render_widget(Paragraph::new(line), row);

    let text = vec![
        Line::from(vec![Span::styled(" Formula  ", skin.muted()), Span::styled(def.formula, skin.text())]),
        Line::styled(" Number fields accept arithmetic, e.g. 1200*1.2 or (5+3)/2", skin.muted()),
    ];
    f.render_widget(Paragraph::new(text), formula);
}

fn edited(app: &mut App, i: usize, ev: InputEvent) {
    match ev {
        InputEvent::Changed => app.recompute_calc(),
        InputEvent::Rejected => app.reject_hint(app.calc_def().inputs[i].kind),
        _ => {}
    }
}

fn move_focus(app: &mut App, d: i32) {
    let n = app.calc.inputs.len() as i32;
    app.calc.focus = (app.calc.focus as i32 + d).rem_euclid(n) as usize;
    app.calc.inputs[app.calc.focus].select_all();
}

pub fn on_key(app: &mut App, k: KeyEvent) {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    match k.code {
        KeyCode::Esc => return app.back(),
        KeyCode::Tab | KeyCode::Down => return move_focus(app, 1),
        KeyCode::BackTab | KeyCode::Up => return move_focus(app, -1),
        _ => {}
    }
    let i = app.calc.focus;
    match app.calc.inputs[i].handle_key(k, &mut app.clipboard) {
        InputEvent::Ignored => match k.code {
            KeyCode::Enter => app.run_action(Action::Copy),
            KeyCode::Char('c') if ctrl => app.run_action(Action::Copy),
            _ => {}
        },
        ev => edited(app, i, ev),
    }
}

pub fn on_mouse(app: &mut App, m: MouseEvent) {
    for i in 0..app.calc.inputs.len() {
        if app.calc.inputs[i].handle_mouse(m) {
            app.calc.focus = i;
            return;
        }
    }
    if m.kind == MouseEventKind::Down(MouseButton::Left)
        && let Some(i) = app.calc.boxes.iter().position(|&b| hit(b, m.column, m.row))
    {
        app.calc.focus = i;
    }
}
