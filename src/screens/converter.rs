//! Stage 3: the live converter. Both sides are editable; the other one follows.
//! Currency rates are fetched here (and only here).

use crate::app::{Action, App, CFocus};
use crate::catalog::{color, numbers, Kind, Unit};
use crate::components::{fit, hit, spinner, InputEvent, ListAction};
use crate::convert::{self, Output};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

pub fn draw(f: &mut Frame, area: Rect, app: &mut App) {
    let wide = area.width >= 80;
    let tall = area.height >= if wide { 12 } else { 18 };
    let box_h = if tall { 5 } else { 3 };
    let io_h = if wide { box_h } else { box_h * 2 + 1 };
    let show_table = app.settings.show_table && area.height >= io_h + 1 + 5;
    let [io, info, table] = Layout::vertical([Constraint::Length(io_h), Constraint::Length(1), Constraint::Min(0)]).areas(area);

    let (boxes, arrow_area) = if wide {
        let [a, b, c] = Layout::horizontal([Constraint::Fill(1), Constraint::Length(5), Constraint::Fill(1)]).areas(io);
        ([a, c], Rect { y: b.y + (b.height - 1) / 2, height: 1, ..b })
    } else {
        let [a, b, c] = Layout::vertical([Constraint::Length(box_h), Constraint::Length(1), Constraint::Length(box_h)]).areas(io);
        ([a, c], b)
    };
    for side in 0..2 {
        draw_side(f, boxes[side], app, side);
    }
    app.conv.boxes = boxes;

    // ── Swap arrow ──
    app.conv.swap_arrow = arrow_area;
    let skin = app.skin;
    let arrow_style = if !app.target_unit().kind.can_parse() {
        skin.muted()
    } else if hit(arrow_area, app.mouse_pos.0, app.mouse_pos.1) {
        Style::new().fg(skin.t.accent2).add_modifier(Modifier::BOLD)
    } else {
        skin.accent()
    };
    let arrow = if wide { "⇄" } else { "⇅ swap" };
    f.render_widget(Paragraph::new(Span::styled(arrow, arrow_style)).alignment(Alignment::Center), arrow_area);

    // ── Info line: unit rates and the rule behind them ──
    let (from, to) = (app.source_unit(), app.target_unit());
    let one = |a: &Unit, b: &Unit| match convert::convert(a, b, "1", app.rates.as_ref()) {
        Ok(Output::Number(v)) => Some(app.grouped(&app.fmt(b.kind, &v).0)),
        _ => None,
    };
    let mut parts = Vec::new();
    if from.kind.groupable() && to.kind.groupable() && app.settings.show_info {
        if let (Some(a), Some(b)) = (one(from, to), one(to, from)) {
            parts.push(format!("1 {} = {a} {}", from.symbol, to.symbol));
            parts.push(format!("1 {} = {b} {}", to.symbol, from.symbol));
        }
        if let Some(rule) = convert::formula(from, to, app.settings.precision.min(10)) {
            parts.push(format!("{} {rule}", to.symbol));
        }
    }
    f.render_widget(Paragraph::new(format!(" {}", parts.join("   ·   "))).style(skin.muted()), info);

    // ── All-units table ──
    if !show_table {
        app.conv.table.outer = Rect::default();
        return;
    }
    let focus = app.conv.focus;
    let grouping = app.settings.grouping_chars();
    let App { conv, catalog, .. } = app;
    let to_i = conv.to;
    let (units, values) = (&conv.table_units, &conv.table_values);
    let title = format!("{} · all units ({})", catalog.categories[catalog.units[conv.from].category].name, units.len());
    conv.table.render(f, table, &skin, focus == CFocus::Table, Line::from(title), units.len().min(values.len()), "Nothing else in this category", |i, w| {
        let (u, value) = (&catalog.units[units[i]], &values[i]);
        let sym_w = (w / 5).clamp(5, 12);
        let val_w = (w / 2).min(44);
        let name_w = w.saturating_sub(sym_w + val_w + 2);
        let value_style = if u.id == catalog.units[to_i].id { Style::new().fg(skin.t.accent2).add_modifier(Modifier::BOLD) } else { skin.text() };
        let value = match grouping {
            Some((sep, dec)) if u.kind.groupable() => convert::group_digits(value, sep, dec),
            _ => value.clone(),
        };
        let value = if value.width() <= val_w { format!("{value:>val_w$}") } else { fit(&value, val_w) };
        Line::from(vec![
            Span::styled(fit(&u.symbol, sym_w), skin.accent()),
            Span::raw(" "),
            Span::styled(fit(&u.name, name_w), skin.text()),
            Span::raw(" "),
            Span::styled(value, value_style),
        ])
    });
}

