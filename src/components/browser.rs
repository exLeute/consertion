//! File browser popup: folders first, files filtered to the extensions that fit, a filter box
//! (typing a path jumps there), parent/home shortcuts, hidden files on request, and a
//! "Save in this folder" row when choosing where to write.

use super::{fit, InputEvent, ListAction, ListView, TextInput};
use crate::clipboard::Clipboard;
use crate::theme::Skin;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;
use std::path::{Path, PathBuf};

/// Folders this big are cut off (a filter narrows them down).
const MAX_ENTRIES: usize = 20_000;

struct Entry {
    name: String,
    dir: bool,
    size: u64,
}

#[derive(Clone, Copy, PartialEq)]
enum Row {
    SaveHere,
    Parent,
    Item(usize),
}

pub enum Picked {
    None,
    /// A file (or, when saving, a folder to save into).
    Path(PathBuf),
    Close,
}

pub struct FileBrowser {
    pub dir: PathBuf,
    entries: Vec<Entry>,
    rows: Vec<Row>,
    pub list: ListView,
    pub filter: TextInput,
    /// Extensions to show (lowercase, no dot); empty shows every file.
    exts: Vec<&'static str>,
    pub all_files: bool,
    pub hidden: bool,
    /// Choosing an output location: folders can be picked.
    save: bool,
    pub title: String,
    error: Option<String>,
    truncated: bool,
}

impl FileBrowser {
    pub fn open(start: &Path, exts: Vec<&'static str>, save: bool, hidden: bool, title: &str) -> Self {
        let mut filter = TextInput::new("type to filter, or a path");
        filter.prefix = "❯ ";
        let mut b = Self {
            dir: PathBuf::new(),
            entries: Vec::new(),
            rows: Vec::new(),
            list: ListView::default(),
            filter,
            exts,
            all_files: false,
            hidden,
            save,
            title: title.into(),
            error: None,
            truncated: false,
        };
        let start = if start.is_dir() { start.to_path_buf() } else { start.parent().map(Path::to_path_buf).unwrap_or_default() };
        let fallback = std::env::current_dir().ok().or_else(dirs::home_dir).unwrap_or_else(|| PathBuf::from("/"));
        if !b.cd(&start) {
            b.cd(&fallback);
        }
        // Start on the file that was already chosen.
        if let Some(name) = start.file_name().filter(|_| !start.is_dir()) {
            b.select_name(&name.to_string_lossy());
        }
        b
    }

