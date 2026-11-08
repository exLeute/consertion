//! App state, navigation between screens, and actions. Screens (in `crate::screens`) draw
//! this state and translate their own keys/mouse into these actions.

use crate::catalog::{calc::CALCS, numbers, Catalog, Kind, Unit};
use crate::clipboard::Clipboard;
use crate::components::toast::Level;
use crate::components::browser::{FileBrowser, Picked};
use crate::components::{Button, Choice, InputEvent, ListView, TextInput, Toast};
use crate::files;
use crate::convert::{self, Output};
use crate::num::R;
use crate::rates::{self, Rates};
use crate::settings::{pair_key, Borders, Settings};
use crate::theme::{self, Skin};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use ratatui::layout::Rect;
use ratatui::widgets::BorderType;
use ratatui::DefaultTerminal;
use crate::translate::{self, Translation};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Screen {
    From,
    To,
    Convert,
    Calc,
    Translate,
    Files,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Overlay {
    None,
    Help,
    Settings,
    /// The file browser (`App::browser`).
    Browse,
    /// The custom converter form (`App::custom_form`).
    Custom,
}

/// What the file browser is choosing.
#[derive(Clone, Copy, PartialEq)]
pub enum BrowseFor {
    /// "Browse for a file…" on the search screen: the file decides the source format.
    Source,
    Input,
    Output,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FFocus {
    Input,
    Output,
    Options,
}

/// The file converter: paths, the chosen converter and its settings, and the running job.
pub struct FileConv {
    pub input: TextInput,
    pub output: TextInput,
    /// The user chose the output themselves; don't rename it when the input changes.
    pub output_custom: bool,
    pub focus: FFocus,
    pub backends: Vec<files::Backend>,
    pub backend: usize,
    pub opts: Vec<&'static files::Opt>,
    pub values: Vec<usize>,
    pub opt_sel: usize,
    pub opt_rows: Vec<Choice>,
    pub boxes: [Rect; 2],
    pub opts_box: Rect,
    pub status_box: Rect,
    pub button: Button,
    pub running: Option<files::Running>,
    pub result: Option<Result<files::Done, String>>,
    /// The output exists and the next Convert replaces it.
    pub confirm: bool,
}

/// A row in the picker: a unit, or a recently used/favorite pair (jumps straight to the converter).
#[derive(Clone, Copy, PartialEq)]
pub enum Entry {
    Unit(usize),
    Pair(usize, usize),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Row {
    /// Section header (category index); not selectable.
    Header(usize),
    Item(Entry),
}

/// Sidebar entries on the "from" screen.
#[derive(Clone, Copy, PartialEq)]
pub enum CatEntry {
    All,
    Favorites,
    Recent,
    Cat(usize),
    /// Section header ("Physics"…); not selectable.
    Group(&'static str),
}

#[derive(Clone, Copy, PartialEq)]
pub enum PFocus {
    Search,
    Cats,
    List,
}

pub struct Picker {
    pub search: TextInput,
    pub cats: ListView,
    pub list: ListView,
    pub cat: usize,
    /// The chosen category itself; `cat` is its index, which shifts when Favorites/Recent appear.
    cat_sel: CatEntry,
    pub cat_entries: Vec<CatEntry>,
    pub counts: Vec<usize>,
    pub rows: Vec<Row>,
    pub focus: PFocus,
    /// Narrow layout: "◀ category ▶" chip instead of a sidebar.
    pub chip: [Button; 2],
    pub sidebar_visible: bool,
    /// Search text of the "from" screen, restored when coming back to it.
    from_query: String,
    /// The search found nothing, so "New custom converter…" is offered for it.
    pub offer_new: bool,
}

/// Everything a key, footer hint or click can trigger.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    Swap,
    Copy,
    NewConversion,
    Refresh,
    Favorite,
    Back,
    Quit,
    Help,
    Settings,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CFocus {
    Side(usize),
    Table,
}

pub struct Conv {
    pub from: usize,
    pub to: usize,
    /// Both sides are editable; `active` is the one typed into last (the source of truth).
    pub sides: [TextInput; 2],
    pub active: usize,
    /// Why the computed side is empty, if it failed.
    pub error: Option<String>,
    pub exact: bool,
    pub table: ListView,
    pub table_units: Vec<usize>,
    /// Cached per recompute (huge numbers make this too slow to redo every frame).
    pub table_values: Vec<String>,
    /// Grouped display value of each numeric side, for the detail line.
    pub shown: [Option<String>; 2],
    pub swap_arrow: Rect,
    pub boxes: [Rect; 2],
    pub focus: CFocus,
    pub changed_at: Instant,
}

pub struct CalcState {
    pub unit: usize,
    pub inputs: Vec<TextInput>,
    pub focus: usize,
    pub boxes: Vec<Rect>,
    pub result_box: Rect,
    pub result: Result<Option<R>, String>,
    pub changed_at: Instant,
}

/// The translator: two multi-line boxes; the one typed into last is translated into the other.
pub struct Tr {
    pub sides: [TextInput; 2],
    pub active: usize,
    pub focus: usize,
    pub boxes: [Rect; 2],
    pub swap_arrow: Rect,
    rx: Option<Receiver<Result<Translation, String>>>,
    /// What the running request is for: (source side, text, from code, to code).
    inflight: Option<(usize, String, String, String)>,
    pub started: Instant,
    /// Translate when this moment passes (typing paused).
    due: Option<Instant>,
    pub error: Option<String>,
    pub detected: Option<String>,
    pub changed_at: Instant,
    cache: HashMap<String, Translation>,
}

pub struct App {
    pub catalog: Catalog,
    pub settings: Settings,
    pub skin: Skin,
    pub screen: Screen,
    pub overlay: Overlay,
    pub picker: Picker,
    pub from: Option<usize>,
    pub conv: Conv,
    pub calc: CalcState,
    pub tr: Tr,
    pub files: FileConv,
    pub browser: Option<FileBrowser>,
    pub custom_form: Option<crate::screens::custom::CustomForm>,
    /// Ctrl+D pressed once on one of your units (its id): a second press soon after deletes it.
    pub delete_armed: Option<(String, Instant)>,
    pub browse_for: BrowseFor,
    pub rates: Option<Rates>,
    rates_rx: Option<Receiver<Result<Rates, String>>>,
    pub rates_started: Instant,
    pub rates_error: Option<String>,
    pub clipboard: Clipboard,
    pub toast: Option<Toast>,
    pub screen_at: Instant,
    pub overlay_at: Instant,
    pub settings_sel: usize,
    pub settings_scroll: usize,
    /// Editing a free-text setting (API key, server URL).
    pub settings_edit: Option<TextInput>,
    pub settings_rows: Vec<Choice>,
    pub help: ListView,
    pub header: [Button; 2],
    pub crumbs: Vec<(Screen, Rect)>,
    pub footer: Vec<(Rect, Action)>,
    pub popup: Rect,
    pub mouse_pos: (u16, u16),
    anim_until: Instant,
    pub quit: bool,
}

fn skin_for(s: &Settings) -> Skin {
    let border = match s.borders {
        Borders::Rounded => BorderType::Rounded,
        Borders::Plain => BorderType::Plain,
        Borders::Double => BorderType::Double,
        Borders::Thick => BorderType::Thick,
    };
    Skin { t: theme::by_name(&s.theme), border, animations: s.animations }
}

/// An input box that only takes values of `kind` (numbers, dates…).
fn input_for(kind: Kind) -> TextInput {
    let mut t = TextInput::new(kind.placeholder());
    t.set_filter(Some(Box::new(move |s| kind.accepts(s))));
    t.paste_cleaner = kind.groupable().then_some(crate::num::sanitize_number as fn(&str) -> String);
    t
}

/// A path field: pasted or dropped paths lose their quotes, `file://` and escapes.
fn path_input() -> TextInput {
    let mut t = TextInput::new("");
    t.paste_cleaner = Some(|s| {
        // Only whole paths; a pasted file name or fragment goes in as typed.
        let t = s.trim();
        let whole = t.starts_with(['/', '~', '\'', '"', '\\']) || t.starts_with("file://") || t.get(1..2) == Some(":");
        if whole { files::display_path(&files::parse_path(t)) } else { s.to_string() }
    });
    t
}

impl App {
    pub fn new() -> Self {
        let settings = Settings::load();
        let mut search = TextInput::new("");
        search.prefix = "❯ ";
        let now = Instant::now();
        // In the past, so nothing animates on the very first frame.
        let long_ago = now.checked_sub(Duration::from_secs(5)).unwrap_or(now);
        let mut app = Self {
            catalog: Catalog::with_custom(&settings.custom),
            skin: skin_for(&settings),
            settings,
            screen: Screen::From,
            overlay: Overlay::None,
            picker: Picker {
                search,
                cats: ListView::default(),
                list: ListView::default(),
                cat: 0,
                cat_sel: CatEntry::All,
                cat_entries: Vec::new(),
                counts: Vec::new(),
                rows: Vec::new(),
                focus: PFocus::Search,
                chip: [Button::new("◀", ""), Button::new("▶", "")],
                sidebar_visible: true,
                from_query: String::new(),
                offer_new: false,
            },
            from: None,
            conv: Conv {
                from: 0,
                to: 0,
                sides: [TextInput::new(""), TextInput::new("")],
                active: 0,
                error: None,
                exact: true,
                table: ListView::default(),
                table_units: Vec::new(),
                table_values: Vec::new(),
                shown: [None, None],
                swap_arrow: Rect::default(),
                boxes: [Rect::default(); 2],
                focus: CFocus::Side(0),
                changed_at: long_ago,
            },
            calc: CalcState {
                unit: usize::MAX,
                inputs: Vec::new(),
                focus: 0,
                boxes: Vec::new(),
                result_box: Rect::default(),
                result: Ok(None),
                changed_at: long_ago,
            },
            tr: Tr {
                sides: [TextInput::multiline(""), TextInput::multiline("")],
                active: 0,
                focus: 0,
                boxes: [Rect::default(); 2],
                swap_arrow: Rect::default(),
                rx: None,
                inflight: None,
                started: now,
                due: None,
                error: None,
                detected: None,
                changed_at: long_ago,
                cache: HashMap::new(),
            },
            files: FileConv {
                input: path_input(),
                output: path_input(),
                output_custom: false,
                focus: FFocus::Input,
                backends: Vec::new(),
                backend: 0,
                opts: Vec::new(),
                values: Vec::new(),
                opt_sel: 0,
                opt_rows: Vec::new(),
                boxes: [Rect::default(); 2],
                opts_box: Rect::default(),
                status_box: Rect::default(),
                button: Button::new("Convert", "⏎"),
                running: None,
                result: None,
                confirm: false,
            },
            browser: None,
            custom_form: None,
            delete_armed: None,
            browse_for: BrowseFor::Input,
            // Cached rates only — the network is touched only when converting currencies.
            rates: Rates::load_cache(),
            rates_rx: None,
            rates_started: now,
            rates_error: None,
            clipboard: Clipboard::new(),
            toast: None,
            screen_at: long_ago,
            overlay_at: long_ago,
            settings_sel: 0,
            settings_scroll: 0,
            settings_edit: None,
            settings_rows: vec![Choice::default(); crate::settings::FIELDS.len()],
            help: ListView::default(),
            header: [Button::new("Help", "F1"), Button::new("Settings", "F2")],
            crumbs: Vec::new(),
            footer: Vec::new(),
            popup: Rect::default(),
            mouse_pos: (0, 0),
            anim_until: now,
            quit: false,
        };
        app.refresh_picker();
        if app.settings.resume {
            app.resume_last();
            app.screen_at = long_ago;
        }
        if !app.catalog.broken.is_empty() {
            let names = app.catalog.broken.join(", ");
            app.notify(format!("Some of your units refer to units that are gone: {names}"), Level::Warn);
        }
        app
    }

    /// Reopen the most recent conversion or calculator.
    fn resume_last(&mut self) {
        let Some((a, b)) = self.settings.recent.first().cloned() else { return };
        let (Some(a), Some(b)) = (self.catalog.find(&a), self.catalog.find(&b)) else { return };
        if matches!(self.catalog.units[a].kind, Kind::Calc(_)) {
            self.open_calc(a);
        } else {
            self.from = Some(a);
            self.open_converter(a, b);
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        let mut dirty = true;
        while !self.quit {
            dirty |= self.tick();
            if dirty || self.busy() {
                // Synchronized output: the terminal shows whole frames only (no flicker/tearing).
                let _ = execute!(std::io::stdout(), BeginSynchronizedUpdate);
                terminal.draw(|f| crate::screens::draw(f, self))?;
                let _ = execute!(std::io::stdout(), EndSynchronizedUpdate);
                dirty = false;
            }
            let timeout = if self.busy() { Duration::from_millis(16) } else { Duration::from_millis(500) };
            if event::poll(timeout)? {
                dirty |= self.on_event(event::read()?);
                // Drain bursts (wheel scrolling, fast typing) before the next redraw.
                while event::poll(Duration::ZERO)? {
                    dirty |= self.on_event(event::read()?);
                }
            }
        }
        self.shutdown();
        Ok(())
    }

    // ── Animation & background work ──────────────────────────────────────────

    pub fn animate(&mut self, ms: u64) {
        if self.skin.animations {
            self.anim_until = self.anim_until.max(Instant::now() + Duration::from_millis(ms));
        }
    }

    fn busy(&self) -> bool {
        Instant::now() < self.anim_until || self.rates_rx.is_some() || self.tr.rx.is_some() || self.tr.due.is_some() || self.files.running.is_some()
    }

    pub fn fetching(&self) -> bool {
        self.rates_rx.is_some()
    }

    /// Background housekeeping. Returns true if something visible changed.
    fn tick(&mut self) -> bool {
        let mut changed = false;
        if self.toast.as_ref().is_some_and(|t| !t.alive()) {
            self.toast = None;
            changed = true;
        }
        changed |= self.tick_translate();
        changed |= self.tick_files();
        let Some(rx) = &self.rates_rx else { return changed };
        match rx.try_recv() {
            Ok(Ok(r)) => {
                self.notify(format!("Live rates loaded ({})", r.date), Level::Ok);
                self.rates = Some(r);
                self.rates_error = None;
                self.rates_rx = None;
                self.recompute();
            }
            Ok(Err(e)) => {
                self.rates_rx = None;
                match &self.rates {
                    Some(r) => self.notify(format!("Offline — using cached rates from {}", r.date), Level::Warn),
                    None => self.notify("Couldn't fetch exchange rates", Level::Error),
                }
                self.rates_error = Some(e);
                self.recompute();
            }
            Err(TryRecvError::Disconnected) => self.rates_rx = None,
            Err(TryRecvError::Empty) => return changed,
        }
        true
    }

    pub fn notify(&mut self, msg: impl Into<String>, level: Level) {
        self.toast = Some(Toast::new(msg, level));
        self.animate(250);
    }

    /// Fetch rates if missing/stale (or `force`). Only ever called for currency conversions.
    pub fn ensure_rates(&mut self, force: bool) {
        if self.rates_rx.is_some() {
            return;
        }
        let stale = self.rates.as_ref().is_none_or(|r| r.age_secs() > self.settings.rates_ttl_min * 60 || r.date < rates::today_utc());
        if force || stale {
            self.rates_rx = Some(rates::fetch_async());
            self.rates_started = Instant::now();
        }
    }

    // ── Navigation ───────────────────────────────────────────────────────────

    pub fn go(&mut self, screen: Screen) {
        if self.screen == Screen::From {
            self.picker.from_query = self.picker.search.text();
        }
        if screen == Screen::From {
            self.from = None;
        }
        self.screen = screen;
        self.screen_at = Instant::now();
        self.animate(200);
        if matches!(screen, Screen::From | Screen::To) {
            let q = if screen == Screen::From { self.picker.from_query.clone() } else { String::new() };
            self.picker.search.set_text(&q);
            self.picker.focus = PFocus::Search;
            self.refresh_picker();
        }
    }

    pub fn back(&mut self) {
        match self.screen {
            Screen::Convert | Screen::Translate | Screen::Files => self.go(Screen::To),
            Screen::To | Screen::Calc => self.go(Screen::From),
            Screen::From => self.notify("Press Ctrl+Q to quit", Level::Info),
        }
    }

    fn entry_key(&self, e: Entry) -> String {
        let id = |u: usize| self.catalog.units[u].id.clone();
        match e {
            Entry::Pair(a, b) if a != b => pair_key(&id(a), &id(b)),
            Entry::Pair(a, _) | Entry::Unit(a) => id(a),
        }
    }

    pub fn is_favorite(&self, e: Entry) -> bool {
        self.settings.is_favorite(&self.entry_key(e))
    }

    fn parse_key(&self, key: &str) -> Option<Entry> {
        match key.split_once('>') {
            Some((a, b)) => Some(Entry::Pair(self.catalog.find(a)?, self.catalog.find(b)?)),
            None => self.catalog.find(key).map(Entry::Unit),
        }
    }

    pub fn toggle_favorite(&mut self, e: Entry) {
        let key = self.entry_key(e);
        let on = self.settings.toggle_favorite(&key);
        self.settings.save();
        self.notify(if on { "★ Added to favorites" } else { "Removed from favorites" }, Level::Ok);
        if matches!(self.screen, Screen::From | Screen::To) {
            let sel = self.picker.list.selected;
            self.refresh_picker();
            self.picker.list.select(sel);
        }
    }

    /// What a star on a picker row means: on the "to" screen, the (from, to) pair.
    pub fn fav_entry(&self, e: Entry) -> Entry {
        match (self.screen, e, self.from) {
            (Screen::To, Entry::Unit(t), Some(f)) => Entry::Pair(f, t),
            _ => e,
        }
    }

    pub fn selected_entry(&self) -> Option<Entry> {
        match self.picker.rows.get(self.picker.list.selected) {
            Some(Row::Item(e)) => Some(*e),
            _ => None,
        }
    }

    pub fn refresh_picker(&mut self) {
        let q = self.picker.search.text();
        let c = &self.catalog;
        let matches_query = |e: &Entry, all: &[usize]| match e {
            _ if q.is_empty() => true,
            Entry::Unit(u) => all.contains(u),
            Entry::Pair(a, b) => all.contains(a) || all.contains(b),
        };
        let (rows, grouped) = match self.screen {
            Screen::To => {
                let from = &c.units[self.from.unwrap_or(0)];
                let rows = c
                    .search(&q, Some(from.category), |u| match (from.kind, u.kind) {
                        (Kind::File(a), Kind::File(b)) => files::convertible(a, b),
                        _ => u.id != from.id && u.kind.can_target() && (u.tag == 0 || from.tag == 0 || u.tag == from.tag),
                    })
                    .into_iter()
                    // Same-format re-encoding (resize, compress) comes after real conversions.
                    .filter(|&u| u != self.from.unwrap_or(usize::MAX))
                    .chain(self.from.filter(|&f| matches!(c.units[f].kind, Kind::File(a) if files::convertible(a, a))))
                    .map(Entry::Unit)
                    .collect();
                self.picker.cat_entries.clear();
                (rows, false)
            }
            _ => {
                let mut all = c.search(&q, None, |u| u.kind.can_parse());
                // Nothing found: offer to make it one of your own units.
                self.picker.offer_new = all.is_empty() && !q.trim().is_empty();
                if self.picker.offer_new
                    && let Some(new) = c.units.iter().position(|u| u.kind == Kind::NewCustom)
                {
                    all.push(new);
                }
                let recent: Vec<Entry> = self
                    .settings
                    .recent
                    .iter()
                    .filter_map(|(a, b)| Some(Entry::Pair(c.find(a)?, c.find(b)?)))
                    .filter(|e| matches_query(e, &all))
                    .collect();
                let favs: Vec<Entry> = self.settings.favorites.iter().filter_map(|k| self.parse_key(k)).filter(|e| matches_query(e, &all)).collect();
                let p = &mut self.picker;
                p.cat_entries = vec![CatEntry::All];
                p.counts = vec![all.len()];
                if !self.settings.favorites.is_empty() {
                    p.cat_entries.push(CatEntry::Favorites);
                    p.counts.push(favs.len());
                }
                if !self.settings.recent.is_empty() {
                    p.cat_entries.push(CatEntry::Recent);
                    p.counts.push(recent.len());
                }
                for &i in &c.order {
                    let group = c.categories[i].group;
                    if !group.is_empty() && !matches!(p.cat_entries.last(), Some(CatEntry::Cat(j)) if c.categories[*j].group == group) {
                        p.cat_entries.push(CatEntry::Group(group));
                        p.counts.push(0);
                    }
                    p.cat_entries.push(CatEntry::Cat(i));
                    p.counts.push(all.iter().filter(|&&u| in_category(c, u, i)).count());
                }
                p.cats.inert = p.cat_entries.iter().map(|e| matches!(e, CatEntry::Group(_))).collect();
                p.cat = p.cat_entries.iter().position(|e| *e == p.cat_sel).unwrap_or(0);
                p.cats.select(p.cat);
                match p.cat_entries[p.cat] {
                    CatEntry::All => {
                        let mut all = all;
                        // Unsearched, list categories in sidebar order; a search keeps best matches first.
                        if q.trim().is_empty() {
                            all.sort_by_key(|&u| c.categories[c.units[u].category].rank);
                        }
                        (all.into_iter().map(Entry::Unit).collect(), true)
                    }
                    CatEntry::Favorites => (favs, true),
                    CatEntry::Recent => (recent, true),
                    CatEntry::Cat(i) => (all.into_iter().filter(|&u| in_category(c, u, i)).map(Entry::Unit).collect(), false),
                    CatEntry::Group(_) => (Vec::new(), false),
                }
            }
        };
        let p = &mut self.picker;
        p.rows = if grouped { group_rows(c, rows) } else { rows.into_iter().map(Row::Item).collect() };
        p.list.inert = p.rows.iter().map(|r| matches!(r, Row::Header(_))).collect();
        p.list.reset();
    }

    pub fn set_category(&mut self, i: usize) {
        if i != self.picker.cat && self.picker.cat_entries.get(i).is_some_and(|e| !matches!(e, CatEntry::Group(_))) {
            self.picker.cat = i;
            self.picker.cat_sel = self.picker.cat_entries[i];
            self.refresh_picker();
        }
    }

    pub fn activate_entry(&mut self) {
        let Some(entry) = self.selected_entry() else { return };
        let is_calc = |u: usize| matches!(self.catalog.units[u].kind, Kind::Calc(_));
        match (self.screen, entry) {
            (_, Entry::Unit(u) | Entry::Pair(u, _)) if is_calc(u) => self.open_calc(u),
            (_, Entry::Unit(u)) if self.catalog.units[u].kind == Kind::File(files::PICK) => self.open_browser(BrowseFor::Source),
            (_, Entry::Unit(u)) if self.catalog.units[u].kind == Kind::NewCustom => {
                let name = self.picker.offer_new.then(|| self.picker.search.text());
                crate::screens::custom::open(self, None, name)
            }
            (Screen::From, Entry::Unit(u)) => {
                self.from = Some(u);
                self.go(Screen::To);
            }
            (_, Entry::Pair(a, b)) => {
                self.from = Some(a);
                self.open_converter(a, b);
            }
            (_, Entry::Unit(t)) => {
                let from = self.from.unwrap_or(0);
                self.open_converter(from, t);
            }
        }
    }

    fn enter_screen(&mut self, s: Screen) {
        if self.screen != s {
            if self.screen == Screen::From {
                self.picker.from_query = self.picker.search.text();
            }
            self.screen = s;
            self.screen_at = Instant::now();
            self.animate(200);
        }
    }

    // ── Converter ────────────────────────────────────────────────────────────

    fn configure_side(&mut self, side: usize) {
        let u = &self.catalog.units[[self.conv.from, self.conv.to][side]];
        let kind = u.kind;
        let input = &mut self.conv.sides[side];
        input.set_filter(Some(Box::new(move |s| kind.accepts(s))));
        input.paste_cleaner = kind.groupable().then_some(crate::num::sanitize_number as fn(&str) -> String);
        input.placeholder = kind.placeholder().into();
    }

    pub fn open_converter(&mut self, from: usize, to: usize) {
        let (fid, tid) = (self.catalog.units[from].id.clone(), self.catalog.units[to].id.clone());
        self.settings.push_recent(&fid, &tid);
        self.settings.save();
        let kind = self.catalog.units[from].kind;
        match kind {
            Kind::Lang(_) => return self.open_translator(from, to),
            Kind::File(_) => return self.open_files(from, to),
            _ => {}
        }
        let category = self.catalog.units[from].category;
        // Keep the typed value within a category (swap, other target); new topics get an example.
        let same_topic = self.screen == Screen::Convert && self.catalog.units[self.conv.from].category == category;
        let sample = sample_input(&self.catalog.units[from], self.catalog.categories[category].id);
        let c = &mut self.conv;
        c.from = from;
        c.to = to;
        let current = c.sides[0].text();
        if current.is_empty() || !kind.accepts(&current) || !same_topic {
            c.sides[0].set_text(&sample);
        }
        c.sides[0].select_all(); // typing replaces the sample right away
        c.active = 0;
        c.table_units = self.catalog.categories[category].units.iter().copied().filter(|&u| u != from && self.catalog.pairs(from, u)).collect();
        c.table.reset();
        let at = c.table_units.iter().position(|&u| u == to).unwrap_or(0);
        c.table.select(at);
        c.focus = CFocus::Side(0);
        self.configure_side(0);
        self.configure_side(1);
        self.enter_screen(Screen::Convert);
        if self.catalog.categories[category].online {
            self.ensure_rates(false);
        }
        self.recompute();
    }

    /// The screen that comes after picking a target, and its step name.
    pub fn final_step(&self) -> (Screen, &'static str) {
        match self.from.map(|u| self.catalog.units[u].kind) {
            Some(Kind::Lang(_)) => (Screen::Translate, "Translate"),
            Some(Kind::File(_)) => (Screen::Files, "Convert"),
            _ => (Screen::Convert, "Convert"),
        }
    }

    pub fn unit(&self, side: usize) -> &Unit {
        &self.catalog.units[[self.conv.from, self.conv.to][side]]
    }

    pub fn source_unit(&self) -> &Unit {
        self.unit(0)
    }

    pub fn target_unit(&self) -> &Unit {
        self.unit(1)
    }

    pub fn fmt(&self, kind: Kind, q: &R) -> (String, bool) {
        convert::format_value(kind, q, self.settings.precision, self.settings.notation)
    }

    pub fn grouped(&self, s: &str) -> String {
        match self.settings.grouping_chars() {
            Some((sep, dec)) => convert::group_digits(s, sep, dec),
            None => s.to_string(),
        }
    }

    /// Recompute the passive side from the active one.
    pub fn recompute(&mut self) {
        match self.screen {
            Screen::Calc => return self.recompute_calc(),
            Screen::Translate | Screen::Files => return,
            _ => {}
        }
        let (src, dst) = (self.conv.active, 1 - self.conv.active);
        let out = convert::convert(self.unit(src), self.unit(dst), &self.conv.sides[src].text(), self.rates.as_ref());
        let kind = self.unit(dst).kind;
        let (text, error, exact) = match out {
            Ok(Output::Empty) => (String::new(), None, true),
            Ok(Output::Number(q)) => {
                let (s, exact) = self.fmt(kind, &q);
                (s, None, exact)
            }
            Ok(Output::Text(s)) => (s, None, true),
            Err(e) => (String::new(), Some(e), true),
        };
        let c = &mut self.conv;
        if text != c.sides[dst].text() || error != c.error {
            c.sides[dst].set_text(&text);
            c.error = error;
            c.changed_at = Instant::now();
            self.animate(350);
        }
        self.conv.exact = exact;

        let shown = [0, 1].map(|side| {
            let kind = self.unit(side).kind;
            let q = convert::number(&self.conv.sides[side].text()).ok()??;
            kind.groupable().then(|| self.grouped(&self.fmt(kind, &q).0))
        });
        let input = self.conv.sides[src].text();
        let table_values = if self.settings.show_table {
            self.conv
                .table_units
                .iter()
                .map(|&u| {
                    let to = &self.catalog.units[u];
                    match convert::convert(self.unit(src), to, &input, self.rates.as_ref()) {
                        Ok(Output::Number(v)) => self.fmt(to.kind, &v).0,
                        Ok(Output::Text(s)) => s,
                        _ => "—".into(),
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        self.conv.shown = shown;
        self.conv.table_values = table_values;
    }

    /// The computed (passive) side, for copying.
    pub fn result_text(&self) -> Option<String> {
        let t = match self.screen {
            Screen::Calc => {
                let q = self.calc.result.as_ref().ok()?.as_ref()?;
                self.fmt(self.calc_def().out, q).0
            }
            Screen::Translate => self.tr.sides[1 - self.tr.active].text(),
            Screen::Files => match &self.files.result {
                Some(Ok(d)) => d.output.to_string_lossy().into_owned(),
                _ => self.output_path().to_string_lossy().into_owned(),
            },
            _ => self.conv.sides[1 - self.conv.active].text(),
        };
        (!t.is_empty()).then_some(t)
    }

    /// Change the target unit in place, keeping the "from" value.
    pub fn set_target(&mut self, to: usize, remember: bool) {
        self.conv.to = to;
        self.conv.active = 0;
        self.configure_side(1);
        if remember {
            let (a, b) = (self.source_unit().id.clone(), self.target_unit().id.clone());
            self.settings.push_recent(&a, &b);
            self.settings.save();
        }
        self.recompute();
    }

    pub fn side_event(&mut self, side: usize, ev: InputEvent) {
        match ev {
            InputEvent::Changed => {
                self.conv.active = side;
                self.recompute();
            }
            InputEvent::Rejected => self.reject_hint(self.unit(side).kind),
            _ => {}
        }
    }

    // ── Translator ───────────────────────────────────────────────────────────

    fn open_translator(&mut self, from: usize, to: usize) {
        let staying = self.screen == Screen::Translate;
        let langs_changed = (self.conv.from, self.conv.to) != (from, to);
        self.conv.from = from;
        self.conv.to = to;
        let tr = &mut self.tr;
        for side in &mut tr.sides {
            side.placeholder = Kind::Lang(1).placeholder().into();
            side.set_filter(Some(Box::new(|s| Kind::Lang(1).accepts(s))));
        }
        tr.sides[1].placeholder = "Translation appears here — you can type here too".into();
        if !staying {
            tr.focus = 0;
            tr.active = 0;
        }
        tr.error = None;
        if langs_changed {
            tr.detected = None;
        }
        self.enter_screen(Screen::Translate);
        if langs_changed && !self.tr.sides[self.tr.active].text().trim().is_empty() {
            self.request_translation(false);
        }
    }

    pub fn lang_code(&self, side: usize) -> &'static str {
        match self.unit(side).kind {
            Kind::Lang(i) => crate::catalog::lang::code(i),
            _ => "auto",
        }
    }

    /// Text in a translator box changed: translate after a pause (or wait for Ctrl+R).
    pub fn translator_typed(&mut self, side: usize) {
        self.tr.active = side;
        self.tr.error = None;
        if self.tr.sides[side].text().trim().is_empty() {
            self.tr.sides[1 - side].set_text("");
            self.tr.due = None;
            return;
        }
        if self.settings.translate_live {
            self.tr.due = Some(Instant::now() + Duration::from_millis(700));
        }
    }

    /// Translate the active box now (`force` skips the cache).
    pub fn request_translation(&mut self, force: bool) {
        self.tr.due = None;
        let src = self.tr.active;
        let text = self.tr.sides[src].text();
        if text.trim().is_empty() {
            return;
        }
        let from = self.lang_code(src).to_string();
        let to = match self.lang_code(1 - src) {
            "auto" => match self.tr.detected.clone() {
                Some(d) => d.to_lowercase(),
                None => {
                    self.tr.error = Some("Choose the language of the left box to translate back into it".into());
                    return;
                }
            },
            c => c.to_string(),
        };
        let key = format!("{:?}|{from}|{to}|{text}", self.settings.provider);
        if !force && let Some(t) = self.tr.cache.get(&key).cloned() {
            return self.apply_translation(src, t);
        }
        if self.tr.rx.is_some() {
            // One request at a time; this one runs when the current one is back.
            self.tr.due = Some(Instant::now());
            return;
        }
        let req = translate::Request::new(&self.settings, &text, &from, &to);
        if let Err(e) = req.check() {
            self.tr.error = Some(e);
            return;
        }
        self.tr.rx = Some(translate::translate_async(req));
        self.tr.inflight = Some((src, text, from, to));
        self.tr.started = Instant::now();
    }

    fn apply_translation(&mut self, src: usize, t: Translation) {
        if src == 0 && self.lang_code(0) == "auto" && t.detected.is_some() {
            self.tr.detected = t.detected.clone();
        }
        let tr = &mut self.tr;
        tr.error = None;
        if t.text != tr.sides[1 - src].text() {
            tr.sides[1 - src].set_text(&t.text);
            tr.changed_at = Instant::now();
            self.animate(350);
        }
    }

    fn tick_translate(&mut self) -> bool {
        if self.tr.rx.is_none() {
            if self.tr.due.is_some_and(|d| Instant::now() >= d) {
                self.request_translation(false);
                return true;
            }
            return false;
        }
        let result = match self.tr.rx.as_ref().map(|rx| rx.try_recv()) {
            Some(Ok(r)) => r,
            Some(Err(TryRecvError::Empty)) | None => return false,
            Some(Err(TryRecvError::Disconnected)) => Err("Translation stopped unexpectedly".into()),
        };
        self.tr.rx = None;
        let Some((src, text, from, to)) = self.tr.inflight.take() else { return true };
        // Typed on since? Then this answer is stale; a new request is already due.
        let current = self.screen == Screen::Translate && self.tr.active == src && self.tr.sides[src].text() == text && self.lang_code(src) == from;
        match result {
            Ok(t) => {
                self.tr.cache.insert(format!("{:?}|{from}|{to}|{text}", self.settings.provider), t.clone());
                if current {
                    self.apply_translation(src, t);
                }
            }
            Err(e) if current => self.tr.error = Some(e),
            Err(_) => {}
        }
        true
    }

    pub fn translating(&self) -> bool {
        self.tr.rx.is_some()
    }

    // ── Files ────────────────────────────────────────────────────────────────

    /// The file format of the source (0) or target (1) unit.
    pub fn file_fmt(&self, side: usize) -> usize {
        match self.unit(side).kind {
            Kind::File(i) if i != files::PICK => i,
            _ => 0,
        }
    }

    fn format_unit(&self, fmt: usize) -> Option<usize> {
        self.catalog.units.iter().position(|u| u.kind == Kind::File(fmt))
    }

    fn open_files(&mut self, from: usize, to: usize) {
        self.conv.from = from;
        self.conv.to = to;
        let f = &mut self.files;
        // A file of another type than the chosen source doesn't belong here.
        let typed = files::parse_path(&f.input.text());
        let src = self.file_fmt(0);
        if !self.files.input.is_empty() && files::of_path(&typed).is_some_and(|i| i != src) {
            self.files.input.set_text("");
            self.files.output_custom = false;
        }
        self.setup_file_backends();
        let f = &mut self.files;
        f.result = None;
        f.confirm = false;
        f.focus = if f.input.is_empty() { FFocus::Input } else { FFocus::Output };
        self.refresh_default_output();
        self.enter_screen(Screen::Files);
    }

    /// Converters for the current pair, and their settings with the last used choices.
    fn setup_file_backends(&mut self) {
        let (a, b) = (self.file_fmt(0), self.file_fmt(1));
        let f = &mut self.files;
        let keep = f.backends.get(f.backend).copied();
        f.backends = files::backends(a, b);
        f.backend = keep.and_then(|k| f.backends.iter().position(|&x| x == k)).unwrap_or(0);
        self.setup_file_options();
    }

    fn setup_file_options(&mut self) {
        let (a, b) = (self.file_fmt(0), self.file_fmt(1));
        let f = &mut self.files;
        f.opts = f.backends.get(f.backend).map_or(Vec::new(), |&be| files::options(a, b, be));
        f.values = f
            .opts
            .iter()
            .map(|o| self.settings.file_opts.get(o.key).and_then(|v| o.choices.iter().position(|c| c == v)).unwrap_or(o.default))
            .collect();
        self.files.opt_sel = self.files.opt_sel.min(self.option_rows().saturating_sub(1));
    }

    /// Settings rows: the converter choice (when there is one) and its options.
    pub fn option_rows(&self) -> usize {
        self.files.opts.len() + usize::from(self.files.backends.len() > 1)
    }

    pub fn option_row(&self, row: usize) -> (&'static str, String) {
        let f = &self.files;
        match row.checked_sub(usize::from(f.backends.len() > 1)) {
            None => ("Converter", f.backends[f.backend].name().to_string()),
            Some(i) => (f.opts[i].label, f.opts[i].choices[f.values[i]].to_string()),
        }
    }

    pub fn step_file_option(&mut self, row: usize, d: i32) {
        let f = &mut self.files;
        let cycle = |i: usize, n: usize| (i as i32 + d).rem_euclid(n.max(1) as i32) as usize;
        match row.checked_sub(usize::from(f.backends.len() > 1)) {
            None => {
                f.backend = cycle(f.backend, f.backends.len());
                self.setup_file_options();
            }
            Some(i) if i < f.opts.len() => {
                f.values[i] = cycle(f.values[i], f.opts[i].choices.len());
                self.settings.file_opts.insert(f.opts[i].key.to_string(), f.opts[i].choices[f.values[i]].to_string());
                self.settings.save();
            }
            Some(_) => {}
        }
        self.files.result = None;
    }

    pub fn output_path(&self) -> std::path::PathBuf {
        let mut p = files::parse_path(&self.files.output.text());
        if p.extension().is_none() && !p.as_os_str().is_empty() && !p.is_dir() {
            p.set_extension(files::FORMATS[self.file_fmt(1)].ext);
        }
        p
    }

    fn refresh_default_output(&mut self) {
        let input = files::parse_path(&self.files.input.text());
        if self.files.output_custom || self.files.input.text().trim().is_empty() {
            return;
        }
        let out = files::default_output(&input, self.file_fmt(1));
        self.files.output.set_text(&files::display_path(&out));
    }

    pub fn file_input_changed(&mut self) {
        let f = &mut self.files;
        f.result = None;
        f.confirm = false;
        // Another type of file: switch the source format when it converts to the same target.
        let p = files::parse_path(&f.input.text());
        if let Some(i) = files::of_path(&p)
            && i != self.file_fmt(0)
            && files::convertible(i, self.file_fmt(1))
            && let Some(u) = self.format_unit(i)
        {
            self.conv.from = u;
            self.from = Some(u);
            self.setup_file_backends();
        }
        self.refresh_default_output();
    }

    pub fn file_output_changed(&mut self) {
        let f = &mut self.files;
        f.output_custom = !f.output.text().trim().is_empty();
        f.confirm = false;
        f.result = None;
    }

    pub fn start_file_convert(&mut self) {
        if self.files.running.is_some() {
            return;
        }
        let input = files::parse_path(&self.files.input.text());
        if self.files.input.text().trim().is_empty() || input.is_dir() {
            return self.open_browser(BrowseFor::Input);
        }
        if self.files.output.text().trim().is_empty() {
            self.files.output_custom = false;
            self.refresh_default_output();
        }
        let output = self.output_path();
        let (a, b) = (self.file_fmt(0), self.file_fmt(1));
        let Some(&backend) = self.files.backends.get(self.files.backend) else {
            self.files.result = Some(Err(format!("Nothing on this computer converts this — install {}", files::suggest(a))));
            return;
        };
        if let Some(i) = files::of_path(&input)
            && !files::convertible(i, b)
        {
            self.files.result = Some(Err(format!("{} can't be converted to {}", files::FORMATS[i].name, files::FORMATS[b].name)));
            return;
        }
        if output.exists() && !self.files.confirm && files::validate(&input, &output, true).is_ok() {
            self.files.confirm = true;
            self.files.result = None;
            self.animate(400);
            return;
        }
        if let Err(e) = files::validate(&input, &output, self.files.confirm) {
            self.files.result = Some(Err(e));
            self.files.confirm = false;
            return;
        }
        let f = &mut self.files;
        let opts = f.opts.iter().zip(&f.values).map(|(o, &v)| (o.key, v)).collect();
        let job = files::Job { input, output, from: a, to: b, backend, opts, overwrite: f.confirm };
        f.running = Some(files::start(job));
        f.result = None;
        f.confirm = false;
    }

    pub fn cancel_file_convert(&mut self) {
        if let Some(r) = &self.files.running {
            r.cancel();
            self.notify("Cancelling…", Level::Info);
        }
    }

    fn tick_files(&mut self) -> bool {
        let Some(r) = &mut self.files.running else { return false };
        let mut done = None;
        let mut changed = false;
        loop {
            match r.rx.try_recv() {
                Ok(files::Event::Progress(p)) => {
                    r.progress = Some(p);
                    changed = true;
                }
                Ok(files::Event::Done(d)) => {
                    done = Some(d);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    done = Some(Err("The conversion stopped unexpectedly".into()));
                    break;
                }
            }
        }
        let Some(d) = done else { return changed };
        self.files.running = None;
        match &d {
            Ok(x) => self.notify(format!("Saved {}", x.output.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned())), Level::Ok),
            Err(e) if e == "Cancelled" => self.notify("Cancelled — nothing was written", Level::Info),
            Err(_) => self.notify("Conversion failed", Level::Error),
        }
        self.files.result = Some(d);
        true
    }

    /// Before quitting: stop a running conversion so no tool keeps writing in the background.
    pub fn shutdown(&mut self) {
        if let Some(r) = self.files.running.take() {
            r.cancel();
            let _ = r.rx.recv_timeout(Duration::from_secs(3));
        }
    }

    pub fn browse_files(&mut self) {
        let what = if self.files.focus == FFocus::Output { BrowseFor::Output } else { BrowseFor::Input };
        self.open_browser(what);
    }

    pub fn open_browser(&mut self, what: BrowseFor) {
        let exts: Vec<&'static str> = match what {
            BrowseFor::Source => Vec::new(),
            BrowseFor::Input => {
                // Every format that converts to the chosen target.
                let to = self.file_fmt(1);
                files::FORMATS.iter().enumerate().filter(|(i, _)| files::convertible(*i, to)).flat_map(|(_, f)| std::iter::once(f.ext).chain(f.alias.iter().copied())).collect()
            }
            BrowseFor::Output => {
                let f = &files::FORMATS[self.file_fmt(1)];
                std::iter::once(f.ext).chain(f.alias.iter().copied()).collect()
            }
        };
        let start = match what {
            BrowseFor::Output if !self.files.output.text().trim().is_empty() => self.output_path(),
            _ => files::parse_path(&self.files.input.text()),
        };
        let title = match what {
            BrowseFor::Source => "Choose a file to convert".to_string(),
            BrowseFor::Input => format!("Choose a file to convert to {}", files::FORMATS[self.file_fmt(1)].name),
            BrowseFor::Output => "Choose where to save".to_string(),
        };
        self.browser = Some(FileBrowser::open(&start, exts, what == BrowseFor::Output, self.settings.show_hidden, &title));
        self.browse_for = what;
        self.overlay = Overlay::Browse;
        self.overlay_at = Instant::now();
        self.animate(200);
    }

    fn browse_picked(&mut self, p: std::path::PathBuf) {
        let shown = files::display_path(&p);
        match self.browse_for {
            BrowseFor::Source => {
                let Some(u) = files::of_path(&p).and_then(|i| self.format_unit(i)) else {
                    let ext = p.extension().map_or("(none)".into(), |e| format!(".{}", e.to_string_lossy()));
                    return self.notify(format!("Unknown file type {ext} — pick a format in the search instead"), Level::Warn);
                };
                self.files.input.set_text(&shown);
                self.files.output_custom = false;
                self.overlay = Overlay::None;
                self.from = Some(u);
                self.go(Screen::To);
            }
            BrowseFor::Input => {
                self.files.input.set_text(&shown);
                self.overlay = Overlay::None;
                self.file_input_changed();
                self.files.focus = FFocus::Output;
            }
            BrowseFor::Output => {
                let path = if p.is_dir() {
                    let name = self.output_path().file_name().map(|n| n.to_os_string()).unwrap_or_else(|| {
                        files::default_output(&files::parse_path(&self.files.input.text()), self.file_fmt(1)).file_name().unwrap_or_default().to_os_string()
                    });
                    p.join(name)
                } else {
                    p
                };
                self.files.output.set_text(&files::display_path(&path));
                self.overlay = Overlay::None;
                self.file_output_changed();
            }
        }
    }

    pub fn browser_key(&mut self, k: KeyEvent) {
        let Some(b) = &mut self.browser else { return self.overlay = Overlay::None };
        let ctrl_h = k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('h');
        match b.handle_key(k, &mut self.clipboard) {
            Picked::Path(p) => self.browse_picked(p),
            Picked::Close => self.overlay = Overlay::None,
            Picked::None if ctrl_h => {
                self.settings.show_hidden = b.hidden;
                self.settings.save();
            }
            Picked::None => {}
        }
    }

    pub fn browser_mouse(&mut self, m: MouseEvent) {
        let Some(b) = &mut self.browser else { return };
        if let Picked::Path(p) = b.handle_mouse(m) {
            self.browse_picked(p);
        }
    }

    // ── Calculators ──────────────────────────────────────────────────────────

    pub fn calc_def(&self) -> &'static crate::catalog::calc::Calc {
        match self.catalog.units.get(self.calc.unit).map(|u| u.kind) {
            Some(Kind::Calc(i)) => &CALCS[i],
            _ => &CALCS[0],
        }
    }

    pub fn open_calc(&mut self, unit: usize) {
        let id = self.catalog.units[unit].id.clone();
        self.settings.push_recent(&id, &id);
        self.settings.save();
        // Reopening the same calculator keeps what was typed; another one starts from its examples.
        if self.calc.unit != unit {
            self.calc.unit = unit;
            let today = crate::catalog::time::today();
            self.calc.inputs = self
                .calc_def()
                .inputs
                .iter()
                .map(|i| {
                    let mut t = input_for(i.kind);
                    let text = match (i.default, i.kind) {
                        ("today", Kind::Date(f)) => f.format(&today).unwrap_or_default().replace("T00:00:00Z", ""),
                        (d, _) => d.to_string(),
                    };
                    t.set_text(&text);
                    t
                })
                .collect();
            self.calc.boxes = vec![Rect::default(); self.calc.inputs.len()];
        }
        self.calc.focus = 0;
        self.calc.inputs[0].select_all();
        self.enter_screen(Screen::Calc);
        self.recompute_calc();
    }

    pub fn recompute_calc(&mut self) {
        let def = self.calc_def();
        let vals: Result<Vec<Option<R>>, String> =
            self.calc.inputs.iter().zip(def.inputs).map(|(t, i)| convert::parse_quantity(i.kind, &t.text())).collect();
        let result = match vals {
            Err(e) => Err(e),
            Ok(v) if v.iter().any(Option::is_none) => Ok(None),
            Ok(v) => {
                let v: Vec<R> = v.into_iter().flatten().collect();
                match (def.compute)(&v) {
                    Ok(r) if !crate::num::in_range(&r) => Err("Result is too large to show".into()),
                    Ok(r) => match def.out {
                        Kind::Date(f) => f.format(&r).map(|_| Some(r)),
                        _ => Ok(Some(r)),
                    },
                    Err(e) => Err(e.into()),
                }
            }
        };
        if result != self.calc.result {
            self.calc.result = result;
            self.calc.changed_at = Instant::now();
            self.animate(350);
        }
    }

    // ── Actions ──────────────────────────────────────────────────────────────

    pub fn run_action(&mut self, action: Action) {
        match action {
            Action::Copy => match self.result_text() {
                Some(s) => {
                    let (kind, symbol) = match self.screen {
                        Screen::Calc => (self.calc_def().out, self.calc_def().suffix.trim().to_string()),
                        Screen::Translate | Screen::Files => (Kind::Lang(1), String::new()),
                        _ => (self.unit(1 - self.conv.active).kind, self.unit(1 - self.conv.active).symbol.clone()),
                    };
                    let s = self.copy_form(s, kind, &symbol);
                    self.copy(s);
                }
                None => self.notify("Nothing to copy yet", Level::Warn),
            },
            Action::Swap if self.screen == Screen::Translate => {
                let (from, to) = (self.conv.from, self.conv.to);
                let new_to = match self.lang_code(0) {
                    "auto" => match self.tr.detected.as_deref().and_then(|d| self.catalog.exact(d, self.unit(0).category).or_else(|| {
                        let name = crate::catalog::lang::name_of(d)?;
                        self.catalog.exact(name, self.unit(0).category)
                    })) {
                        Some(u) => u,
                        None => return self.notify("Type something first so the language can be detected", Level::Warn),
                    },
                    _ => from,
                };
                let texts = [self.tr.sides[1].text(), self.tr.sides[0].text()];
                for (side, t) in self.tr.sides.iter_mut().zip(texts) {
                    side.set_text(&t);
                }
                self.tr.active = 0;
                self.tr.detected = None;
                self.from = Some(to);
                self.open_converter(to, new_to);
                self.animate(200);
            }
            Action::Swap if self.screen == Screen::Files => {
                let (from, to) = (self.conv.from, self.conv.to);
                if !files::convertible(self.file_fmt(1), self.file_fmt(0)) {
                    return self.notify("This conversion only works one way", Level::Warn);
                }
                // Convert the result back: the old output becomes the input.
                let out = self.output_path();
                if out.is_file() {
                    self.files.input.set_text(&files::display_path(&out));
                    self.files.output_custom = false;
                }
                self.from = Some(to);
                self.open_converter(to, from);
            }
            Action::Swap if self.screen == Screen::Convert => {
                let (from, to) = (self.conv.from, self.conv.to);
                if !self.catalog.units[to].kind.can_parse() {
                    let name = self.catalog.units[to].name.clone();
                    return self.notify(format!("{name} can only be a target"), Level::Warn);
                }
                // The other side's value becomes the input, so the same quantity stays on screen.
                let other = self.conv.sides[1].text();
                if !other.is_empty() {
                    self.conv.sides[0].set_text(&other);
                }
                self.from = Some(to);
                self.open_converter(to, from);
                self.animate(200);
            }
            Action::NewConversion => {
                self.from = None;
                self.picker.from_query.clear();
                self.go(Screen::From);
            }
            Action::Refresh if self.screen == Screen::Translate => self.request_translation(true),
            Action::Refresh => {
                if self.screen == Screen::Convert && self.catalog.categories[self.source_unit().category].online {
                    self.ensure_rates(true);
                    self.notify("Fetching live rates…", Level::Info);
                }
            }
            Action::Favorite => {
                let entry = match self.screen {
                    Screen::Convert | Screen::Translate | Screen::Files => Some(Entry::Pair(self.conv.from, self.conv.to)),
                    Screen::Calc => Some(Entry::Unit(self.calc.unit)),
                    _ => self.selected_entry().map(|e| self.fav_entry(e)),
                };
                if let Some(e) = entry {
                    self.toggle_favorite(e);
                }
            }
            Action::Back => self.back(),
            Action::Quit => self.quit = true,
            Action::Help => self.toggle_overlay(Overlay::Help),
            Action::Settings => self.toggle_overlay(Overlay::Settings),
            Action::Swap => {}
        }
    }

    /// Apply the "Copy results as" setting to a computed value.
    pub fn copy_form(&self, plain: String, kind: Kind, symbol: &str) -> String {
        use crate::settings::CopyFormat;
        match self.settings.copy_format {
            CopyFormat::Grouped if kind.is_numeric() => self.grouped(&plain),
            CopyFormat::WithUnit if kind.is_numeric() && !symbol.is_empty() => format!("{plain} {symbol}"),
            _ => plain,
        }
    }

    pub fn copy(&mut self, s: String) {
        if s.is_empty() {
            return self.notify("Nothing to copy", Level::Warn);
        }
        self.clipboard.set(&s);
        let shown: String = s.chars().take(40).collect();
        self.notify(format!("Copied {shown}{}", if s.chars().count() > 40 { "…" } else { "" }), Level::Ok);
    }

    /// Right-click: what to copy under the pointer (an input's selection or text, a result, a table value).
    fn text_at(&self, x: u16, y: u16) -> Option<String> {
        let hit = |r: Rect| crate::components::hit(r, x, y);
        match self.screen {
            Screen::From | Screen::To => hit(self.picker.search.area()).then(|| self.picker.search.copy_text()),
            Screen::Convert => {
                if let Some(side) = (0..2).find(|&i| hit(self.conv.boxes[i])) {
                    let input = &self.conv.sides[side];
                    return Some(match input.selected_text() {
                        Some(sel) => sel,
                        None => self.copy_form(input.text(), self.unit(side).kind, &self.unit(side).symbol),
                    });
                }
                let i = self.conv.table.row_at(x, y)?;
                let u = &self.catalog.units[*self.conv.table_units.get(i)?];
                Some(self.copy_form(self.conv.table_values.get(i)?.clone(), u.kind, &u.symbol))
            }
            Screen::Translate => (0..2).find(|&i| hit(self.tr.boxes[i])).map(|i| self.tr.sides[i].copy_text()),
            Screen::Files if hit(self.files.status_box) => self.result_text(),
            Screen::Files => (0..2).find(|&i| hit(self.files.boxes[i])).map(|i| [&self.files.input, &self.files.output][i].copy_text()),
            Screen::Calc => match (0..self.calc.inputs.len()).find(|&i| hit(self.calc.boxes[i])) {
                Some(i) => Some(self.calc.inputs[i].copy_text()),
                None if hit(self.calc.result_box) => {
                    let s = self.result_text()?;
                    Some(self.copy_form(s, self.calc_def().out, self.calc_def().suffix.trim()))
                }
                None => None,
            },
        }
    }

    // ── Settings ─────────────────────────────────────────────────────────────

    pub fn step_setting(&mut self, i: usize, d: i32) {
        let mouse_before = self.settings.mouse;
        (crate::settings::FIELDS[i].step)(&mut self.settings, d);
        self.skin = skin_for(&self.settings);
        if self.settings.mouse != mouse_before {
            crate::set_mouse(self.settings.mouse);
        }
        self.settings.save();
        if matches!(self.screen, Screen::Convert | Screen::Calc) {
            self.recompute();
        }
    }

    pub fn start_setting_edit(&mut self) {
        let Some(edit) = crate::settings::FIELDS[self.settings_sel].edit else { return };
        let mut input = TextInput::new("type or paste…");
        input.set_text(&edit(&mut self.settings).clone());
        input.select_all();
        self.settings_edit = Some(input);
    }

    pub fn finish_setting_edit(&mut self, save: bool) {
        let (Some(input), Some(edit)) = (self.settings_edit.take(), crate::settings::FIELDS[self.settings_sel].edit) else { return };
        if save {
            *edit(&mut self.settings) = input.text().trim().to_string();
            self.settings.save();
        }
    }

    pub fn toggle_overlay(&mut self, o: Overlay) {
        self.settings_edit = None;
        self.overlay = if self.overlay == o { Overlay::None } else { o };
        self.overlay_at = Instant::now();
        self.help.reset();
        self.animate(200);
    }

    // ── Events ───────────────────────────────────────────────────────────────

    /// Returns true if the screen needs redrawing.
    pub fn on_event(&mut self, ev: Event) -> bool {
        match ev {
            // Windows also reports key releases; act on presses/repeats only.
            Event::Key(k) if k.kind != KeyEventKind::Release => self.on_key(k),
            Event::Mouse(m) => self.on_mouse(m),
            Event::Paste(s) => self.on_paste(&s),
            Event::Resize(..) => {}
            _ => return false,
        }
        true
    }

    fn on_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Char('q') if ctrl => return self.quit = true,
            KeyCode::F(1) => return self.toggle_overlay(Overlay::Help),
            KeyCode::F(2) => return self.toggle_overlay(Overlay::Settings),
            // On the file screen Ctrl+O browses for a file instead.
            KeyCode::Char('o') if ctrl && !(self.screen == Screen::Files && self.overlay == Overlay::None) => return self.toggle_overlay(Overlay::Settings),
            _ => {}
        }
        if self.overlay == Overlay::None && ctrl {
            let action = match k.code {
                KeyCode::Char('f') => Some(Action::Favorite),
                KeyCode::Char('n') => Some(Action::NewConversion),
                KeyCode::Char('s') => Some(Action::Swap),
                KeyCode::Char('r') => Some(Action::Refresh),
                _ => None,
            };
            if let Some(a) = action {
                return self.run_action(a);
            }
        }
        match self.overlay {
            Overlay::Help => crate::screens::overlays::help_key(self, k),
            Overlay::Settings => crate::screens::overlays::settings_key(self, k),
            Overlay::Browse => self.browser_key(k),
            Overlay::Custom => crate::screens::custom::on_key(self, k),
            Overlay::None => match self.screen {
                Screen::From | Screen::To => crate::screens::picker::on_key(self, k),
                Screen::Convert => crate::screens::converter::on_key(self, k),
                Screen::Calc => crate::screens::calculator::on_key(self, k),
                Screen::Translate => crate::screens::translator::on_key(self, k),
                Screen::Files => crate::screens::files::on_key(self, k),
            },
        }
    }

    fn on_mouse(&mut self, m: MouseEvent) {
        self.mouse_pos = (m.column, m.row);
        if m.kind == MouseEventKind::Moved {
            return;
        }
        if self.header[0].clicked(&m) {
            return self.toggle_overlay(Overlay::Help);
        }
        if self.header[1].clicked(&m) {
            return self.toggle_overlay(Overlay::Settings);
        }
        if self.overlay != Overlay::None {
            return crate::screens::overlays::on_mouse(self, m);
        }
        if m.kind == MouseEventKind::Down(MouseButton::Right) {
            if let Some(s) = self.text_at(m.column, m.row) {
                self.copy(s);
            }
            return;
        }
        if let MouseEventKind::Down(_) = m.kind {
            let hit = |r: &Rect| crate::components::hit(*r, m.column, m.row);
            if let Some(&(_, a)) = self.footer.iter().find(|(r, _)| hit(r)) {
                return self.run_action(a);
            }
            if let Some(&(s, _)) = self.crumbs.iter().find(|(_, r)| hit(r)) {
                return self.go(s);
            }
        }
        match self.screen {
            Screen::From | Screen::To => crate::screens::picker::on_mouse(self, m),
            Screen::Convert => crate::screens::converter::on_mouse(self, m),
            Screen::Calc => crate::screens::calculator::on_mouse(self, m),
            Screen::Translate => crate::screens::translator::on_mouse(self, m),
            Screen::Files => crate::screens::files::on_mouse(self, m),
        }
    }

    fn on_paste(&mut self, s: &str) {
        if let Some(input) = &mut self.settings_edit {
            input.paste(s.trim());
            return;
        }
        if self.overlay == Overlay::Browse {
            if let Some(b) = &mut self.browser {
                b.paste(s);
            }
            return;
        }
        if self.overlay == Overlay::Custom {
            return crate::screens::custom::paste(self, s);
        }
        if self.overlay != Overlay::None {
            return;
        }
        match self.screen {
            Screen::Convert => {
                let side = match self.conv.focus {
                    CFocus::Side(i) => i,
                    CFocus::Table => 0,
                };
                self.conv.focus = CFocus::Side(side);
                let ev = self.conv.sides[side].paste(s);
                self.side_event(side, ev);
            }
            Screen::Files => {
                // A dropped/pasted path goes to the focused path field (the input by default).
                if self.files.focus == FFocus::Output {
                    self.files.output.paste(s);
                    self.file_output_changed();
                } else {
                    self.files.focus = FFocus::Input;
                    self.files.input.paste(s);
                    self.file_input_changed();
                }
            }
            Screen::Translate => {
                let side = self.tr.focus;
                match self.tr.sides[side].paste(s) {
                    InputEvent::Changed => self.translator_typed(side),
                    InputEvent::Rejected => self.reject_hint(Kind::Lang(1)),
                    _ => {}
                }
            }
            Screen::Calc => {
                let i = self.calc.focus;
                match self.calc.inputs[i].paste(s) {
                    InputEvent::Rejected => self.reject_hint(self.calc_def().inputs[i].kind),
                    _ => self.recompute_calc(),
                }
            }
            _ => {
                self.picker.focus = PFocus::Search;
                self.picker.search.paste(s);
                self.refresh_picker();
            }
        }
    }

    pub fn reject_hint(&mut self, kind: Kind) {
        self.animate(400);
        let msg = match kind {
            k if !k.can_parse() => "This side is output only".into(),
            Kind::Radix(r) => format!("Only base-{r} digits can be typed here"),
            Kind::Roman => "Only Roman numeral letters (I V X L C D M)".into(),
            Kind::Text(_) => "That character isn't valid in this format".into(),
            Kind::Color(_) => "That character isn't valid in this color format".into(),
            Kind::Date(_) => "Dates look like 2026-11-07 or 2026-11-07 14:30 (UTC, or add +02:00)".into(),
            Kind::Duration(_) => "Durations look like 1:30:00, 45:00 or 2d 4:00".into(),
            Kind::Lang(_) => format!("Text is limited to {} characters", translate::MAX_CHARS),
            Kind::Chem(_) => "Formulas use element symbols, digits and ( ), e.g. Ca(OH)2".into(),
            Kind::Zone(_) => "Times look like 15:00, 3:30 pm, now or 2026-11-07 09:30".into(),
            Kind::Size(..) => "That's too long for a size".into(),
            Kind::Note => "Notes look like A4, C#5, Bb3 or A4 +20¢".into(),
            Kind::Interval => "Intervals look like P5, m3, minor third or octave".into(),
            Kind::Coord(_) => "Coordinates use digits, N S E W, ° ′ ″ and commas".into(),
            Kind::Code(f) => format!("Part codes look like {}", f.placeholder().trim_start_matches("e.g. ")),
            _ => "Only numbers and + − × ÷ ^ ( ) can be typed here".into(),
        };
        self.notify(msg, Level::Warn);
    }
}

/// Sidebar membership: "My Converters" also lists your units that joined other categories.
fn in_category(c: &Catalog, u: usize, cat: usize) -> bool {
    c.units[u].category == cat || (c.categories[cat].id == "custom" && c.units[u].id.starts_with("custom:"))
}

/// Group entries under category headers, categories ordered by their best-ranked entry.
fn group_rows(c: &Catalog, entries: Vec<Entry>) -> Vec<Row> {
    let cat_of = |e: &Entry| match e {
        Entry::Unit(u) | Entry::Pair(u, _) => c.units[*u].category,
    };
    let mut order: Vec<usize> = Vec::new();
    for e in &entries {
        let cat = cat_of(e);
        if !order.contains(&cat) {
            order.push(cat);
        }
    }
    let mut rows = Vec::with_capacity(entries.len() + order.len());
    for cat in order {
        rows.push(Row::Header(cat));
        rows.extend(entries.iter().filter(|e| cat_of(e) == cat).map(|&e| Row::Item(e)));
    }
    rows
}

/// The example value a converter opens with.
fn sample_input(u: &Unit, category: &str) -> String {
    use crate::catalog::{numbers::Frac, time};
    // Dates and timestamps start at "now".
    if category == "datetime"
        && let Ok(v) = convert::from_base(u, time::now())
    {
        return convert::format_value(u.kind, &v, 20, crate::num::Notation::Plain).0;
    }
    // Music starts from concert A and a common tempo, shown in the chosen unit.
    let typical = match category {
        "pitch" => Some(440),
        "tempo" => Some(120),
        _ => None,
    };
    if let Some(b) = typical
        && u.kind != Kind::Note
        && let Ok(v) = convert::from_base(u, crate::num::int(b))
    {
        return convert::format_value(u.kind, &v, 6, crate::num::Notation::Plain).0;
    }
    match u.kind {
        Kind::Radix(r) => numbers::to_radix(&255.into(), r),
        Kind::Roman => "MMXXVI".into(),
        Kind::Text(f) if category == "perms" => f.encode("755"),
        Kind::Text(f) => f.encode("Hello, world"),
        Kind::Color(f) => f.format([1.0, 0.388, 0.278]),
        Kind::Duration(_) => "1:30:00".into(),
        Kind::Fraction(Frac::Mixed) => "2 3/4".into(),
        Kind::Fraction(_) => "3/8".into(),
        Kind::Chem(_) => "C6H12O6".into(),
        Kind::Zone(i) => crate::catalog::zones::now_text(i),
        Kind::Size(t, c) => crate::catalog::sizes::example(t, c),
        Kind::Note => "A4".into(),
        Kind::Interval => "perfect fifth".into(),
        Kind::Coord(f) => f.format((50.4501, 30.5234)),
        Kind::Code(f) => f.example().into(),
        _ => "1".into(),
    }
}
