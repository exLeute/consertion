//! Stage 3 for files: input and output paths (typed, pasted, dropped or browsed), the settings
//! the chosen converter supports, and a Convert button with progress. Nothing is overwritten
//! without a second Enter.

use crate::app::{Action, App, FFocus};
use crate::components::choice::ChoiceHit;
use crate::components::{hit, spinner, InputEvent};
use crate::files::{self, FORMATS};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// The line under a path: what it is, or what's wrong with it. (text, is_problem)
fn input_info(app: &App) -> (String, bool) {
    let text = app.files.input.text();
    if text.trim().is_empty() {
        return ("Enter or Ctrl+O to browse — or type, paste or drop a file here".into(), false);
    }
    let p = files::parse_path(&text);
    let Ok(meta) = std::fs::metadata(&p) else { return ("File not found".into(), true) };
    if meta.is_dir() {
        return ("That's a folder — Enter to browse it".into(), true);
    }
    let (from, to) = (app.file_fmt(0), app.file_fmt(1));
    let size = files::human_size(meta.len());
    match files::of_path(&p) {
        Some(i) if !files::convertible(i, to) => (format!("{} can't be converted to {} — Esc to pick another target", FORMATS[i].name, FORMATS[to].name), true),
        Some(i) => (format!("{} · {size}", FORMATS[i].name), false),
        None => (format!("Unknown extension — read as {} · {size}", FORMATS[from].name), false),
    }
}

fn output_info(app: &App) -> (String, bool) {
    let text = app.files.output.text();
    let to = &FORMATS[app.file_fmt(1)];
    if text.trim().is_empty() {
        return ("Name of the file to create (Ctrl+O to choose a folder)".into(), false);
    }
    let p = app.output_path();
    if p.parent().is_some_and(|d| !d.as_os_str().is_empty() && !d.is_dir()) {
        return ("This folder doesn't exist".into(), true);
    }
    if p.is_dir() {
        return ("That's a folder — add a file name".into(), true);
    }
    let ext_note = match files::of_path(&p) {
        Some(i) if FORMATS[i].ext != to.ext => format!(" · saved as {} despite the .{} name", to.name, p.extension().map_or(String::new(), |e| e.to_string_lossy().into_owned())),
        _ => String::new(),
    };
    if p.exists() {
        return (format!("Already exists — you'll be asked before it's replaced{ext_note}"), true);
    }
    (format!("Will be created · {}{ext_note}", to.name), false)
}