/// One value box: title, editable value, and a detail line (grouped value, swatch, error…).
fn draw_side(f: &mut Frame, area: Rect, app: &mut App, side: usize) {
    let skin = app.skin;
    let unit = app.unit(side);
    let online = app.catalog.categories[unit.category].online;
    let focused = app.conv.focus == CFocus::Side(side);
    let passive = side != app.conv.active;
    let input = &app.conv.sides[side];
    let rejected = input.rejected_at.is_some_and(|t| t.elapsed().as_millis() < 350);
    let fresh = passive && skin.animations && app.conv.changed_at.elapsed().as_millis() < 300;

    let title = if unit.name.contains(&unit.symbol) { unit.name.clone() } else { format!("{} ({})", unit.name, unit.symbol) };
    let mut block = skin.block(title, focused);
    if rejected {
        block = block.border_style(Style::new().fg(skin.t.err));
    } else if fresh {
        block = block.border_style(Style::new().fg(skin.t.accent2));
    }
    let inner = block.inner(area);
    f.render_widget(block, area);
    let inner = Rect { x: inner.x + 1, width: inner.width.saturating_sub(2), ..inner };
    let value_row = Rect { y: inner.y + inner.height.saturating_sub(1) / 2, height: 1, ..inner };

    let text = input.text();
    let detail: Option<Line> = if passive && app.conv.error.is_some() {
        Some(Line::styled(format!("⚠ {}", app.conv.error.as_deref().unwrap_or("")), Style::new().fg(skin.t.err)))
    } else if passive && online && app.fetching() && app.rates.is_none() {
        Some(Line::styled(format!("{} Fetching live exchange rates…", spinner(app.rates_started)), skin.accent()))
    } else if text.is_empty() {
        None
    } else {
        detail_line(app, side, &text)
    };

    // Short boxes have no room for a detail line; show errors in place of the value instead.
    if inner.height < 3 && passive && text.is_empty()
        && let Some(d) = &detail {
            f.render_widget(Paragraph::new(d.clone()), value_row);
            app.conv.sides[side].render(f, Rect::default(), &skin, focused, Line::default());
            return;
        }
    app.conv.sides[side].render(f, value_row, &skin, focused, Line::default());
    if let (Some(d), true) = (detail, inner.height >= 3) {
        f.render_widget(Paragraph::new(d), Rect { y: inner.y + inner.height - 1, height: 1, ..inner });
    }
}

fn detail_line(app: &App, side: usize, text: &str) -> Option<Line<'static>> {
    let skin = app.skin;
    let (kind, passive) = (app.unit(side).kind, side != app.conv.active);
    let unit = |s: String| Span::styled(s, skin.muted());
    let category = app.catalog.categories[app.unit(side).category].id;
    Some(match kind {
        // Timestamps and dates: spell the moment out, with how far away it is.
        _ if category == "datetime" => {
            let v = convert::parse_quantity(kind, text).ok()??;
            let v = convert::to_base(app.unit(side), v).ok()?;
            let short = crate::catalog::time::DateFmt::Rfc2822.format(&v).ok()?.replace(" +0000", " UTC");
            let days = ((&v - crate::catalog::time::now()) / crate::num::int(86400)).round().to_integer();
            let when = match days.to_string().as_str() {
                "0" => "today".to_string(),
                "1" => "tomorrow".to_string(),
                "-1" => "yesterday".to_string(),
                d if d.starts_with('-') => format!("{} days ago", &d[1..]),
                d => format!("in {d} days"),
            };
            // Date formats already show the date; add the weekday. Numbers get the full moment.
            let what = match kind {
                Kind::Date(_) => short[..3].to_string(),
                _ => short,
            };
            Line::from(unit(format!("{what} · {when}")))
        }
        Kind::Duration(_) => {
            let v = convert::parse_quantity(kind, text).ok()??;
            Line::from(vec![unit("= ".into()), Span::styled(app.grouped(&app.fmt(Kind::Linear, &v).0), skin.text()), unit(" seconds".into())])
        }
        Kind::Fraction(_) => {
            let v = convert::parse_quantity(kind, text).ok()??;
            Line::from(vec![unit("= ".into()), Span::styled(app.grouped(&app.fmt(Kind::Linear, &v).0), skin.text())])
        }
        Kind::Scale(_) if passive => Line::from(unit(format!("{} {} ≈ floating point", app.conv.shown[side].clone()?, app.unit(side).symbol))),
        k if k.is_numeric() => {
            let shown = app.conv.shown[side].clone()?;
            let mut spans = vec![Span::styled(shown, skin.text().add_modifier(Modifier::BOLD))];
            let note = match (passive, app.conv.exact) {
                (true, true) => " exact",
                (true, false) => " ≈ rounded",
                _ => "",
            };
            spans.push(unit(format!(" {}{note}", app.unit(side).symbol)));
            Line::from(spans)
        }
        Kind::Color(fmt) => {
            let c = fmt.parse(text).ok()??;
            let rgb = color::to8(c);
            let (name, exact) = color::nearest_name(c);
            Line::from(vec![
                Span::styled("      ", Style::new().bg(Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8))),
                unit(format!("  {}{name}", if exact { "" } else { "≈ " })),
            ])
        }
        Kind::Radix(r) if r != 10 => {
            let n = numbers::parse_radix(text, r).ok()??;
            Line::from(vec![unit("= ".into()), Span::styled(app.grouped(&n.to_string()), skin.text()), unit(" decimal".into())])
        }
        Kind::Roman => {
            let n = numbers::parse_roman(text).ok()?;
            Line::from(vec![unit("= ".into()), Span::styled(n.to_string(), skin.text()), unit(" decimal".into())])
        }
        Kind::Chem(crate::catalog::chem::ChemFmt::Formula) => {
            let hill = match crate::catalog::chem::convert(crate::catalog::chem::ChemFmt::Hill, text, 12).ok()?? {
                Output::Text(t) => t,
                _ => return None,
            };
            let atoms = match crate::catalog::chem::convert(crate::catalog::chem::ChemFmt::Atoms, text, 12).ok()?? {
                Output::Number(n) => n.to_string(),
                _ => return None,
            };
            Line::from(unit(format!("{hill} · {atoms} atoms")))
        }
        Kind::Text(_) => Line::from(unit(format!("{} characters", text.chars().count()))),
        Kind::Zone(i) => {
            use crate::catalog::zones;
            let v = convert::parse_quantity(kind, text).ok()??;
            let here = zones::offset_at(i, &v)?;
            let other = app.unit(1 - side);
            let gap = match other.kind {
                Kind::Zone(j) => format!(" · {}", zones::gap(here, zones::offset_at(j, &v)?, &zones::ZONES[j].0.replace(" (this computer)", ""))),
                _ => String::new(),
            };
            Line::from(unit(format!("{}{gap}", zones::offset_text(here))))
        }
        Kind::Note => {
            let hz = convert::parse_quantity(kind, text).ok()??;
            let midi = crate::catalog::music::midi_of(num_traits::ToPrimitive::to_f64(&hz)?);
            Line::from(unit(format!("≈ {} Hz · MIDI {}", app.fmt(Kind::Linear, &hz).0, (midi * 100.0).round() / 100.0)))
        }
        Kind::Interval => {
            let cents = convert::parse_quantity(kind, text).ok()??;
            let ratio = 2f64.powf(num_traits::ToPrimitive::to_f64(&cents)? / 1200.0);
            Line::from(unit(format!("= {} cents · ratio {ratio:.4}", app.fmt(Kind::Linear, &cents).0)))
        }
        _ => return None,
    })
}

