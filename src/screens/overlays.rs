//! Popups drawn over any screen: help and settings.

use crate::app::{App, Overlay};
use crate::components::choice::ChoiceHit;
use crate::components::{ease_out, fit, hit, wrap};
use crate::settings::FIELDS;
use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Wrap};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

/// (keys, description); "#" marks a section heading. Descriptions wrap to the popup width.
const HELP: &[(&str, &str)] = &[
    ("#", "Getting around"),
    ("type", "Search units, currencies, dates, colors, formulas, text formats and calculators"),
    ("↑ ↓  PgUp PgDn", "Move in lists"),
    ("Enter", "Choose; in a converter or calculator, copy the result"),
    ("Tab  Shift+Tab", "Move focus between panels and fields"),
    ("Esc", "Back to the search box, clear it, show All, then go back a step"),
    ("Ctrl+F", "Add or remove a favorite (a unit, a pair or a calculator)"),
    ("Ctrl+N", "Start a new conversion"),
    ("F1", "This help"),
    ("F2  Ctrl+O", "Settings: themes, digits, notation, translator, API keys, history…"),
    ("Ctrl+Q", "Quit"),
    ("#", "Converter"),
    ("", "Both boxes are editable: type in either one and the other follows."),
    ("Ctrl+S", "Swap the two units"),
    ("Enter  Ctrl+C", "Copy the computed value (format set in Settings)"),
    ("↑ ↓", "Try other target units live; Tab reaches the all-units table"),
    ("Ctrl+R", "Refresh exchange rates (currencies only)"),
    ("#", "Translator"),
    ("", "Search a language (\"german\", \"українська\") or \"Detect language\", then the target. Type or paste in either box; the other is translated after a short pause."),
    ("Ctrl+R", "Translate now (always, when \"Translate as you type\" is off)"),
    ("Ctrl+S", "Swap languages and texts (a detected language is used)"),
    ("Enter", "New line; Ctrl+C copies the translation"),
    ("", "MyMemory is free with no key (5,000 characters a day, 50,000 with an email in Settings). DeepL and LibreTranslate use your own key, set in Settings → Translation."),
    ("#", "Files"),
    ("", "Search a format (\"png\", \"mp4\", \"docx\") or \"Browse for a file…\", then pick a target. Only conversions something on this computer can do are listed."),
    ("Ctrl+O", "Browse for the input file, or where to save (on the file screen)"),
    ("type / paste / drop", "A path works too: quotes, file:// and ~ are understood"),
    ("Enter", "Convert; if the output exists, Enter again replaces it"),
    ("Esc", "Cancel a running conversion (nothing is left behind)"),
    ("← →", "Change a setting: quality, size, bitrate, resolution, encoding…"),
    ("", "Built in: images (PNG, JPEG, WebP, GIF, BMP, TIFF, ICO, TGA, PPM, QOI), JSON/YAML/TOML/CSV/TSV, text encodings, Markdown → HTML, SRT ↔ WebVTT. With ffmpeg: audio and video. ImageMagick: AVIF, HEIC, JPEG XL, PSD, SVG. LibreOffice or pandoc: documents, spreadsheets, slides. Poppler: PDF → text or image."),
    ("#", "File browser"),
    ("Enter  →", "Open a folder or choose a file"),
    ("⌫  ←", "Parent folder"),
    ("~", "Home folder"),
    ("type", "Filter; a typed path (with /) jumps there"),
    ("Tab", "Show all files, not only ones that convert"),
    ("Ctrl+H", "Show hidden files"),
    ("#", "Calculators"),
    ("", "Percentages, math, electronics, chemistry, money, dates, health, photography, music and audio. Tab or ↑ ↓ moves between fields; the result updates as you type."),
    ("#", "Your own converters"),
    ("", "✎ My Converters → \"New custom converter…\": give a name and what it equals, e.g. step = 0.75 m. It then converts with every length unit. Leave Equals empty to start a new kind (pizza), then add slice = 1/8 pizza."),
    ("Ctrl+E", "Edit one of your units (on the search screen)"),
    ("Ctrl+D", "Delete one of your units (press twice)"),
    ("Tab", "In Equals: complete the suggested unit (↑↓ choose)"),
    ("", "A search that finds nothing offers to create it as your own unit."),
    ("#", "Times, sizes, music"),
    ("15:00  3pm  now", "Time zones: a time alone means today in that city; daylight saving is handled"),
    ("A4  C#5  Bb3", "Note names ↔ Hz and MIDI; \"A4 +20¢\" for cents"),
    ("P5  m3  octave", "Interval names ↔ cents, semitones and frequency ratios"),
    ("A4  34C  L½", "Size charts (paper, bra, ring, clothing): sizes not in the chart snap to the nearest"),
    ("50.45, 30.52", "Coordinates: paste decimal, DMS or DDM with N/S/E/W, plus codes or geohashes"),
    ("#", "Numbers"),
    ("1.5e300  1,5", "Any size; comma or dot as the decimal mark"),
    ("2*(3+4)^2", "Arithmetic works in every number field"),
    ("1:30  5:30:15", "Minutes and seconds: 1:30 = 1.5 (hours, pace, degrees…)"),
    ("", "Math is exact (fractions, no floating point) and only rounded for display; \"≈\" marks a rounded value. Logarithmic scales (dB, pH, AWG) and roots use floating point to 15 digits."),
    ("#", "Text fields"),
    ("← →  Home End", "Move the cursor"),
    ("Ctrl+← →", "Jump by word"),
    ("Shift+arrows", "Select (add Ctrl for words)"),
    ("Ctrl+A", "Select all"),
    ("Ctrl+C X V", "Copy, cut, paste"),
    ("Ctrl+Z", "Undo"),
    ("Ctrl+Y  Ctrl+Shift+Z", "Redo"),
    ("Ctrl+W  Ctrl+⌫", "Delete a word"),
    ("Ctrl+U  Ctrl+K", "Delete to the start / end"),
    ("#", "Mouse"),
    ("click", "Focus or choose; click a selected row again to open it"),
    ("right-click", "Copy a field, a result or a table value"),
    ("click ★ column", "Toggle a favorite"),
    ("drag", "Select text in an input"),
    ("double-click", "Select all text in an input"),
    ("wheel", "Scroll lists"),
    ("header / footer", "Click finished steps and key hints"),
    ("#", "Command line"),
    ("consertion 5 mi km", "Convert once and print the result, e.g. consertion 100 usd eur"),
    ("consertion file a.png b.jpg", "Convert a file (never replaces an existing one)"),
    ("consertion translate auto de \"hi\"", "Translate once"),
    ("#", "Online"),
    ("", "Only currencies and the translator use the internet, and only when you convert or translate. Rates are cached on disk and reused offline. Text to translate goes to the translator shown under the boxes; files never leave your computer."),
];

