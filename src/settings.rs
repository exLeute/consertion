//! User settings, persisted as JSON in the platform config dir.
//!
//! Adding a setting: add a field (with a default) to `Settings`, then one `Field` entry to
//! `FIELDS` — the settings screen renders and edits it automatically.

use crate::num::Notation;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Grouping {
    Off,
    Comma,
    Space,
    Dot,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum CopyFormat {
    /// "1234567.5" — pastes cleanly anywhere.
    Plain,
    /// "1,234,567.5" — as shown, with digit grouping.
    Grouped,
    /// "1234567.5 km"
    WithUnit,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Provider {
    /// Free, no key needed.
    MyMemory,
    DeepL,
    LibreTranslate,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Borders {
    Rounded,
    Plain,
    Double,
    Thick,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub animations: bool,
    pub precision: usize,
    pub notation: Notation,
    pub grouping: Grouping,
    pub show_table: bool,
    pub rates_ttl_min: u64,
    pub mouse: bool,
    pub borders: Borders,
    pub copy_format: CopyFormat,
    /// Reopen the last conversion on launch instead of the search.
    pub resume: bool,
    /// The "1 A = x B · rule" line under the converter.
    pub show_info: bool,
    pub recent_limit: usize,
    pub provider: Provider,
    pub deepl_key: String,
    pub libre_url: String,
    pub libre_key: String,
    /// Optional; MyMemory allows 50,000 instead of 5,000 characters a day with an email.
    pub mymemory_email: String,
    /// Translate after a short pause in typing (else only on Ctrl+R).
    pub translate_live: bool,
    pub show_hidden: bool,
    /// Last used file-conversion options: option key → choice label.
    pub file_opts: std::collections::BTreeMap<String, String>,
    /// Recently used (from, to) unit ids, newest first. Calculators use (id, id).
    pub recent: Vec<(String, String)>,
    /// Favorite unit ids ("length:meter") and pairs ("length:meter>length:foot").
    pub favorites: Vec<String>,
    /// Your own units ("step = 0.75 m").
    pub custom: Vec<crate::catalog::custom::CustomUnit>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "Terminal".into(),
            animations: true,
            precision: 12,
            notation: Notation::Auto,
            grouping: Grouping::Comma,
            show_table: true,
            rates_ttl_min: 60,
            mouse: true,
            borders: Borders::Rounded,
            copy_format: CopyFormat::Plain,
            resume: false,
            show_info: true,
            recent_limit: 20,
            provider: Provider::MyMemory,
            deepl_key: String::new(),
            libre_url: "https://libretranslate.com".into(),
            libre_key: String::new(),
            mymemory_email: String::new(),
            translate_live: true,
            show_hidden: false,
            file_opts: Default::default(),
            recent: Vec::new(),
            favorites: Vec::new(),
            custom: Vec::new(),
        }
    }
}

pub fn pair_key(from: &str, to: &str) -> String {
    format!("{from}>{to}")
}

impl Settings {
    fn path() -> Option<PathBuf> {
        Some(dirs::config_dir()?.join("consertion").join("settings.json"))
    }

    /// Missing or broken files fall back to defaults; unknown fields are ignored.
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Write-then-rename, so a crash never leaves a truncated settings file. Owner-only on Unix
    /// (it can hold API keys).
    pub fn save(&self) {
        if let Some(p) = Self::path() {
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            let tmp = p.with_extension("json.tmp");
            if std::fs::write(&tmp, serde_json::to_vec_pretty(self).unwrap_or_default()).is_ok() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
                }
                let _ = std::fs::rename(tmp, p);
            }
        }
    }

    pub fn push_recent(&mut self, from: &str, to: &str) {
        let pair = (from.to_string(), to.to_string());
        self.recent.retain(|p| *p != pair);
        self.recent.insert(0, pair);
        self.recent.truncate(self.recent_limit);
    }

    pub fn is_favorite(&self, key: &str) -> bool {
        self.favorites.iter().any(|f| f == key)
    }

    /// Returns the new state.
    pub fn toggle_favorite(&mut self, key: &str) -> bool {
        if self.is_favorite(key) {
            self.favorites.retain(|f| f != key);
            false
        } else {
            self.favorites.insert(0, key.to_string());
            true
        }
    }

    pub fn grouping_chars(&self) -> Option<(char, char)> {
        match self.grouping {
            Grouping::Off => None,
            Grouping::Comma => Some((',', '.')),
            Grouping::Space => Some(('\u{202f}', '.')),
            Grouping::Dot => Some(('.', ',')),
        }
    }
}

