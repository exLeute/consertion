//! The "custom converter" form: name, symbol and what it equals. Opened from
//! "New custom converter…" or with Ctrl+E on one of your units.

use crate::app::{App, Overlay};
use crate::catalog::custom::{self, CustomUnit};
use crate::catalog::Catalog;
use crate::components::toast::Level;
use crate::components::TextInput;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Wrap};
use ratatui::Frame;

const LABELS: [&str; 3] = ["Name", "Symbol", "Equals"];

pub struct CustomForm {
    pub fields: [TextInput; 3],
    pub focus: usize,
    /// Index in `settings.custom` when editing.
    pub editing: Option<usize>,
    pub error: Option<String>,
    /// Ctrl+D was pressed once; a second press deletes.
    pub confirm_delete: bool,
    /// Highlighted unit suggestion for "Equals".
    pub pick: usize,
}

impl CustomForm {
    fn unit(&self) -> CustomUnit {
        CustomUnit { name: self.fields[0].text().trim().into(), symbol: self.fields[1].text().trim().into(), equals: self.fields[2].text().trim().into() }
    }
}

/// `name`: prefilled name (from a search that found nothing).
pub fn open(app: &mut App, editing: Option<usize>, name: Option<String>) {
    let mut fields = [TextInput::new("e.g. step"), TextInput::new("optional, e.g. stp"), TextInput::new("e.g. 0.75 m — or empty for a new kind")];
    if let Some(u) = editing.and_then(|i| app.settings.custom.get(i)) {
        for (f, v) in fields.iter_mut().zip([&u.name, &u.symbol, &u.equals]) {
            f.set_text(v);
        }
    }
    let mut focus = 0;
    if let Some(n) = name.filter(|n| !n.trim().is_empty()) {
        fields[0].set_text(n.trim());
        focus = 2;
    }
    app.custom_form = Some(CustomForm { fields, focus, editing, error: None, confirm_delete: false, pick: 0 });
    app.toggle_overlay(Overlay::Custom);
}

/// Ctrl+D on a picker row: delete one of your units, after a second press.
pub fn delete_unit(app: &mut App, u: usize) {
    let id = app.catalog.units[u].id.clone();
    let Some(i) = app.settings.custom.iter().position(|c| c.id() == id) else {
        return app.notify("Only your own units can be deleted", Level::Info);
    };
    let armed = app.delete_armed.take().is_some_and(|(a, t)| a == id && t.elapsed().as_secs() < 4);
    if !armed {
        app.delete_armed = Some((id, std::time::Instant::now()));
        return app.notify(format!("Press Ctrl+D again to delete {}", app.settings.custom[i].name), Level::Warn);
    }
    match custom::remove(&app.settings.custom, i) {
        Ok(list) => {
            let name = app.settings.custom[i].name.clone();
            commit(app, list, None);
            app.notify(format!("Deleted {name}"), Level::Info);
        }
        Err(e) => app.notify(e, Level::Warn),
    }
}

/// Ctrl+E on a picker row: edit it if it's one of yours.
pub fn edit_unit(app: &mut App, u: usize) -> bool {
    let id = &app.catalog.units[u].id;
    match app.settings.custom.iter().position(|c| &c.id() == id) {
        Some(i) => {
            open(app, Some(i), None);
            true
        }
        None => false,
    }
}

fn close(app: &mut App) {
    app.custom_form = None;
    if app.overlay == Overlay::Custom {
        app.toggle_overlay(Overlay::Custom);
    }
}

/// Use a new list of units: save it, rebuild the catalog, follow renamed ids.
fn commit(app: &mut App, list: Vec<CustomUnit>, renamed: Option<(String, String)>) {
    if let Some((old, new)) = renamed.filter(|(o, n)| o != n) {
        let swap = |id: &mut String| {
            if *id == old {
                *id = new.clone();
            }
        };
        for (a, b) in &mut app.settings.recent {
            swap(a);
            swap(b);
        }
        for f in &mut app.settings.favorites {
            let parts: Vec<String> = f.split('>').map(|p| if p == old { new.clone() } else { p.to_string() }).collect();
            *f = parts.join(">");
        }
    }
    app.settings.custom = list;
    app.settings.save();
    app.catalog = Catalog::with_custom(&app.settings.custom);
    app.refresh_picker();
}