fn focus_order(app: &App) -> Vec<CFocus> {
    let mut order = vec![CFocus::Side(0), CFocus::Side(1)];
    if app.conv.table.outer.width > 0 {
        order.push(CFocus::Table);
    }
    order
}

fn move_focus(app: &mut App, d: i32) {
    let order = focus_order(app);
    let i = order.iter().position(|f| *f == app.conv.focus).unwrap_or(0) as i32;
    app.conv.focus = order[(i + d).rem_euclid(order.len() as i32) as usize];
}

fn table_target(app: &mut App, remember: bool) {
    if let Some(&u) = app.conv.table_units.get(app.conv.table.selected) {
        app.set_target(u, remember);
    }
}

pub fn on_key(app: &mut App, k: KeyEvent) {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    match k.code {
        KeyCode::Esc => return app.back(),
        KeyCode::Tab => return move_focus(app, 1),
        KeyCode::BackTab => return move_focus(app, -1),
        _ => {}
    }
    match app.conv.focus {
        CFocus::Side(side) => match app.conv.sides[side].handle_key(k, &mut app.clipboard) {
            InputEvent::Ignored => match k.code {
                KeyCode::Enter => app.run_action(Action::Copy),
                KeyCode::Char('c') if ctrl => app.run_action(Action::Copy),
                KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown
                    if app.conv.table.handle_key(k) == ListAction::Moved => {
                        table_target(app, false);
                    }
                _ => {}
            },
            ev => app.side_event(side, ev),
        },
        CFocus::Table => match k.code {
            KeyCode::Char('c') if ctrl => app.run_action(Action::Copy),
            KeyCode::Char(_) | KeyCode::Backspace if !ctrl => {
                // Typing jumps back to the input.
                app.conv.focus = CFocus::Side(0);
                on_key(app, k);
            }
            _ => match app.conv.table.handle_key(k) {
                ListAction::Moved => table_target(app, false),
                ListAction::Activate => {
                    table_target(app, true);
                    app.conv.focus = CFocus::Side(0);
                }
                ListAction::None => {}
            },
        },
    }
}

pub fn on_mouse(app: &mut App, m: MouseEvent) {
    for side in 0..2 {
        if app.conv.sides[side].handle_mouse(m) {
            app.conv.focus = CFocus::Side(side);
            return;
        }
    }
    if m.kind == MouseEventKind::Down(MouseButton::Left) {
        if hit(app.conv.swap_arrow, m.column, m.row) {
            return app.run_action(Action::Swap);
        }
        if let Some(side) = (0..2).find(|&i| hit(app.conv.boxes[i], m.column, m.row)) {
            app.conv.focus = CFocus::Side(side);
            return;
        }
    }
    match app.conv.table.handle_mouse(m) {
        ListAction::Moved => {
            app.conv.focus = CFocus::Table;
            table_target(app, false);
        }
        ListAction::Activate => {
            table_target(app, true);
            app.conv.focus = CFocus::Side(0);
        }
        ListAction::None => {}
    }
}