fn cycle<T: PartialEq + Copy>(list: &[T], cur: T, step: i32) -> T {
    let i = list.iter().position(|x| *x == cur).unwrap_or(0) as i32;
    list[(i + step).rem_euclid(list.len() as i32) as usize]
}

fn on_off(b: bool) -> String {
    if b { "On".into() } else { "Off".into() }
}

fn secret(s: &str) -> String {
    match s.chars().count() {
        0 => "Not set".into(),
        n if n <= 4 => "•".repeat(n),
        n => format!("••••{}", s.chars().skip(n - 4).collect::<String>()),
    }
}

fn or_none(s: &str) -> String {
    if s.is_empty() { "Not set".into() } else { s.to_string() }
}

pub struct Field {
    /// Section heading shown above the first field of each group.
    pub group: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub value: fn(&Settings) -> String,
    /// Change the value by one step (-1 / +1).
    pub step: fn(&mut Settings, i32),
    /// Free-text fields are edited in place (Enter) instead of stepped.
    pub edit: Option<fn(&mut Settings) -> &mut String>,
}

impl Field {
    const fn new(group: &'static str, label: &'static str, help: &'static str, value: fn(&Settings) -> String, step: fn(&mut Settings, i32)) -> Self {
        Self { group, label, help, value, step, edit: None }
    }

    const fn text(group: &'static str, label: &'static str, help: &'static str, value: fn(&Settings) -> String, edit: fn(&mut Settings) -> &mut String) -> Self {
        Self { group, label, help, value, step: |_, _| {}, edit: Some(edit) }
    }
}