fn save(app: &mut App) {
    let Some(form) = &mut app.custom_form else { return };
    let unit = form.unit();
    match custom::apply(&app.settings.custom, form.editing, &unit) {
        Err(e) => form.error = Some(e),
        Ok(list) => {
            let old_id = form.editing.and_then(|i| app.settings.custom.get(i)).map(CustomUnit::id);
            commit(app, list, old_id.map(|o| (o, unit.id())));
            close(app);
            let what = if unit.equals.is_empty() { "a new kind of unit".to_string() } else { format!("1 {} = {}", unit.symbol(), unit.equals) };
            app.notify(format!("Saved {} — {what}", unit.name), Level::Info);
        }
    }
}

fn delete(app: &mut App) {
    let Some(form) = &mut app.custom_form else { return };
    let Some(i) = form.editing else { return };
    if !form.confirm_delete {
        form.confirm_delete = true;
        form.error = Some("Press Ctrl+D again to delete it".into());
        return;
    }
    match custom::remove(&app.settings.custom, i) {
        Err(e) => {
            form.error = Some(e);
            form.confirm_delete = false;
        }
        Ok(list) => {
            let name = app.settings.custom[i].name.clone();
            commit(app, list, None);
            close(app);
            app.notify(format!("Deleted {name}"), Level::Info);
        }
    }
}

fn suggestions(app: &App) -> Vec<usize> {
    match &app.custom_form {
        Some(f) if f.focus == 2 => custom::suggest(&app.catalog, &f.fields[2].text(), &f.unit().id()),
        _ => Vec::new(),
    }
}

pub fn on_key(app: &mut App, k: KeyEvent) {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let sugg = suggestions(app);
    let Some(form) = &mut app.custom_form else { return close(app) };
    if !(ctrl && k.code == KeyCode::Char('d')) {
        form.confirm_delete = false;
    }
    let pick = form.pick.min(sugg.len().saturating_sub(1));
    match k.code {
        KeyCode::Esc => close(app),
        KeyCode::Char('d') if ctrl => delete(app),
        // In "Equals", ↑↓ choose a suggested unit and Tab takes it.
        KeyCode::Down if !sugg.is_empty() => form.pick = (pick + 1) % sugg.len(),
        KeyCode::Up if !sugg.is_empty() => form.pick = (pick + sugg.len() - 1) % sugg.len(),
        KeyCode::Tab if !sugg.is_empty() => {
            let done = custom::complete(&form.fields[2].text(), &app.catalog.units[sugg[pick]].symbol);
            form.fields[2].set_text(&done);
            form.pick = 0;
        }
        KeyCode::Enter if form.focus < 2 => form.focus += 1,
        KeyCode::Enter => save(app),
        KeyCode::Tab | KeyCode::Down => form.focus = (form.focus + 1) % 3,
        KeyCode::BackTab | KeyCode::Up => form.focus = (form.focus + 2) % 3,
        _ => {
            form.fields[form.focus].handle_key(k, &mut app.clipboard);
            form.error = None;
            form.pick = 0;
        }
    }
}

pub fn paste(app: &mut App, s: &str) {
    if let Some(form) = &mut app.custom_form {
        form.fields[form.focus].paste(s.lines().next().unwrap_or("").trim());
        form.error = None;
    }
}

pub fn on_mouse(app: &mut App, m: MouseEvent) {
    let Some(form) = &mut app.custom_form else { return };
    for i in 0..3 {
        if form.fields[i].handle_mouse(m) {
            form.focus = i;
            return;
        }
    }
}