pub fn draw(f: &mut Frame, area: Rect, app: &mut App) {
    let skin = app.skin;
    let tall = area.height >= 22;
    let path_h = if tall { 4 } else { 3 };
    let rows = app.option_rows();
    let opts_h = (rows.max(1) as u16 + 2).min(area.height.saturating_sub(path_h * 2 + 4).max(3));
    let [input, output, opts, action] =
        Layout::vertical([Constraint::Length(path_h), Constraint::Length(path_h), Constraint::Length(opts_h), Constraint::Length(3)]).areas(area);

    // ── Paths ──
    let (to, from) = (&FORMATS[app.file_fmt(1)], &FORMATS[app.file_fmt(0)]);
    let infos = [input_info(app), output_info(app)];
    let titles = [format!("Input · {}", from.name), format!("Output · {}", to.name)];
    for (i, r) in [input, output].into_iter().enumerate() {
        let focused = app.files.focus == [FFocus::Input, FFocus::Output][i];
        let block = skin.block(titles[i].clone(), focused).title_bottom(Line::styled(" ^O browse ", skin.muted()).right_aligned());
        let inner = block.inner(r);
        f.render_widget(block, r);
        let inner = Rect { x: inner.x + 1, width: inner.width.saturating_sub(2), ..inner };
        let input = if i == 0 { &mut app.files.input } else { &mut app.files.output };
        input.render(f, Rect { height: 1, ..inner }, &skin, focused, Line::default());
        if inner.height >= 2 {
            let (text, bad) = &infos[i];
            let style = if *bad { Style::new().fg(if i == 1 && text.starts_with("Already") { skin.t.warn } else { skin.t.err }) } else { skin.muted() };
            f.render_widget(Paragraph::new(Span::styled(text.clone(), style)), Rect { y: inner.y + 1, height: 1, ..inner });
        }
    }
    app.files.boxes = [input, output];

    // ── Settings for this conversion ──
    let be = app.files.backends.get(app.files.backend).copied();
    let title = match be {
        Some(b) => format!("Settings · via {}", b.name()),
        None => "Settings".into(),
    };
    let focused = app.files.focus == FFocus::Options;
    let block = skin.block(title, focused);
    let inner = block.inner(opts);
    f.render_widget(block, opts);
    app.files.opts_box = opts;
    app.files.opt_rows.resize(rows, Default::default());
    if rows == 0 {
        let msg = if be.is_none() { "Nothing on this computer can do this conversion" } else { "Nothing to set for this conversion" };
        f.render_widget(Paragraph::new(Span::styled(format!(" {msg}"), skin.muted())), Rect { height: 1, ..inner });
    }
    // Scroll when there are more settings than rows.
    let h = inner.height as usize;
    let top = app.files.opt_sel.saturating_sub(h.saturating_sub(1)).min(rows.saturating_sub(h));
    for row in 0..rows {
        if row < top || row >= top + h {
            app.files.opt_rows[row] = Default::default();
            continue;
        }
        let r = Rect { x: inner.x + 1, y: inner.y + (row - top) as u16, width: inner.width.saturating_sub(2), height: 1 };
        let (label, value) = app.option_row(row);
        let selected = focused && app.files.opt_sel == row;
        app.files.opt_rows[row].render(f, r, &skin, label, &value, selected);
    }

    // ── Convert button and status ──
    let bw = app.files.button.width().max(14);
    let [button, _, status] = Layout::horizontal([Constraint::Length(bw), Constraint::Length(2), Constraint::Min(0)]).areas(action);
    let running = app.files.running.is_some();
    app.files.button.label = if running { "Cancel".into() } else { "Convert".into() };
    app.files.button.hint = if running { "Esc" } else { "⏎" };
    let hover = app.mouse_pos;
    app.files.button.render(f, button, &skin, false, hover);
    let line: Line = if let Some(r) = &app.files.running {
        let secs = r.started.elapsed().as_secs();
        match r.progress {
            Some(p) => {
                let w = (status.width as usize).saturating_sub(20).clamp(4, 40);
                let done = ((p * w as f32).round() as usize).min(w);
                Line::from(vec![
                    Span::styled(format!("{} {:>3.0}% ", spinner(r.started), p * 100.0), skin.accent()),
                    Span::styled("━".repeat(done), skin.accent()),
                    Span::styled("─".repeat(w - done), skin.muted()),
                    Span::styled(format!(" {secs} s"), skin.muted()),
                ])
            }
            None => Line::styled(format!("{} Converting… {secs} s", spinner(r.started)), skin.accent()),
        }
    } else if app.files.confirm {
        Line::styled("⚠ The output exists — Enter again to replace it, or change the name", Style::new().fg(skin.t.warn).add_modifier(Modifier::BOLD))
    } else {
        match &app.files.result {
            Some(Ok(d)) => Line::from(vec![
                Span::styled("✓ Saved ", Style::new().fg(skin.t.ok).add_modifier(Modifier::BOLD)),
                Span::styled(d.output.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned()), skin.text().add_modifier(Modifier::BOLD)),
                Span::styled(format!(" · {} · {:.1} s", files::human_size(d.bytes), d.secs), skin.muted()),
                Span::styled(format!(" · in {}", d.output.parent().map_or(String::new(), files::display_path)), skin.muted()),
            ]),
            Some(Err(e)) if e == "Cancelled" => Line::styled("Cancelled — nothing was written", skin.muted()),
            Some(Err(e)) => Line::styled(format!("⚠ {e}"), Style::new().fg(skin.t.err)),
            None => Line::styled(format!("Converts locally{}", be.map_or(String::new(), |b| format!(" with {}", b.name()))), skin.muted()),
        }
    };
    let status_row = Rect { y: status.y + status.height.saturating_sub(1) / 2, height: 1, ..status };
    f.render_widget(Paragraph::new(line), status_row);
    app.files.status_box = status_row;
}