pub const FIELDS: &[Field] = &[
    Field::new(
        "Look",
        "Color theme",
        "\"Terminal\" follows your terminal's palette. Daylight, Latte and Solarized Light suit light backgrounds.",
        |s| s.theme.clone(),
        |s, d| {
            let names: Vec<&str> = crate::theme::THEMES.iter().map(|t| t.name).collect();
            s.theme = cycle(&names, s.theme.as_str(), d).to_string();
        },
    ),
    Field::new(
        "Look",
        "Borders",
        "Panel border style.",
        |s| format!("{:?}", s.borders),
        |s, d| s.borders = cycle(&[Borders::Rounded, Borders::Plain, Borders::Double, Borders::Thick], s.borders, d),
    ),
    Field::new(
        "Look",
        "Animations",
        "Screen slides, popups, notifications and value highlights.",
        |s| on_off(s.animations),
        |s, _| s.animations = !s.animations,
    ),
    Field::new(
        "Look",
        "Mouse support",
        "Turn off to use your terminal's native text selection.",
        |s| on_off(s.mouse),
        |s, _| s.mouse = !s.mouse,
    ),
    Field::new(
        "Numbers",
        "Significant digits",
        "Math is exact; this only sets how many digits are shown (and copied). Up to 50.",
        |s| s.precision.to_string(),
        |s, d| {
            let steps = [4, 6, 8, 10, 12, 15, 20, 30, 40, 50];
            let i = steps.iter().position(|&p| p >= s.precision).unwrap_or(4) as i32;
            s.precision = steps[(i + d).clamp(0, steps.len() as i32 - 1) as usize];
        },
    ),
    Field::new(
        "Numbers",
        "Number notation",
        "Auto switches to 1.5e21 only for very large or tiny values. Plain always writes every digit.",
        |s| match s.notation {
            Notation::Auto => "Auto".into(),
            Notation::Scientific => "Scientific (1.2345e4)".into(),
            Notation::Engineering => "Engineering (12.345e3)".into(),
            Notation::Plain => "Plain (12345)".into(),
        },
        |s, d| s.notation = cycle(&[Notation::Auto, Notation::Scientific, Notation::Engineering, Notation::Plain], s.notation, d),
    ),
    Field::new(
        "Numbers",
        "Digit grouping",
        "How large numbers are shown under the values and in the table. Inputs stay plain.",
        |s| match s.grouping {
            Grouping::Off => "Off (1234567.5)".into(),
            Grouping::Comma => "1,234,567.5".into(),
            Grouping::Space => "1 234 567.5".into(),
            Grouping::Dot => "1.234.567,5".into(),
        },
        |s, d| s.grouping = cycle(&[Grouping::Off, Grouping::Comma, Grouping::Space, Grouping::Dot], s.grouping, d),
    ),
    Field::new(
        "Converter",
        "All-units table",
        "Show the value in every unit of the category under the converter.",
        |s| on_off(s.show_table),
        |s, _| s.show_table = !s.show_table,
    ),
    Field::new(
        "Converter",
        "Rates & formula line",
        "Show \"1 A = x B · 1 B = y A · B = A × k\" under the converter.",
        |s| on_off(s.show_info),
        |s, _| s.show_info = !s.show_info,
    ),
    Field::new(
        "Converter",
        "Copy results as",
        "What Enter, Ctrl+C and right-click put on the clipboard for a computed value.",
        |s| match s.copy_format {
            CopyFormat::Plain => "Plain (1234567.5)".into(),
            CopyFormat::Grouped => "As shown (1,234,567.5)".into(),
            CopyFormat::WithUnit => "With unit (1234567.5 km)".into(),
        },
        |s, d| s.copy_format = cycle(&[CopyFormat::Plain, CopyFormat::Grouped, CopyFormat::WithUnit], s.copy_format, d),
    ),
    Field::new(
        "Converter",
        "On launch",
        "Start at the search, or reopen the last conversion or calculator.",
        |s| if s.resume { "Reopen last conversion".into() } else { "Search".into() },
        |s, _| s.resume = !s.resume,
    ),
    Field::new(
        "Converter",
        "Recent history",
        "How many recent conversions to keep. Off clears the history and stops recording it.",
        |s| match s.recent_limit {
            0 => "Off (cleared)".into(),
            n => format!("{n} items"),
        },
        |s, d| {
            s.recent_limit = cycle(&[0, 10, 20, 50, 100], s.recent_limit, d);
            s.recent.truncate(s.recent_limit);
        },
    ),
    Field::new(
        "Converter",
        "Refresh rates after",
        "Exchange rates are fetched only when converting currencies, and reused for this long.",
        |s| match s.rates_ttl_min {
            m if m < 60 => format!("{m} min"),
            m => format!("{} h", m / 60),
        },
        |s, d| s.rates_ttl_min = cycle(&[15, 60, 360, 1440], s.rates_ttl_min, d),
    ),
    Field::new(
        "Translation",
        "Translator",
        "MyMemory is free with no key. DeepL and LibreTranslate need your own API key below. Text is sent online only when translating.",
        |s| match s.provider {
            Provider::MyMemory => "MyMemory (free)".into(),
            Provider::DeepL => "DeepL (your key)".into(),
            Provider::LibreTranslate => "LibreTranslate".into(),
        },
        |s, d| s.provider = cycle(&[Provider::MyMemory, Provider::DeepL, Provider::LibreTranslate], s.provider, d),
    ),
    Field::new(
        "Translation",
        "Translate as you type",
        "Off: translate only when you press Ctrl+R (saves your daily quota).",
        |s| on_off(s.translate_live),
        |s, _| s.translate_live = !s.translate_live,
    ),
    Field::text(
        "Translation",
        "DeepL API key",
        "From deepl.com/your-account/keys. Free keys end in \":fx\". Stored in the settings file (owner-only).",
        |s| secret(&s.deepl_key),
        |s| &mut s.deepl_key,
    ),
    Field::text(
        "Translation",
        "LibreTranslate server",
        "Your own instance (e.g. http://localhost:5000) or a public one. libretranslate.com needs a key.",
        |s| or_none(&s.libre_url),
        |s| &mut s.libre_url,
    ),
    Field::text(
        "Translation",
        "LibreTranslate API key",
        "Leave empty for servers that don't need one.",
        |s| secret(&s.libre_key),
        |s| &mut s.libre_key,
    ),
    Field::text(
        "Translation",
        "MyMemory email",
        "Optional. With an email MyMemory allows 50,000 characters a day instead of 5,000.",
        |s| or_none(&s.mymemory_email),
        |s| &mut s.mymemory_email,
    ),
    Field::new(
        "Files",
        "Hidden files in browser",
        "Show dotfiles in the file browser (also Ctrl+H while browsing).",
        |s| on_off(s.show_hidden),
        |s, _| s.show_hidden = !s.show_hidden,
    ),
];