/// What the form will do, or what's wrong with it.
fn preview(app: &App, form: &CustomForm) -> (String, bool) {
    if let Some(e) = &form.error {
        return (e.clone(), true);
    }
    let u = form.unit();
    let name = if u.name.is_empty() { "it" } else { u.name.as_str() };
    if u.equals.is_empty() {
        return (format!("Starts a new kind of unit. Then add others from it, e.g. slice = 1/8 {name}."), false);
    }
    match custom::resolve(&app.catalog, &u.equals, &u.id()) {
        Ok((amount, r)) => {
            let r = &app.catalog.units[r];
            let ten = app.fmt(crate::catalog::Kind::Linear, &(amount * crate::num::int(10))).0;
            let kind = app.catalog.categories[r.category].name.trim_start_matches("✎ ");
            (format!("✓ 1 {s} = {} · 10 {s} = {ten} {} · works with every {kind} unit", u.equals, r.symbol, s = u.symbol()), false)
        }
        Err(e) => (e, true),
    }
}

pub fn draw(f: &mut Frame, screen: Rect, app: &mut App) {
    let skin = app.skin;
    let Some(form) = &app.custom_form else { return };
    let (msg, bad) = preview(app, form);
    let title = match form.editing.and_then(|i| app.settings.custom.get(i)) {
        Some(u) => format!("Edit {}", u.name),
        None => "New custom converter".into(),
    };
    let sugg = suggestions(app);
    let w = 76.min(screen.width.saturating_sub(4));
    let area = crate::screens::overlays::popup(app, screen, w, 14);
    app.popup = area;
    f.render_widget(Clear, area);
    let block = skin.block(title, true);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 5 || inner.width < 20 {
        return;
    }
    let Some(form) = &mut app.custom_form else { return };
    let label_w = 9;
    for (i, label) in LABELS.iter().enumerate() {
        let y = inner.y + 1 + i as u16;
        if y >= inner.y + inner.height {
            break;
        }
        let style = if form.focus == i { skin.accent() } else { skin.muted() };
        f.buffer_mut().set_stringn(inner.x + 2, y, label, label_w, style);
        let field = Rect { x: inner.x + 2 + label_w as u16, y, width: inner.width.saturating_sub(4 + label_w as u16), height: 1 };
        form.fields[i].render(f, field, &skin, form.focus == i, Line::default());
    }
    // Suggested units under "Equals": "↳ m meter · mi mile · mm millimeter".
    if !sugg.is_empty() && inner.height > 6 {
        let pick = form.pick.min(sugg.len() - 1);
        let mut spans = vec![Span::styled("↳ ", skin.muted())];
        for (i, &u) in sugg.iter().enumerate() {
            let u = &app.catalog.units[u];
            let style = if i == pick { skin.accent().add_modifier(ratatui::style::Modifier::BOLD | ratatui::style::Modifier::REVERSED) } else { skin.text() };
            if i > 0 {
                spans.push(Span::styled(" · ", skin.muted()));
            }
            spans.push(Span::styled(format!(" {} ", u.symbol), style));
            spans.push(Span::styled(format!(" {}", u.name), skin.muted()));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), Rect { x: inner.x + 2 + label_w as u16, y: inner.y + 4, width: inner.width.saturating_sub(4 + label_w as u16), height: 1 });
    }
    let text_area = Rect { x: inner.x + 2, y: inner.y + 6, width: inner.width.saturating_sub(4), height: inner.height.saturating_sub(7) };
    let style = if bad { Style::new().fg(skin.t.err) } else { skin.text() };
    let help = "Examples: step = 0.75 m · mug = 350 ml · workday = 8 h. Leave Equals empty to start a new kind (pizza), then slice = 1/8 pizza.";
    f.render_widget(
        Paragraph::new(vec![Line::from(Span::styled(msg, style)), Line::default(), Line::from(Span::styled(help, skin.muted()))]).wrap(Wrap { trim: true }),
        text_area,
    );
    let keys = match (sugg.is_empty(), form.editing.is_some()) {
        (false, _) => "↑↓ choose unit · Tab complete · ⏎ save · Esc cancel",
        (true, true) => "⏎ next / save · Tab move · ^D delete · Esc cancel",
        (true, false) => "⏎ next / save · Tab move · Esc cancel",
    };
    f.buffer_mut().set_stringn(inner.x + 2, inner.y + inner.height - 1, keys, inner.width.saturating_sub(4) as usize, skin.muted());
}