/// Centered popup rect that grows in when opened.
pub fn popup(app: &App, screen: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(screen.width.saturating_sub(4));
    let h = h.min(screen.height.saturating_sub(4));
    let h = ((h as f32) * (0.6 + 0.4 * ease_out(app.overlay_at, 160, app.skin.animations))) as u16;
    Rect { x: screen.x + (screen.width - w) / 2, y: screen.y + (screen.height - h) / 2, width: w, height: h.max(3) }
}

pub fn draw_help(f: &mut Frame, screen: Rect, app: &mut App) {
    let skin = app.skin;
    let w = 86.min(screen.width.saturating_sub(4));
    // Border, selection marker, scrollbar and a margin take 5 columns.
    let content = w.saturating_sub(5) as usize;
    let kw = if content >= 56 { 22 } else { 0 };
    let dw = if kw > 0 { content - kw } else { content.saturating_sub(2) };
    let mut rows: Vec<Line<'static>> = Vec::new();
    for &(k, d) in HELP {
        if k == "#" {
            if !rows.is_empty() {
                rows.push(Line::default());
            }
            rows.push(Line::styled(d.to_string(), skin.accent().add_modifier(ratatui::style::Modifier::BOLD)));
            continue;
        }
        // Narrow popups put the keys on their own line, with the description indented below.
        // Keys too long for their column get a line of their own, too.
        let own_line = !k.is_empty() && (kw == 0 || k.width() >= kw);
        if own_line {
            rows.push(Line::styled(k.to_string(), skin.key()));
        }
        for (i, part) in wrap(d, dw).into_iter().enumerate() {
            let key = if i == 0 && !own_line { fit(k, kw) } else { " ".repeat(if kw > 0 { kw } else { 2 }) };
            rows.push(Line::from(vec![Span::styled(key, skin.key()), Span::styled(part, skin.text())]));
        }
    }
    let area = popup(app, screen, w, rows.len() as u16 + 2);
    app.popup = area;
    app.help.plain = true;
    f.render_widget(Clear, area);
    app.help.render(f, area, &skin, true, Line::from("Help · keyboard & mouse"), rows.len(), "", |i, _| rows[i].clone());
}

