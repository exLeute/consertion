//! Color themes. Backgrounds are never set (except selections), so the terminal's own
//! background — including transparency — always shows through.

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::text::{Line, Span};

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub name: &'static str,
    pub accent: Color,
    pub accent2: Color,
    pub text: Color,
    pub muted: Color,
    pub border: Color,
    pub sel_bg: Color,
    pub sel_fg: Color,
    pub ok: Color,
    pub warn: Color,
    pub err: Color,
    /// Text drawn on an accent-colored background (the current breadcrumb step).
    pub on_accent: Color,
}

const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// A truecolor theme: accent, accent2, text, muted, border, sel_bg, sel_fg, ok, warn, err, on_accent.
const fn hex(name: &'static str, c: [u32; 11]) -> Theme {
    Theme {
        name, accent: rgb(c[0]), accent2: rgb(c[1]), text: rgb(c[2]), muted: rgb(c[3]), border: rgb(c[4]),
        sel_bg: rgb(c[5]), sel_fg: rgb(c[6]), ok: rgb(c[7]), warn: rgb(c[8]), err: rgb(c[9]), on_accent: rgb(c[10]),
    }
}

pub const THEMES: &[Theme] = &[
    // Uses the terminal's own 16-color palette, so it matches whatever the terminal theme is.
    Theme {
        name: "Terminal", accent: Color::Cyan, accent2: Color::Magenta, text: Color::Reset,
        muted: Color::DarkGray, border: Color::Gray, sel_bg: Color::Cyan, sel_fg: Color::Black,
        ok: Color::Green, warn: Color::Yellow, err: Color::Red, on_accent: Color::Black,
    },
    hex("Lavender", [0xcba6f7, 0x89b4fa, 0xcdd6f4, 0x7f849c, 0x585b70, 0x45475a, 0xf5e0dc, 0xa6e3a1, 0xf9e2af, 0xf38ba8, 0x1e1e2e]),
    hex("Ocean", [0x4fc3f7, 0x80cbc4, 0xe0f2f1, 0x6c8a99, 0x37505c, 0x1e4a66, 0xe1f5fe, 0x81c784, 0xffd54f, 0xef5350, 0x0b1f2a]),
    hex("Forest", [0xa3be8c, 0xebcb8b, 0xe5e9d8, 0x7d8a6e, 0x4c5a42, 0x3b4a32, 0xf0f4e0, 0xa3be8c, 0xebcb8b, 0xbf616a, 0x1d2618]),
    hex("Sunset", [0xff9e64, 0xf7768e, 0xf2e5d5, 0x9a8478, 0x5c4a44, 0x5a3a3a, 0xffe8d6, 0x9ece6a, 0xe0af68, 0xf7768e, 0x2a1a14]),
    hex("Frost", [0x88c0d0, 0xb48ead, 0xeceff4, 0x7b88a1, 0x4c566a, 0x434c5e, 0xeceff4, 0xa3be8c, 0xebcb8b, 0xbf616a, 0x2e3440]),
    hex("Gruvbox", [0xfabd2f, 0x83a598, 0xebdbb2, 0x928374, 0x504945, 0x504945, 0xfbf1c7, 0xb8bb26, 0xfe8019, 0xfb4934, 0x282828]),
    hex("Dracula", [0xbd93f9, 0xff79c6, 0xf8f8f2, 0x6272a4, 0x44475a, 0x44475a, 0xf8f8f2, 0x50fa7b, 0xf1fa8c, 0xff5555, 0x282a36]),
    hex("Tokyo Night", [0x7aa2f7, 0xbb9af7, 0xc0caf5, 0x565f89, 0x3b4261, 0x283457, 0xc0caf5, 0x9ece6a, 0xe0af68, 0xf7768e, 0x1a1b26]),
    hex("One Dark", [0x61afef, 0xc678dd, 0xabb2bf, 0x5c6370, 0x3e4451, 0x3e4451, 0xd7dae0, 0x98c379, 0xe5c07b, 0xe06c75, 0x282c34]),
    hex("Monokai", [0xa6e22e, 0x66d9ef, 0xf8f8f2, 0x75715e, 0x49483e, 0x49483e, 0xf8f8f2, 0xa6e22e, 0xe6db74, 0xf92672, 0x272822]),
    hex("Rosé Pine", [0xebbcba, 0xc4a7e7, 0xe0def4, 0x6e6a86, 0x403d52, 0x403d52, 0xe0def4, 0x9ccfd8, 0xf6c177, 0xeb6f92, 0x191724]),
    hex("Kanagawa", [0x7e9cd8, 0x957fb8, 0xdcd7ba, 0x727169, 0x54546d, 0x2d4f67, 0xdcd7ba, 0x98bb6c, 0xe6c384, 0xff5d62, 0x1f1f28]),
    hex("Everforest", [0xa7c080, 0x83c092, 0xd3c6aa, 0x859289, 0x4f585e, 0x475258, 0xd3c6aa, 0xa7c080, 0xdbbc7f, 0xe67e80, 0x2d353b]),
    hex("Solarized Dark", [0x268bd2, 0x2aa198, 0x93a1a1, 0x657b83, 0x586e75, 0x073642, 0xeee8d5, 0x859900, 0xb58900, 0xdc322f, 0xfdf6e3]),
    hex("Synthwave", [0xff2a6d, 0x05d9e8, 0xd1f7ff, 0x7a6f9b, 0x3d2c5c, 0x3d2c5c, 0xffffff, 0x72f1b8, 0xfede5d, 0xfe4450, 0x1a1028]),
    hex("Matrix", [0x39ff14, 0x00cc44, 0x9cff9c, 0x2e8b57, 0x1f5f3a, 0x0f3d1f, 0xc8ffc8, 0x39ff14, 0xccff00, 0xff5555, 0x001a00]),
    hex("Amber", [0xffb000, 0xffcc66, 0xffd27f, 0xa0701a, 0x6b4a10, 0x4a3206, 0xffe0a0, 0xffcc66, 0xffb000, 0xff6040, 0x1a1000]),
    // For light terminal backgrounds.
    hex("Daylight", [0x0969da, 0x8250df, 0x24292f, 0x6e7781, 0xafb8c1, 0xddf4ff, 0x0a3069, 0x1a7f37, 0x9a6700, 0xcf222e, 0xffffff]),
    hex("Latte", [0x8839ef, 0x1e66f5, 0x4c4f69, 0x8c8fa1, 0xbcc0cc, 0xccd0da, 0x4c4f69, 0x40a02b, 0xdf8e1d, 0xd20f39, 0xeff1f5]),
    hex("Solarized Light", [0x268bd2, 0xd33682, 0x586e75, 0x93a1a1, 0x93a1a1, 0xeee8d5, 0x073642, 0x859900, 0xb58900, 0xdc322f, 0xfdf6e3]),
    Theme {
        name: "High Contrast", accent: Color::Yellow, accent2: Color::LightCyan, text: Color::White,
        muted: Color::Gray, border: Color::White, sel_bg: Color::Yellow, sel_fg: Color::Black,
        ok: Color::LightGreen, warn: Color::LightYellow, err: Color::LightRed, on_accent: Color::Black,
    },
    Theme {
        name: "Mono", accent: Color::White, accent2: Color::Gray, text: Color::Reset,
        muted: Color::DarkGray, border: Color::Gray, sel_bg: Color::White, sel_fg: Color::Black,
        ok: Color::White, warn: Color::White, err: Color::White, on_accent: Color::Black,
    },
];

