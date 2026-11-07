//! Screens: each draws part of the `App` and maps its own input to app actions.

pub mod calculator;
pub mod converter;
pub mod custom;
pub mod files;
pub mod overlays;
pub mod picker;
pub mod translator;

use crate::app::{Action, App, CFocus, Overlay, PFocus, Screen};
use crate::components::{ease_out, hit, spinner};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

pub const MIN_W: u16 = 40;
pub const MIN_H: u16 = 12;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    if area.width < MIN_W || area.height < MIN_H {
        let msg = format!("Terminal too small\n{}×{} — need at least {MIN_W}×{MIN_H}", area.width, area.height);
        let y = area.height.saturating_sub(2) / 2;
        f.render_widget(
            Paragraph::new(msg).style(app.skin.muted()).alignment(Alignment::Center),
            Rect { y, height: 2.min(area.height), ..area },
        );
        return;
    }
    let [header, body, footer] = Layout::vertical([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)]).areas(area);
    draw_header(f, header, app);

    // Slide the new screen in from the right (a short nudge, not a big gap).
    let shift = ((1.0 - ease_out(app.screen_at, 160, app.skin.animations)) * 4.0).round() as u16;
    let body = Rect { x: body.x + shift, width: body.width - shift, ..body };
    match app.screen {
        Screen::From | Screen::To => picker::draw(f, body, app),
        Screen::Convert => converter::draw(f, body, app),
        Screen::Calc => calculator::draw(f, body, app),
        Screen::Translate => translator::draw(f, body, app),
        Screen::Files => files::draw(f, body, app),
    }
    draw_footer(f, footer, app);

    if app.overlay != Overlay::None {
        // Dim everything behind the popup.
        f.buffer_mut().set_style(area, Style::new().add_modifier(Modifier::DIM));
        match app.overlay {
            Overlay::Help => overlays::draw_help(f, area, app),
            Overlay::Settings => overlays::draw_settings(f, area, app),
            Overlay::Browse => overlays::draw_browser(f, area, app),
            Overlay::Custom => custom::draw(f, area, app),
            Overlay::None => {}
        }
    }
    if let Some(t) = &app.toast {
        t.render(f, area, &app.skin);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &mut App) {
    let skin = app.skin;
    let mut x = area.x;
    let put = |f: &mut Frame, x: &mut u16, s: &str, style: Style| {
        f.buffer_mut().set_stringn(*x, area.y, s, (area.x + area.width).saturating_sub(*x) as usize, style);
        *x += s.width() as u16;
    };
    put(f, &mut x, " ⇄ ", skin.accent());
    put(f, &mut x, "consertion", skin.accent().add_modifier(Modifier::BOLD));
    put(f, &mut x, "  ", skin.muted());

    // Stepper: done steps are "✓ name" (clickable), the current one is a filled chip.
    let unit_name = |i: usize| app.catalog.units[i].name.clone();
    let steps: Vec<(Screen, String)> = match app.screen {
        Screen::Calc => vec![(Screen::From, "Search".into()), (Screen::Calc, unit_name(app.calc.unit))],
        Screen::From => vec![(Screen::From, "From".into()), (Screen::To, "To".into()), (Screen::Convert, "Convert".into())],
        _ => {
            let last = app.final_step();
            vec![
                (Screen::From, app.from.map_or("From".into(), unit_name)),
                (Screen::To, if app.screen == last.0 { unit_name(app.conv.to) } else { "To".into() }),
                (last.0, last.1.into()),
            ]
        }
    };
    let current = steps.iter().position(|s| s.0 == app.screen).unwrap_or(0);
    let buttons_w: u16 = app.header.iter().map(|b| b.width() + 1).sum();
    let room = area.width.saturating_sub(x - area.x + buttons_w + 1) as usize;
    app.crumbs.clear();
    // Each step costs its label plus ~8 columns of decoration; too narrow → current step only.
    let each = (room / steps.len()).saturating_sub(8);
    let all = each >= 4;
    let each = if all { each } else { room.saturating_sub(6) };
    if each >= 3 {
        for (i, (screen, label)) in steps.iter().enumerate() {
            if !all && i != current {
                continue;
            }
            let label = crate::components::fit(label, each).trim_end().to_string();
            let start = x;
            match i.cmp(&current) {
                std::cmp::Ordering::Less => {
                    let hovered = app.mouse_pos.1 == area.y && (start..start + label.width() as u16 + 2).contains(&app.mouse_pos.0);
                    let style = if hovered { skin.accent().add_modifier(Modifier::UNDERLINED) } else { skin.accent() };
                    put(f, &mut x, "✓ ", skin.text().fg(skin.t.ok));
                    put(f, &mut x, &label, style);
                    app.crumbs.push((*screen, Rect { x: start, y: area.y, width: x - start, height: 1 }));
                }
                std::cmp::Ordering::Equal => {
                    // Half-block caps round the chip off without needing a special font.
                    put(f, &mut x, "▐", skin.accent());
                    put(f, &mut x, &format!("{} {label}", i + 1), skin.chip());
                    put(f, &mut x, "▌", skin.accent());
                }
                std::cmp::Ordering::Greater => {
                    put(f, &mut x, &format!("{} ", i + 1), skin.muted().add_modifier(Modifier::BOLD));
                    put(f, &mut x, &label, skin.muted());
                }
            }
            if all && i + 1 < steps.len() {
                put(f, &mut x, " ── ", if i < current { skin.accent() } else { skin.muted() });
            }
        }
    }

    let mut bx = area.x + area.width;
    let (hover, help_open, settings_open) = (app.mouse_pos, app.overlay == Overlay::Help, app.overlay == Overlay::Settings);
    for (b, open) in app.header.iter_mut().zip([help_open, settings_open]).rev() {
        let w = b.width();
        bx = bx.saturating_sub(w + 1);
        if bx < x {
            b.rect = Rect::default();
            continue;
        }
        b.render(f, Rect { x: bx, y: area.y, width: w, height: 1 }, &skin, open, hover);
    }
}