/// Settings rows: a group heading (`None`) or a field index.
fn settings_rows() -> Vec<Option<usize>> {
    let mut rows = Vec::new();
    for (i, f) in FIELDS.iter().enumerate() {
        if i == 0 || FIELDS[i - 1].group != f.group {
            rows.push(None);
        }
        rows.push(Some(i));
    }
    rows
}

pub fn draw_settings(f: &mut Frame, screen: Rect, app: &mut App) {
    let skin = app.skin;
    let rows = settings_rows();
    // Rows, a gap and 3 lines of help, borders.
    let area = popup(app, screen, 68, rows.len() as u16 + 6);
    app.popup = area;
    f.render_widget(Clear, area);
    let block = skin.block("Settings", true);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let list_h = inner.height.saturating_sub(4) as usize;
    // Keep the selected field (and its heading) in view.
    let sel_row = rows.iter().position(|r| *r == Some(app.settings_sel)).unwrap_or(0);
    let top = sel_row.saturating_sub(1).min(app.settings_scroll);
    app.settings_scroll = if sel_row >= top + list_h { sel_row + 1 - list_h.max(1) } else { top }.min(rows.len().saturating_sub(list_h));
    app.settings_rows.iter_mut().for_each(|c| *c = Default::default());
    for (y, row) in rows.iter().enumerate().skip(app.settings_scroll).take(list_h) {
        let r = Rect { x: inner.x + 1, y: inner.y + (y - app.settings_scroll) as u16, width: inner.width.saturating_sub(2), height: 1 };
        let Some(i) = *row else {
            let name = FIELDS[rows.get(y + 1).copied().flatten().unwrap_or(0)].group;
            f.buffer_mut().set_stringn(r.x + 1, r.y, name, r.width as usize, skin.accent().add_modifier(ratatui::style::Modifier::BOLD));
            continue;
        };
        let field = &FIELDS[i];
        let selected = i == app.settings_sel;
        match (&mut app.settings_edit, field.edit) {
            (Some(input), Some(_)) if selected => {
                app.settings_rows[i].render_text(f, r, &skin, field.label, "", true);
                let x = r.x + (r.width / 2).min(28);
                input.render(f, Rect { x, width: (r.x + r.width).saturating_sub(x + 1), ..r }, &skin, true, Line::default());
            }
            (_, Some(_)) => app.settings_rows[i].render_text(f, r, &skin, field.label, &(field.value)(&app.settings), selected),
            _ => app.settings_rows[i].render(f, r, &skin, field.label, &(field.value)(&app.settings), selected),
        }
    }
    if rows.len() > list_h {
        let mut state = ratatui::widgets::ScrollbarState::new(rows.len() - list_h).position(app.settings_scroll).viewport_content_length(list_h);
        let bar = ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("│"))
            .thumb_symbol("┃")
            .track_style(skin.muted())
            .thumb_style(skin.accent());
        f.render_stateful_widget(bar, Rect { height: list_h as u16, ..inner }, &mut state);
    }
    let help = Rect { x: inner.x + 2, y: inner.y + list_h as u16 + 1, width: inner.width.saturating_sub(4), height: inner.height.saturating_sub(list_h as u16 + 1) };
    let field = &FIELDS[app.settings_sel];
    let tail = match (field.edit, app.settings_edit.is_some()) {
        (Some(_), true) => "Enter saves · Esc cancels",
        (Some(_), false) => "Enter to edit. Changes save immediately.",
        _ => "Changes apply and save immediately.",
    };
    let text = vec![Line::styled(format!("{} {tail}", field.help), skin.muted())];
    f.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), help);
}