pub fn by_name(name: &str) -> Theme {
    *THEMES.iter().find(|t| t.name == name).unwrap_or(&THEMES[0])
}

/// Everything a component needs to draw itself consistently.
#[derive(Clone, Copy)]
pub struct Skin {
    pub t: Theme,
    pub border: BorderType,
    pub animations: bool,
}

impl Skin {
    pub fn text(&self) -> Style {
        Style::new().fg(self.t.text)
    }
    pub fn muted(&self) -> Style {
        Style::new().fg(self.t.muted)
    }
    pub fn accent(&self) -> Style {
        Style::new().fg(self.t.accent)
    }
    pub fn selected(&self) -> Style {
        Style::new().bg(self.t.sel_bg).fg(self.t.sel_fg)
    }
    /// Filled chip: accent background.
    pub fn chip(&self) -> Style {
        Style::new().bg(self.t.accent).fg(self.t.on_accent).add_modifier(Modifier::BOLD)
    }
    pub fn key(&self) -> Style {
        Style::new().fg(self.t.accent).add_modifier(Modifier::BOLD)
    }

    /// The standard panel: themed border, focus shown by color *and* a bold "●" title marker.
    pub fn block<'a>(&self, title: impl Into<Line<'a>>, focused: bool) -> Block<'a> {
        let mut spans = vec![Span::raw(" ")];
        if focused {
            spans.push(Span::raw("● "));
        }
        spans.extend(title.into().spans);
        spans.push(Span::raw(" "));
        let (border, style) = if focused {
            (self.t.accent, Style::new().fg(self.t.accent).add_modifier(Modifier::BOLD))
        } else {
            (self.t.border, self.muted())
        };
        Block::new()
            .borders(Borders::ALL)
            .border_type(self.border)
            .border_style(Style::new().fg(border))
            .title(Line::from(spans).style(style))
    }
}
