//! Stage 1 and 2: search + categories + results. Works fully offline.

use crate::app::{Action, App, CatEntry, Entry, PFocus, Row, Screen};
use crate::catalog::{Catalog, Kind};
use crate::components::{fit, InputEvent, ListAction};
use crate::theme::Skin;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

/// Width of the "★ " favorite gutter at the start of each row.
const GUTTER: u16 = 2;

fn cat_name(c: &Catalog, e: CatEntry) -> String {
    match e {
        CatEntry::All => "All".into(),
        CatEntry::Favorites => "★ Favorites".into(),
        CatEntry::Recent => "↺ Recent".into(),
        CatEntry::Cat(i) => c.categories[i].name.into(),
        CatEntry::Group(g) => g.into(),
    }
}

/// A quiet "Name ─────" section line.
fn header(name: &str, w: usize, skin: &Skin) -> Line<'static> {
    let rule = "─".repeat(w.saturating_sub(name.width() + 1));
    Line::from(vec![Span::styled(format!("{name} "), skin.muted()), Span::styled(rule, Style::new().fg(skin.t.border))])
}

pub fn draw(f: &mut Frame, area: Rect, app: &mut App) {
    let skin = app.skin;
    let [search_area, rest] = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);
    let favs: Vec<bool> = app.picker.rows.iter().map(|r| matches!(r, Row::Item(e) if app.is_favorite(app.fav_entry(*e)))).collect();
    // Your units show what they are ("step = 0.75 m").
    let defs: Vec<(String, String)> = app.settings.custom.iter().filter(|u| !u.equals.is_empty()).map(|u| (u.id(), u.equals.clone())).collect();
    let offer = app.picker.offer_new.then(|| app.picker.search.text().trim().to_string());
    let App { picker: p, catalog: c, screen, from, mouse_pos, .. } = app;

    let title = match (*screen, *from) {
        (Screen::To, Some(u)) => {
            let u = &c.units[u];
            let online = if c.categories[u.category].online { " · live rates" } else { "" };
            format!("Convert {} ({}) to…{online}", u.name, u.symbol)
        }
        _ => format!("Search {} units, currencies, formats and calculators", c.units.len()),
    };
    p.search.render(f, search_area, &skin, p.focus == PFocus::Search, Line::from(title));

    let has_cats = *screen == Screen::From;
    let narrow = rest.width < 72;
    p.sidebar_visible = has_cats && !narrow;
    let list_area = if !has_cats {
        rest
    } else if narrow {
        let [chip, list] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(rest);
        draw_chip(f, chip, &skin, p, c, *mouse_pos);
        list
    } else {
        let side_w = (rest.width * 2 / 7).clamp(26, 38);
        let [side, list] = Layout::horizontal([Constraint::Length(side_w), Constraint::Min(0)]).areas(rest);
        let (entries, counts) = (&p.cat_entries, &p.counts);
        p.cats.render(f, side, &skin, p.focus == PFocus::Cats, Line::from("Categories"), entries.len(), "", |i, w| {
            if let CatEntry::Group(g) = entries[i] {
                return header(g, w, &skin);
            }
            let n = counts[i];
            let count = if n == 0 { "·".to_string() } else { n.to_string() };
            let style = if n == 0 { skin.muted() } else { skin.text() };
            // Categories sit indented under their group header; All/Favorites/Recent stay flush.
            let indent = if matches!(entries[i], CatEntry::Cat(k) if !c.categories[k].group.is_empty()) { "  " } else { "" };
            Line::from(vec![
                Span::raw(indent),
                Span::styled(fit(&cat_name(c, entries[i]), w.saturating_sub(count.len() + 1 + indent.len())), style),
                Span::styled(format!(" {count}"), skin.muted()),
            ])
        });
        list
    };

    let query = p.search.text().to_lowercase();
    let rows = &p.rows;
    let items = rows.iter().filter(|r| matches!(r, Row::Item(_))).count();
    let list_title = match *screen {
        Screen::To => format!("{} · {items}", from.map_or("", |u| c.categories[c.units[u].category].name)),
        _ => format!("{} · {items}", cat_name(c, p.cat_entries[p.cat])),
    };
    let from_file = match (*screen, from.map(|u| c.units[u].kind)) {
        (Screen::To, Some(Kind::File(i))) if query.is_empty() => Some(i),
        _ => None,
    };
    let empty = match (p.cat_entries.get(p.cat), from_file) {
        (_, Some(i)) => format!("Nothing on this computer converts {} — install {}", crate::files::FORMATS[i].name, crate::files::suggest(i)),
        (Some(CatEntry::Favorites), _) if query.is_empty() => "No favorites yet — press Ctrl+F on any row".into(),
        _ => "No matches — try another word".into(),
    };
    p.list.render(f, list_area, &skin, p.focus == PFocus::List, Line::from(list_title), rows.len(), &empty, |i, w| match rows[i] {
        Row::Header(cat) => header(c.categories[cat].name, w, &skin),
        Row::Item(e) => {
            let w = w.saturating_sub(GUTTER as usize);
            let sym_w = (w / 4).clamp(6, 14);
            let name_w = w.saturating_sub(sym_w + 1);
            let (sym, name) = match e {
                Entry::Unit(u) | Entry::Pair(u, _) if matches!(c.units[u].kind, Kind::Calc(_)) => {
                    (c.units[u].symbol.clone(), c.units[u].name.clone())
                }
                Entry::Unit(u) if c.units[u].kind == Kind::NewCustom && offer.is_some() => {
                    (c.units[u].symbol.clone(), format!("Create “{}” as your own unit…", offer.as_deref().unwrap_or("")))
                }
                Entry::Unit(u) => {
                    let unit = &c.units[u];
                    match defs.iter().find(|d| d.0 == unit.id) {
                        Some((_, eq)) => (unit.symbol.clone(), format!("{} = {eq}", unit.name)),
                        None => (unit.symbol.clone(), unit.name.clone()),
                    }
                }
                Entry::Pair(a, b) => {
                    let (a, b) = (&c.units[a], &c.units[b]);
                    (format!("{}→{}", a.symbol, b.symbol), format!("{} → {}", a.name, b.name))
                }
            };
            let star = if favs[i] { Span::styled("★ ", Style::new().fg(skin.t.warn)) } else { Span::raw("  ") };
            let mut spans = vec![star, Span::styled(fit(&sym, sym_w), skin.accent()), Span::raw(" ")];
            spans.extend(highlight(&fit(&name, name_w), &query, &skin));
            Line::from(spans)
        }
    });
}