/// (key, description, action when clicked)
type Hint = (&'static str, &'static str, Option<Action>);

fn hints(app: &App) -> Vec<Hint> {
    use Action::*;
    let online = app.screen == Screen::Convert && app.catalog.categories[app.source_unit().category].online;
    match (app.overlay, app.screen) {
        (Overlay::Help, _) => vec![("↑↓", "scroll", None), ("Esc", "close", Some(Help))],
        (Overlay::Browse, _) => vec![("⏎", "open", None), ("⌫", "up", None), ("Tab", "all files", None), ("Esc", "close", None)],
        (Overlay::Settings, _) => vec![("↑↓", "choose", None), ("←→", "change", None), ("Esc", "close", Some(Settings))],
        (Overlay::Custom, _) => vec![("⏎", "next / save", None), ("Tab", "move", None), ("Esc", "cancel", None)],
        (_, Screen::Convert) => {
            let mut h = vec![("⏎", "copy", Some(Copy)), ("^S", "swap", Some(Swap)), ("^F", "favorite", Some(Favorite))];
            if online {
                h.push(("^R", "refresh rates", Some(Refresh)));
            }
            h.push(match app.conv.focus {
                CFocus::Side(_) => ("↑↓", "change target", None),
                CFocus::Table => ("⏎", "set target", None),
            });
            h.extend([("Tab", "focus", None), ("^N", "new", Some(NewConversion)), ("Esc", "back", Some(Back))]);
            h
        }
        (_, Screen::Translate) => vec![
            ("^R", "translate", Some(Refresh)),
            ("^C", "copy", Some(Copy)),
            ("^S", "swap", Some(Swap)),
            ("^F", "favorite", Some(Favorite)),
            ("Tab", "other box", None),
            ("^N", "new", Some(NewConversion)),
            ("Esc", "back", Some(Back)),
        ],
        (_, Screen::Files) => {
            let running = app.files.running.is_some();
            let mut h = vec![("⏎", "convert", None), ("^O", "browse", None), ("Tab", "next", None)];
            if app.files.focus == crate::app::FFocus::Options {
                h.push(("←→", "change", None));
            }
            h.extend([("^S", "swap", Some(Swap)), ("^F", "favorite", Some(Favorite)), ("^N", "new", Some(NewConversion))]);
            h.push(if running { ("Esc", "cancel", None) } else { ("Esc", "back", Some(Back)) });
            h
        }
        (_, Screen::Calc) => vec![
            ("⏎", "copy", Some(Copy)),
            ("Tab", "next field", None),
            ("^F", "favorite", Some(Favorite)),
            ("^N", "new", Some(NewConversion)),
            ("Esc", "back", Some(Back)),
        ],
        _ => {
            let mut h = vec![("⏎", "select", None), ("^F", "favorite", Some(Favorite))];
            let mine = matches!(app.selected_entry(), Some(crate::app::Entry::Unit(u)) if app.catalog.units[u].id.starts_with("custom:"));
            if mine && app.screen == Screen::From {
                h.extend([("^E", "edit", None), ("^D", "delete", None)]);
            }
            h.push(match app.picker.focus {
                PFocus::Cats => ("↑↓", "category", None),
                _ => ("↑↓", "move", None),
            });
            h.extend([("Tab", "focus", None), ("Esc", "back", Some(Back)), ("^Q", "quit", Some(Quit))]);
            h
        }
    }
}

fn draw_footer(f: &mut Frame, area: Rect, app: &mut App) {
    let skin = app.skin;
    // Status on the right: live-rates state when relevant.
    let status: Option<Span> = if app.fetching() {
        Some(Span::styled(format!("{} fetching rates ", spinner(app.rates_started)), skin.accent()))
    } else if app.screen == Screen::Convert && app.catalog.categories[app.source_unit().category].online {
        Some(match (&app.rates, &app.rates_error) {
            (Some(r), None) => Span::styled(format!("● rates {} ", r.date), skin.text().fg(skin.t.ok)),
            (Some(r), Some(_)) => Span::styled(format!("● offline · cached {} ", r.date), skin.text().fg(skin.t.warn)),
            (None, _) => Span::styled("● no rates ".to_string(), skin.text().fg(skin.t.err)),
        })
    } else {
        None
    };
    let status_w = status.as_ref().map_or(0, |s| s.width() as u16);

    app.footer.clear();
    let mut x = area.x + 1;
    let end = (area.x + area.width).saturating_sub(status_w + 2);
    for (k, d, action) in hints(app) {
        let w = (k.width() + d.width() + 1) as u16;
        if x + w > end {
            break;
        }
        let r = Rect { x, y: area.y, width: w, height: 1 };
        let hovered = action.is_some() && hit(r, app.mouse_pos.0, app.mouse_pos.1);
        let desc = if hovered { skin.accent().add_modifier(Modifier::UNDERLINED) } else { skin.muted() };
        f.buffer_mut().set_string(x, area.y, k, skin.key());
        f.buffer_mut().set_string(x + k.width() as u16 + 1, area.y, d, desc);
        if let Some(a) = action {
            app.footer.push((r, a));
        }
        x += w + 3;
    }
    if let Some(s) = status {
        f.render_widget(Paragraph::new(Line::from(s)).alignment(Alignment::Right), area);
    }
}