    /// Enter a folder. On failure (permissions…) stay put and say why.
    fn cd(&mut self, dir: &Path) -> bool {
        let rd = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(e) => {
                self.error = Some(format!("Can't open {}: {}", crate::files::display_path(dir), e.kind()));
                return false;
            }
        };
        let mut entries: Vec<Entry> = Vec::new();
        self.truncated = false;
        for e in rd.flatten() {
            if entries.len() >= MAX_ENTRIES {
                self.truncated = true;
                break;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            // Follow symlinks, so a linked folder opens like a folder.
            let meta = std::fs::metadata(e.path()).ok();
            entries.push(Entry { name, dir: meta.as_ref().is_some_and(|m| m.is_dir()), size: meta.map_or(0, |m| m.len()) });
        }
        entries.sort_by(|a, b| b.dir.cmp(&a.dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        self.dir = dir.to_path_buf();
        self.entries = entries;
        self.error = None;
        self.filter.set_text("");
        self.refresh();
        self.list.reset();
        true
    }

    fn refresh(&mut self) {
        let q = self.filter.text().to_lowercase();
        let wants = |e: &Entry| {
            let ext = Path::new(&e.name).extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
            e.dir || self.all_files || self.exts.is_empty() || self.exts.contains(&ext.as_str())
        };
        let mut rows = Vec::new();
        if q.is_empty() {
            if self.save {
                rows.push(Row::SaveHere);
            }
            if self.dir.parent().is_some() {
                rows.push(Row::Parent);
            }
        }
        rows.extend(
            self.entries
                .iter()
                .enumerate()
                .filter(|(_, e)| (self.hidden || !e.name.starts_with('.')) && wants(e) && (q.is_empty() || e.name.to_lowercase().contains(&q)))
                .map(|(i, _)| Row::Item(i)),
        );
        self.rows = rows;
        self.list.select(0);
    }

    fn select_name(&mut self, name: &str) {
        if let Some(i) = self.rows.iter().position(|r| matches!(r, Row::Item(i) if self.entries[*i].name == name)) {
            self.list.select(i);
        }
    }

    fn up(&mut self) {
        if let Some(parent) = self.dir.parent().map(Path::to_path_buf) {
            let from = self.dir.file_name().map(|n| n.to_string_lossy().into_owned());
            if self.cd(&parent)
                && let Some(n) = from
            {
                self.select_name(&n);
            }
        }
    }

    fn activate(&mut self) -> Picked {
        // A typed path wins: jump to a folder, or pick a file.
        let typed = self.filter.text();
        if typed.contains('/') || typed.contains(std::path::MAIN_SEPARATOR) || typed.starts_with('~') {
            let p = crate::files::parse_path(&typed);
            if p.is_dir() {
                self.cd(&p);
                return Picked::None;
            }
            if p.is_file() || self.save {
                return Picked::Path(p);
            }
            self.error = Some(format!("Not found: {}", crate::files::display_path(&p)));
            return Picked::None;
        }
        match self.rows.get(self.list.selected).copied() {
            Some(Row::SaveHere) => Picked::Path(self.dir.clone()),
            Some(Row::Parent) => {
                self.up();
                Picked::None
            }
            Some(Row::Item(i)) if self.entries[i].dir => {
                let d = self.dir.join(&self.entries[i].name);
                self.cd(&d);
                Picked::None
            }
            Some(Row::Item(i)) => Picked::Path(self.dir.join(&self.entries[i].name)),
            None => Picked::None,
        }
    }

    pub fn handle_key(&mut self, k: KeyEvent, clip: &mut Clipboard) -> Picked {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let empty = self.filter.is_empty();
        match k.code {
            KeyCode::Esc if !empty => {
                self.filter.set_text("");
                self.refresh();
            }
            KeyCode::Esc => return Picked::Close,
            KeyCode::Enter => return self.activate(),
            KeyCode::Backspace | KeyCode::Left if empty => self.up(),
            KeyCode::Right if empty => {
                if matches!(self.rows.get(self.list.selected), Some(Row::Item(i)) if self.entries[*i].dir) {
                    return self.activate();
                }
            }
            KeyCode::Char('~') if empty => {
                if let Some(h) = dirs::home_dir() {
                    self.cd(&h);
                }
            }
            KeyCode::Char('h') if ctrl => {
                self.hidden = !self.hidden;
                self.refresh();
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.all_files = !self.all_files;
                self.refresh();
            }
            KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
                self.list.handle_key(k);
            }
            KeyCode::Home | KeyCode::End if empty => {
                self.list.handle_key(k);
            }
            _ => {
                if self.filter.handle_key(k, clip) == InputEvent::Changed {
                    self.refresh();
                }
            }
        }
        Picked::None
    }

    pub fn paste(&mut self, s: &str) {
        self.filter.paste(s);
        self.refresh();
    }

    pub fn handle_mouse(&mut self, m: MouseEvent) -> Picked {
        if self.filter.handle_mouse(m) {
            return Picked::None;
        }
        match self.list.handle_mouse(m) {
            ListAction::Activate => self.activate(),
            _ => Picked::None,
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, skin: &Skin) {
        f.render_widget(Clear, area);
        let block = skin.block(self.title.clone(), true);
        let inner = block.inner(area);
        f.render_widget(block, area);
        if inner.height < 4 {
            return;
        }
        let path = Rect { x: inner.x + 1, width: inner.width.saturating_sub(2), height: 1, ..inner };
        let shown = crate::files::display_path(&self.dir);
        let tail: String = {
            // Keep the end of long paths, where the current folder is.
            let w = path.width as usize;
            let n = shown.chars().count();
            if n > w { format!("…{}", shown.chars().skip(n + 1 - w).collect::<String>()) } else { shown }
        };
        f.buffer_mut().set_stringn(path.x, path.y, &tail, path.width as usize, skin.accent());
        let filter = Rect { y: inner.y + 1, ..path };
        self.filter.render(f, filter, skin, true, Line::default());

        let hint_h = 1;
        let list = Rect { y: inner.y + 2, height: inner.height.saturating_sub(2 + hint_h), ..inner };
        let (rows, entries) = (&self.rows, &self.entries);
        let what = if self.all_files || self.exts.is_empty() { "all files" } else { "matching files" };
        let items = rows.iter().filter(|r| matches!(r, Row::Item(_))).count();
        let title = format!("{items} items · {what}{}", if self.truncated { " · folder cut off, type to filter" } else { "" });
        let empty = self.error.clone().unwrap_or_else(|| "Nothing here — Tab shows all files".into());
        let muted = skin.muted();
        let (accent, text) = (skin.accent(), skin.text());
        self.list.render(f, list, skin, true, Line::from(title), rows.len(), &empty, |i, w| match rows[i] {
            Row::SaveHere => Line::from(Span::styled("✓ Save in this folder", accent)),
            Row::Parent => Line::from(Span::styled("↑ ..", muted)),
            Row::Item(j) => {
                let e = &entries[j];
                if e.dir {
                    Line::from(vec![Span::styled("▸ ", accent), Span::styled(fit(&format!("{}/", e.name), w.saturating_sub(2)), text)])
                } else {
                    let size = crate::files::human_size(e.size);
                    let name_w = w.saturating_sub(size.len() + 3);
                    Line::from(vec![Span::raw("  "), Span::styled(fit(&e.name, name_w), text), Span::styled(format!(" {size:>w$}", w = size.len()), muted)])
                }
            }
        });
        if let Some(e) = &self.error
            && !self.rows.is_empty()
        {
            let r = Rect { y: list.y + list.height.saturating_sub(1), height: 1, x: list.x + 2, width: list.width.saturating_sub(4) };
            f.render_widget(Paragraph::new(Span::styled(e.clone(), ratatui::style::Style::new().fg(skin.t.err))), r);
        }
        let hint = Rect { y: inner.y + inner.height - 1, ..path };
        let keys = if self.save { "⏎ open/choose · ⌫ up · ~ home · Tab all files · ^H hidden · Esc close" } else { "⏎ open · ⌫ up · ~ home · Tab all files · ^H hidden · Esc close" };
        f.buffer_mut().set_stringn(hint.x, hint.y, keys, hint.width as usize, skin.muted());
    }
}