/// Underline the first place the search query appears in `s`.
fn highlight(s: &str, query: &str, skin: &Skin) -> Vec<Span<'static>> {
    let lower = s.to_lowercase();
    let term = query.split_whitespace().find(|t| lower.contains(t));
    // Only when lowercasing kept byte positions (true for ASCII and most text).
    match term.filter(|_| lower.len() == s.len()).and_then(|t| lower.find(t).map(|i| (i, i + t.len()))) {
        Some((a, b)) if s.is_char_boundary(a) && s.is_char_boundary(b) => vec![
            Span::styled(s[..a].to_string(), skin.text()),
            Span::styled(s[a..b].to_string(), Style::new().fg(skin.t.accent2).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            Span::styled(s[b..].to_string(), skin.text()),
        ],
        _ => vec![Span::styled(s.to_string(), skin.text())],
    }
}

fn draw_chip(f: &mut Frame, area: Rect, skin: &Skin, p: &mut crate::app::Picker, c: &Catalog, hover: (u16, u16)) {
    let focused = p.focus == PFocus::Cats;
    let label = format!("{} · {}", cat_name(c, p.cat_entries[p.cat]), p.counts[p.cat]);
    let style = if focused { skin.accent().add_modifier(Modifier::BOLD) } else { skin.text() };
    let bw = p.chip[0].width();
    p.chip[0].render(f, Rect { width: bw, ..area }, skin, false, hover);
    p.chip[1].render(f, Rect { x: area.x + area.width - bw, width: bw, ..area }, skin, false, hover);
    let mid = Rect { x: area.x + bw, width: area.width.saturating_sub(bw * 2), ..area };
    f.render_widget(Paragraph::new(Span::styled(label, style)).alignment(Alignment::Center), mid);
}

fn cycle_category(app: &mut App, d: i32) {
    let n = app.picker.cat_entries.len() as i32;
    // Step over section headers.
    let next = (1..n).map(|k| (app.picker.cat as i32 + d * k).rem_euclid(n) as usize).find(|&i| !matches!(app.picker.cat_entries[i], CatEntry::Group(_)));
    if let Some(i) = next {
        app.set_category(i);
    }
}