fn cycle_focus(app: &mut App, d: i32) {
    let mut order = vec![FFocus::Input, FFocus::Output];
    if app.option_rows() > 0 {
        order.push(FFocus::Options);
    }
    let i = order.iter().position(|f| *f == app.files.focus).unwrap_or(0) as i32;
    app.files.focus = order[(i + d).rem_euclid(order.len() as i32) as usize];
}

pub fn on_key(app: &mut App, k: KeyEvent) {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    match k.code {
        KeyCode::Esc if app.files.running.is_some() => return app.cancel_file_convert(),
        KeyCode::Esc => return app.back(),
        KeyCode::Tab => return cycle_focus(app, 1),
        KeyCode::BackTab => return cycle_focus(app, -1),
        KeyCode::Char('o') if ctrl => return app.browse_files(),
        _ => {}
    }
    match app.files.focus {
        FFocus::Input | FFocus::Output => {
            let out = app.files.focus == FFocus::Output;
            let input = if out { &mut app.files.output } else { &mut app.files.input };
            match input.handle_key(k, &mut app.clipboard) {
                InputEvent::Changed if out => app.file_output_changed(),
                InputEvent::Changed => app.file_input_changed(),
                InputEvent::Ignored => match k.code {
                    KeyCode::Enter if !out && !files::parse_path(&app.files.input.text()).is_file() => app.browse_files(),
                    KeyCode::Enter if !out => app.files.focus = FFocus::Output,
                    KeyCode::Enter => app.start_file_convert(),
                    KeyCode::Down => cycle_focus(app, 1),
                    KeyCode::Up if out => app.files.focus = FFocus::Input,
                    KeyCode::Char('c') if ctrl => app.run_action(Action::Copy),
                    _ => {}
                },
                _ => {}
            }
        }
        FFocus::Options => {
            let n = app.option_rows();
            match k.code {
                KeyCode::Up if app.files.opt_sel == 0 => app.files.focus = FFocus::Output,
                KeyCode::Up => app.files.opt_sel -= 1,
                KeyCode::Down => app.files.opt_sel = (app.files.opt_sel + 1).min(n.saturating_sub(1)),
                KeyCode::Left => app.step_file_option(app.files.opt_sel, -1),
                KeyCode::Right | KeyCode::Char(' ') => app.step_file_option(app.files.opt_sel, 1),
                KeyCode::Enter => app.start_file_convert(),
                KeyCode::Char('c') if ctrl => app.run_action(Action::Copy),
                _ => {}
            }
        }
    }
}

pub fn on_mouse(app: &mut App, m: MouseEvent) {
    if app.files.input.handle_mouse(m) {
        app.files.focus = FFocus::Input;
        return;
    }
    if app.files.output.handle_mouse(m) {
        app.files.focus = FFocus::Output;
        return;
    }
    if app.files.button.clicked(&m) {
        app.files.button.press();
        return if app.files.running.is_some() { app.cancel_file_convert() } else { app.start_file_convert() };
    }
    let hits: Vec<_> = app.files.opt_rows.iter().map(|c| c.handle_mouse(&m)).collect();
    for (i, h) in hits.into_iter().enumerate() {
        match h {
            Some(ChoiceHit::Row) => {}
            Some(ChoiceHit::Step(d)) => app.step_file_option(i, d),
            None => continue,
        }
        app.files.focus = FFocus::Options;
        app.files.opt_sel = i;
        return;
    }
    if m.kind == MouseEventKind::Down(MouseButton::Left) {
        if hit(app.files.boxes[0], m.column, m.row) {
            app.files.focus = FFocus::Input;
        } else if hit(app.files.boxes[1], m.column, m.row) {
            app.files.focus = FFocus::Output;
        }
    }
    let n = app.option_rows();
    match m.kind {
        MouseEventKind::ScrollDown if hit(app.files.opts_box, m.column, m.row) => app.files.opt_sel = (app.files.opt_sel + 1).min(n.saturating_sub(1)),
        MouseEventKind::ScrollUp if hit(app.files.opts_box, m.column, m.row) => app.files.opt_sel = app.files.opt_sel.saturating_sub(1),
        _ => {}
    }
}
