//! Reusable widgets. Each one owns its interaction state, remembers where it was drawn
//! (for mouse hit-testing), and draws itself with a `Skin`.

pub mod browser;
pub mod button;
pub mod choice;
pub mod input;
pub mod list;
pub mod toast;

pub use button::Button;
pub use choice::Choice;
pub use input::{InputEvent, TextInput};
pub use list::{ListAction, ListView};
pub use toast::Toast;

use ratatui::layout::Rect;
use std::time::{Duration, Instant};

pub fn hit(r: Rect, x: u16, y: u16) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

/// 0.0 → 1.0 over `ms`, eased out (fast start, soft landing). 1.0 when animations are off.
pub fn ease_out(start: Instant, ms: u64, enabled: bool) -> f32 {
    if !enabled {
        return 1.0;
    }
    let t = (start.elapsed().as_secs_f32() / Duration::from_millis(ms).as_secs_f32()).min(1.0);
    1.0 - (1.0 - t).powi(3)
}

pub fn spinner(start: Instant) -> &'static str {
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    FRAMES[(start.elapsed().as_millis() / 80) as usize % FRAMES.len()]
}

/// Pad or truncate (with "…") to an exact display width.
pub fn fit(s: &str, width: usize) -> String {
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
    if s.width() <= width {
        return format!("{s}{}", " ".repeat(width - s.width()));
    }
    let mut out = String::new();
    let mut w = 0;
    for c in s.chars() {
        let cw = c.width().unwrap_or(0);
        if w + cw + 1 > width {
            break;
        }
        out.push(c);
        w += cw;
    }
    if width > 0 {
        out.push('…');
        w += 1;
    }
    out + &" ".repeat(width.saturating_sub(w))
}

/// Word-wrap to `width` display columns; words longer than a line are split.
pub fn wrap(s: &str, width: usize) -> Vec<String> {
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
    let width = width.max(1);
    let mut lines = vec![String::new()];
    for word in s.split(' ') {
        let cur = lines.last_mut().unwrap();
        let sep = usize::from(!cur.is_empty());
        if cur.width() + sep + word.width() <= width {
            if sep == 1 {
                cur.push(' ');
            }
            cur.push_str(word);
            continue;
        }
        if !cur.is_empty() {
            lines.push(String::new());
        }
        for c in word.chars() {
            let cur = lines.last_mut().unwrap();
            if cur.width() + c.width().unwrap_or(0) > width {
                lines.push(String::new());
            }
            lines.last_mut().unwrap().push(c);
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    #[test]
    fn wraps_words() {
        assert_eq!(super::wrap("one two three", 7), ["one two", "three"]);
        assert_eq!(super::wrap("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        assert_eq!(super::wrap("", 4), [""]);
    }
}