pub fn on_key(app: &mut App, k: KeyEvent) {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let has_cats = app.screen == Screen::From;
    match k.code {
        // Esc steps out gradually: back to the search box, clear it, show All, then go back.
        KeyCode::Esc => {
            if app.picker.focus != PFocus::Search {
                app.picker.focus = PFocus::Search;
            } else if !app.picker.search.is_empty() {
                app.picker.search.set_text("");
                app.refresh_picker();
            } else if has_cats && app.picker.cat != 0 {
                app.set_category(0);
            } else {
                app.back();
            }
            return;
        }
        KeyCode::Tab | KeyCode::BackTab => {
            let order: &[PFocus] = if has_cats { &[PFocus::Search, PFocus::Cats, PFocus::List] } else { &[PFocus::Search, PFocus::List] };
            let i = order.iter().position(|f| *f == app.picker.focus).unwrap_or(0) as i32;
            let d = if k.code == KeyCode::Tab { 1 } else { -1 };
            app.picker.focus = order[(i + d).rem_euclid(order.len() as i32) as usize];
            return;
        }
        KeyCode::Char('c') if ctrl && app.picker.search.selected_text().is_none() => return app.run_action(Action::Quit),
        KeyCode::Char('e') if ctrl && app.screen == Screen::From => {
            if let Some(Entry::Unit(u)) = app.selected_entry()
                && !crate::screens::custom::edit_unit(app, u)
            {
                app.notify("Only your own units can be edited — pick one in ✎ My Converters", crate::components::toast::Level::Info);
            }
            return;
        }
        KeyCode::Char('d') if ctrl && app.screen == Screen::From => {
            if let Some(Entry::Unit(u)) = app.selected_entry() {
                crate::screens::custom::delete_unit(app, u);
            }
            return;
        }
        _ => {}
    }

    // Typing anywhere goes to the search box.
    let typing = matches!(k.code, KeyCode::Char(_) | KeyCode::Backspace) && !ctrl;
    if typing && app.picker.focus != PFocus::Search {
        app.picker.focus = PFocus::Search;
    }

    match app.picker.focus {
        PFocus::Search => match app.picker.search.handle_key(k, &mut app.clipboard) {
            InputEvent::Changed => app.refresh_picker(),
            InputEvent::Ignored
                if matches!(k.code, KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown | KeyCode::Enter)
                    && app.picker.list.handle_key(k) == ListAction::Activate =>
            {
                app.activate_entry()
            }
            _ => {}
        },
        PFocus::Cats => match k.code {
            KeyCode::Up | KeyCode::Left => cycle_category(app, -1),
            KeyCode::Down | KeyCode::Right if !app.picker.sidebar_visible || k.code == KeyCode::Down => cycle_category(app, 1),
            KeyCode::Enter | KeyCode::Right => app.picker.focus = PFocus::List,
            KeyCode::Home => app.set_category(0),
            KeyCode::End => app.set_category(app.picker.cat_entries.len() - 1),
            _ => {}
        },
        PFocus::List => match k.code {
            KeyCode::Left if has_cats => app.picker.focus = PFocus::Cats,
            _ => {
                if app.picker.list.handle_key(k) == ListAction::Activate {
                    app.activate_entry();
                }
            }
        },
    }
}

pub fn on_mouse(app: &mut App, m: MouseEvent) {
    if app.picker.search.handle_mouse(m) {
        app.picker.focus = PFocus::Search;
        return;
    }
    if app.screen == Screen::From && !app.picker.sidebar_visible {
        for (i, d) in [(0, -1), (1, 1)] {
            if app.picker.chip[i].clicked(&m) {
                app.picker.chip[i].press();
                app.animate(200);
                return cycle_category(app, d);
            }
        }
    }
    if app.picker.sidebar_visible && app.picker.cats.handle_mouse(m) != ListAction::None {
        app.picker.focus = PFocus::Cats;
        let i = app.picker.cats.selected;
        return app.set_category(i);
    }
    // Clicking the star gutter toggles a favorite without opening the row.
    if m.kind == MouseEventKind::Down(MouseButton::Left) {
        let x = app.picker.list.content_x();
        if (x..x + GUTTER).contains(&m.column)
            && let Some(Row::Item(e)) = app.picker.list.row_at(m.column, m.row).map(|i| app.picker.rows[i]) {
                let e = app.fav_entry(e);
                return app.toggle_favorite(e);
            }
    }
    match app.picker.list.handle_mouse(m) {
        ListAction::Moved => app.picker.focus = PFocus::List,
        ListAction::Activate => app.activate_entry(),
        ListAction::None => {}
    }
}