pub fn draw_browser(f: &mut Frame, screen: Rect, app: &mut App) {
    let h = screen.height.saturating_sub(4).min(34);
    let area = popup(app, screen, 90, h);
    app.popup = area;
    let skin = app.skin;
    if let Some(b) = &mut app.browser {
        b.render(f, area, &skin);
    }
}

pub fn help_key(app: &mut App, k: KeyEvent) {
    match k.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => app.toggle_overlay(Overlay::Help),
        _ => {
            app.help.handle_key(k);
        }
    }
}

pub fn settings_key(app: &mut App, k: KeyEvent) {
    if let Some(input) = &mut app.settings_edit {
        match k.code {
            KeyCode::Enter => app.finish_setting_edit(true),
            KeyCode::Esc => app.finish_setting_edit(false),
            _ => {
                input.handle_key(k, &mut app.clipboard);
            }
        }
        return;
    }
    let n = FIELDS.len();
    let editable = FIELDS[app.settings_sel].edit.is_some();
    match k.code {
        KeyCode::Esc | KeyCode::Char('q') => app.toggle_overlay(Overlay::Settings),
        KeyCode::Up | KeyCode::BackTab => app.settings_sel = (app.settings_sel + n - 1) % n,
        KeyCode::Down | KeyCode::Tab => app.settings_sel = (app.settings_sel + 1) % n,
        KeyCode::PageUp | KeyCode::Home => app.settings_sel = 0,
        KeyCode::PageDown | KeyCode::End => app.settings_sel = n - 1,
        KeyCode::Enter | KeyCode::Right if editable => app.start_setting_edit(),
        KeyCode::Left => app.step_setting(app.settings_sel, -1),
        KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => app.step_setting(app.settings_sel, 1),
        _ => {}
    }
}

pub fn on_mouse(app: &mut App, m: MouseEvent) {
    // Clicking outside a popup closes it.
    if m.kind == MouseEventKind::Down(MouseButton::Left) && !hit(app.popup, m.column, m.row) {
        let o = app.overlay;
        return app.toggle_overlay(o);
    }
    match app.overlay {
        Overlay::Help => {
            app.help.handle_mouse(m);
        }
        Overlay::Settings => {
            match m.kind {
                MouseEventKind::ScrollDown => app.settings_sel = (app.settings_sel + 1).min(FIELDS.len() - 1),
                MouseEventKind::ScrollUp => app.settings_sel = app.settings_sel.saturating_sub(1),
                _ => {}
            }
            if let Some(input) = &mut app.settings_edit
                && input.handle_mouse(m)
            {
                return;
            }
            let hits: Vec<_> = app.settings_rows.iter().map(|c| c.handle_mouse(&m)).collect();
            for (i, h) in hits.into_iter().enumerate() {
                match h {
                    Some(ChoiceHit::Row) if FIELDS[i].edit.is_some() => {
                        app.finish_setting_edit(true);
                        app.settings_sel = i;
                        app.start_setting_edit();
                    }
                    Some(ChoiceHit::Row) => app.settings_sel = i,
                    Some(ChoiceHit::Step(d)) => {
                        app.settings_sel = i;
                        app.step_setting(i, d);
                    }
                    None => continue,
                }
                break;
            }
        }
        Overlay::Browse => app.browser_mouse(m),
        Overlay::Custom => crate::screens::custom::on_mouse(app, m),
        Overlay::None => {}
    }
}
