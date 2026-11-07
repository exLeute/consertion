//! Color formats. Everything goes through sRGB (0–1 floats); CIE spaces use the D65 white point.

use super::{Builder, Kind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorFmt {
    Hex, Rgb, RgbFloat, Hsl, Hsv, Hwb, Cmyk, Xyz, Lab, Lch, Oklab, Oklch, Integer, Ansi256, Name,
}

pub type Rgb = [f64; 3];

const FORMATS: &[(ColorFmt, &str, &str, &str)] = &[
    (ColorFmt::Hex, "HEX color", "#hex", "hexadecimal web css html"),
    (ColorFmt::Rgb, "RGB", "rgb()", "red green blue 255"),
    (ColorFmt::RgbFloat, "RGB float (0–1)", "vec3()", "normalized shader glsl"),
    (ColorFmt::Hsl, "HSL", "hsl()", "hue saturation lightness"),
    (ColorFmt::Hsv, "HSV / HSB", "hsv()", "hue saturation value brightness"),
    (ColorFmt::Hwb, "HWB", "hwb()", "hue whiteness blackness"),
    (ColorFmt::Cmyk, "CMYK", "cmyk()", "print cyan magenta yellow key"),
    (ColorFmt::Xyz, "CIE XYZ (D65)", "xyz()", "cie tristimulus"),
    (ColorFmt::Lab, "CIELAB (D65)", "lab()", "cie l*a*b"),
    (ColorFmt::Lch, "CIE LCh", "lch()", "cie lch"),
    (ColorFmt::Oklab, "OKLab", "oklab()", "perceptual"),
    (ColorFmt::Oklch, "OKLCH", "oklch()", "perceptual css"),
    (ColorFmt::Integer, "Color integer", "0xRRGGBB", "decimal int packed"),
    (ColorFmt::Ansi256, "ANSI 256 color", "ansi", "xterm terminal 256"),
    (ColorFmt::Name, "CSS color name", "name", "named web"),
];

pub fn register(b: &mut Builder) {
    b.category("color", "Color", false);
    for &(f, name, sym, kw) in FORMATS {
        b.add(name, sym, Kind::Color(f), kw, true);
    }
}

impl ColorFmt {
    fn symbol(self) -> &'static str {
        FORMATS.iter().find(|f| f.0 == self).map_or("", |f| f.2)
    }

    pub fn accepts(self, s: &str) -> bool {
        match self {
            ColorFmt::Hex => s.len() <= 9 && s.strip_prefix('#').unwrap_or(s).chars().all(|c| c.is_ascii_hexdigit()),
            ColorFmt::Name => s.len() <= 30 && s.chars().all(|c| c.is_ascii_alphabetic()),
            ColorFmt::Integer => s.len() <= 8 && s.chars().all(|c| c.is_ascii_digit()),
            ColorFmt::Ansi256 => s.len() <= 3 && s.chars().all(|c| c.is_ascii_digit()),
            _ => {
                let letters = format!("{}deg", self.symbol());
                s.len() <= 80 && s.chars().all(|c| "0123456789 ,.%-+()/".contains(c) || (c.is_ascii_alphabetic() && letters.contains(c.to_ascii_lowercase())))
            }
        }
    }

    pub fn placeholder(self) -> &'static str {
        match self {
            ColorFmt::Hex => "#ff5733",
            ColorFmt::Name => "tomato",
            _ => "",
        }
    }

    /// Ok(None) while the input is still incomplete.
    pub fn parse(self, s: &str) -> Result<Option<Rgb>, String> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(None);
        }
        let body = s.split_once('(').map_or(s, |(_, b)| b);
        let nums: Vec<f64> = body
            .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+'))
            .filter_map(|t| t.parse().ok())
            .collect();
        let need = |n: usize| nums.len() >= n;
        let range = |v: f64, lo: f64, hi: f64, what: &str| {
            if (lo..=hi).contains(&v) { Ok(v) } else { Err(format!("{what} must be {lo}–{hi}")) }
        };
        let rgb = match self {
            ColorFmt::Hex => {
                let h = s.trim_start_matches('#');
                let h = match h.len() {
                    3 => h.chars().flat_map(|c| [c, c]).collect(),
                    6 => h.to_string(),
                    8 => h[..6].to_string(), // alpha is ignored
                    _ => return Ok(None),
                };
                let v = u32::from_str_radix(&h, 16).map_err(|_| "Bad hex color")?;
                from8(v)
            }
            ColorFmt::Integer => from8(s.parse::<u32>().ok().filter(|v| *v <= 0xFFFFFF).ok_or("Color integers go up to 16777215")?),
            ColorFmt::Ansi256 => ansi_rgb(s.parse::<u32>().ok().filter(|v| *v <= 255).ok_or("ANSI colors are 0–255")? as usize),
            ColorFmt::Name => match CSS_NAMES.iter().find(|n| n.0.eq_ignore_ascii_case(s)) {
                Some(n) => from8(n.1),
                None => return if CSS_NAMES.iter().any(|n| n.0.starts_with(&s.to_lowercase())) { Ok(None) } else { Err("Unknown color name".into()) },
            },
            _ if !need(if self == ColorFmt::Cmyk { 4 } else { 3 }) => return Ok(None),
            ColorFmt::Rgb => {
                let c = |i| range(nums[i], 0.0, 255.0, "RGB values").map(|v| v / 255.0);
                [c(0)?, c(1)?, c(2)?]
            }
            ColorFmt::RgbFloat => {
                let c = |i| range(nums[i], 0.0, 1.0, "Values");
                [c(0)?, c(1)?, c(2)?]
            }
            ColorFmt::Hsl => {
                let (s, l) = (range(nums[1], 0.0, 100.0, "Saturation")?, range(nums[2], 0.0, 100.0, "Lightness")?);
                hsl_to_rgb(nums[0], s / 100.0, l / 100.0)
            }
            ColorFmt::Hsv => {
                let (s, v) = (range(nums[1], 0.0, 100.0, "Saturation")?, range(nums[2], 0.0, 100.0, "Value")?);
                hsv_to_rgb(nums[0], s / 100.0, v / 100.0)
            }
            ColorFmt::Hwb => {
                let (w, b) = (range(nums[1], 0.0, 100.0, "Whiteness")? / 100.0, range(nums[2], 0.0, 100.0, "Blackness")? / 100.0);
                if w + b >= 1.0 {
                    let g = w / (w + b);
                    [g, g, g]
                } else {
                    hsv_to_rgb(nums[0], 1.0, 1.0).map(|c| c * (1.0 - w - b) + w)
                }
            }
            ColorFmt::Cmyk => {
                let c = |i| range(nums[i], 0.0, 100.0, "CMYK values").map(|v| v / 100.0);
                let k = c(3)?;
                [(1.0 - c(0)?) * (1.0 - k), (1.0 - c(1)?) * (1.0 - k), (1.0 - c(2)?) * (1.0 - k)]
            }
            ColorFmt::Xyz => xyz_to_rgb([nums[0] / 100.0, nums[1] / 100.0, nums[2] / 100.0]),
            ColorFmt::Lab => lab_to_rgb(range(nums[0], 0.0, 100.0, "Lightness")?, nums[1], nums[2]),
            ColorFmt::Lch => {
                let h = nums[2].to_radians();
                lab_to_rgb(range(nums[0], 0.0, 100.0, "Lightness")?, nums[1] * h.cos(), nums[1] * h.sin())
            }
            ColorFmt::Oklab => oklab_to_rgb(range(nums[0], 0.0, 1.0, "Lightness")?, nums[1], nums[2]),
            ColorFmt::Oklch => {
                let h = nums[2].to_radians();
                oklab_to_rgb(range(nums[0], 0.0, 1.0, "Lightness")?, nums[1] * h.cos(), nums[1] * h.sin())
            }
        };
        Ok(Some(rgb.map(|c| c.clamp(0.0, 1.0))))
    }

    pub fn format(self, c: Rgb) -> String {
        let [r, g, b] = c;
        let (max, min) = (r.max(g).max(b), r.min(g).min(b));
        let d = max - min;
        let n = |v: f64, places: usize| {
            let s = format!("{v:.places$}");
            let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
            if s == "-0" { "0".into() } else { s }
        };
        match self {
            ColorFmt::Hex => format!("#{:06x}", to8(c)),
            ColorFmt::Rgb => {
                let [r, g, b] = c.map(|v| (v * 255.0).round());
                format!("rgb({r}, {g}, {b})")
            }
            ColorFmt::RgbFloat => format!("vec3({r:.3}, {g:.3}, {b:.3})"),
            ColorFmt::Hsl => {
                let l = (max + min) / 2.0;
                let s = if d == 0.0 { 0.0 } else { d / (1.0 - (2.0 * l - 1.0).abs()) };
                format!("hsl({}, {}%, {}%)", n(hue(c), 1), n(s * 100.0, 1), n(l * 100.0, 1))
            }
            ColorFmt::Hsv => {
                let s = if max == 0.0 { 0.0 } else { d / max };
                format!("hsv({}, {}%, {}%)", n(hue(c), 1), n(s * 100.0, 1), n(max * 100.0, 1))
            }
            ColorFmt::Hwb => format!("hwb({} {}% {}%)", n(hue(c), 1), n(min * 100.0, 1), n((1.0 - max) * 100.0, 1)),
            ColorFmt::Cmyk => {
                let k = 1.0 - max;
                let f = |v: f64| if k >= 1.0 { 0.0 } else { (1.0 - v - k) / (1.0 - k) * 100.0 };
                format!("cmyk({}%, {}%, {}%, {}%)", n(f(r), 1), n(f(g), 1), n(f(b), 1), n(k * 100.0, 1))
            }
            ColorFmt::Xyz => {
                let [x, y, z] = rgb_to_xyz(c).map(|v| v * 100.0);
                format!("xyz({}, {}, {})", n(x, 3), n(y, 3), n(z, 3))
            }
            ColorFmt::Lab => {
                let [l, a, b] = rgb_to_lab(c);
                format!("lab({} {} {})", n(l, 2), n(a, 2), n(b, 2))
            }
            ColorFmt::Lch => {
                let [l, a, b] = rgb_to_lab(c);
                format!("lch({} {} {})", n(l, 2), n(a.hypot(b), 2), n(polar_hue(a, b), 2))
            }
            ColorFmt::Oklab => {
                let [l, a, b] = rgb_to_oklab(c);
                format!("oklab({} {} {})", n(l, 4), n(a, 4), n(b, 4))
            }
            ColorFmt::Oklch => {
                let [l, a, b] = rgb_to_oklab(c);
                format!("oklch({} {} {})", n(l, 4), n(a.hypot(b), 4), n(polar_hue(a, b), 2))
            }
            ColorFmt::Integer => to8(c).to_string(),
            ColorFmt::Ansi256 => nearest_ansi(c).to_string(),
            ColorFmt::Name => nearest_name(c).0.to_string(),
        }
    }
}

pub fn to8(c: Rgb) -> u32 {
    let [r, g, b] = c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u32);
    r << 16 | g << 8 | b
}

fn from8(v: u32) -> Rgb {
    [(v >> 16 & 255) as f64 / 255.0, (v >> 8 & 255) as f64 / 255.0, (v & 255) as f64 / 255.0]
}

fn hue([r, g, b]: Rgb) -> f64 {
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    h.rem_euclid(360.0)
}

fn polar_hue(a: f64, b: f64) -> f64 {
    if a.hypot(b) < 1e-9 { 0.0 } else { b.atan2(a).to_degrees().rem_euclid(360.0) }
}

fn sector(h: f64, c: f64, x: f64) -> Rgb {
    match (h.rem_euclid(360.0) / 60.0) as u32 {
        0 => [c, x, 0.0],
        1 => [x, c, 0.0],
        2 => [0.0, c, x],
        3 => [0.0, x, c],
        4 => [x, 0.0, c],
        _ => [c, 0.0, x],
    }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h.rem_euclid(360.0) / 60.0) % 2.0 - 1.0).abs());
    sector(h, c, x).map(|v| v + l - c / 2.0)
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> Rgb {
    let c = v * s;
    let x = c * (1.0 - ((h.rem_euclid(360.0) / 60.0) % 2.0 - 1.0).abs());
    sector(h, c, x).map(|p| p + v - c)
}

fn to_linear(c: f64) -> f64 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn from_linear(c: f64) -> f64 {
    if c <= 0.0031308 { 12.92 * c } else { 1.055 * c.max(0.0).powf(1.0 / 2.4) - 0.055 }
}

fn rgb_to_xyz(c: Rgb) -> [f64; 3] {
    let [r, g, b] = c.map(to_linear);
    [
        0.4124564 * r + 0.3575761 * g + 0.1804375 * b,
        0.2126729 * r + 0.7151522 * g + 0.0721750 * b,
        0.0193339 * r + 0.1191920 * g + 0.9503041 * b,
    ]
}

fn xyz_to_rgb([x, y, z]: [f64; 3]) -> Rgb {
    [
        3.2404542 * x - 1.5371385 * y - 0.4985314 * z,
        -0.9692660 * x + 1.8760108 * y + 0.0415560 * z,
        0.0556434 * x - 0.2040259 * y + 1.0572252 * z,
    ]
    .map(from_linear)
}

const WHITE: [f64; 3] = [0.95047, 1.0, 1.08883];
const DELTA: f64 = 6.0 / 29.0;

fn rgb_to_lab(c: Rgb) -> [f64; 3] {
    let f = |t: f64| if t > DELTA.powi(3) { t.cbrt() } else { t / (3.0 * DELTA * DELTA) + 4.0 / 29.0 };
    let xyz = rgb_to_xyz(c);
    let [fx, fy, fz] = [f(xyz[0] / WHITE[0]), f(xyz[1] / WHITE[1]), f(xyz[2] / WHITE[2])];
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn lab_to_rgb(l: f64, a: f64, b: f64) -> Rgb {
    let finv = |t: f64| if t > DELTA { t.powi(3) } else { 3.0 * DELTA * DELTA * (t - 4.0 / 29.0) };
    let fy = (l + 16.0) / 116.0;
    xyz_to_rgb([WHITE[0] * finv(fy + a / 500.0), WHITE[1] * finv(fy), WHITE[2] * finv(fy - b / 200.0)])
}

fn rgb_to_oklab(c: Rgb) -> [f64; 3] {
    let [r, g, b] = c.map(to_linear);
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}

fn oklab_to_rgb(l: f64, a: f64, b: f64) -> Rgb {
    let l_ = (l + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m_ = (l - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s_ = (l - 0.0894841775 * a - 1.2914855480 * b).powi(3);
    [
        4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_,
        -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_,
        -0.0041960863 * l_ - 0.7034186147 * m_ + 1.7076147010 * s_,
    ]
    .map(from_linear)
}

fn ansi_rgb(i: usize) -> Rgb {
    const BASE: [u32; 16] = [
        0x000000, 0x800000, 0x008000, 0x808000, 0x000080, 0x800080, 0x008080, 0xc0c0c0,
        0x808080, 0xff0000, 0x00ff00, 0xffff00, 0x0000ff, 0xff00ff, 0x00ffff, 0xffffff,
    ];
    const LEVELS: [u32; 6] = [0, 95, 135, 175, 215, 255];
    match i {
        0..=15 => from8(BASE[i]),
        16..=231 => {
            let i = i - 16;
            from8(LEVELS[i / 36] << 16 | LEVELS[i / 6 % 6] << 8 | LEVELS[i % 6])
        }
        _ => {
            let g = 8 + 10 * (i as u32 - 232);
            from8(g << 16 | g << 8 | g)
        }
    }
}

/// Perceptual distance (OKLab).
fn distance(a: Rgb, b: Rgb) -> f64 {
    let (p, q) = (rgb_to_oklab(a), rgb_to_oklab(b));
    (p[0] - q[0]).hypot(p[1] - q[1]).hypot(p[2] - q[2])
}

/// Nearest of the 240 fixed xterm colors (16–255); 0–15 depend on the terminal theme.
fn nearest_ansi(c: Rgb) -> usize {
    (16..256).min_by(|&a, &b| distance(c, ansi_rgb(a)).total_cmp(&distance(c, ansi_rgb(b)))).unwrap()
}

/// Nearest CSS color name and whether it matches exactly.
pub fn nearest_name(c: Rgb) -> (&'static str, bool) {
    let n = CSS_NAMES.iter().min_by(|a, b| distance(c, from8(a.1)).total_cmp(&distance(c, from8(b.1)))).unwrap();
    (n.0, n.1 == to8(c))
}

const CSS_NAMES: &[(&str, u32)] = &[
    ("black", 0x000000), ("white", 0xffffff), ("red", 0xff0000), ("lime", 0x00ff00), ("blue", 0x0000ff),
    ("aliceblue", 0xf0f8ff), ("antiquewhite", 0xfaebd7), ("aqua", 0x00ffff), ("aquamarine", 0x7fffd4),
    ("azure", 0xf0ffff), ("beige", 0xf5f5dc), ("bisque", 0xffe4c4), ("blanchedalmond", 0xffebcd),
    ("blueviolet", 0x8a2be2), ("brown", 0xa52a2a), ("burlywood", 0xdeb887), ("cadetblue", 0x5f9ea0),
    ("chartreuse", 0x7fff00), ("chocolate", 0xd2691e), ("coral", 0xff7f50), ("cornflowerblue", 0x6495ed),
    ("cornsilk", 0xfff8dc), ("crimson", 0xdc143c), ("cyan", 0x00ffff), ("darkblue", 0x00008b),
    ("darkcyan", 0x008b8b), ("darkgoldenrod", 0xb8860b), ("darkgray", 0xa9a9a9), ("darkgreen", 0x006400),
    ("darkgrey", 0xa9a9a9), ("darkkhaki", 0xbdb76b), ("darkmagenta", 0x8b008b), ("darkolivegreen", 0x556b2f),
    ("darkorange", 0xff8c00), ("darkorchid", 0x9932cc), ("darkred", 0x8b0000), ("darksalmon", 0xe9967a),
    ("darkseagreen", 0x8fbc8f), ("darkslateblue", 0x483d8b), ("darkslategray", 0x2f4f4f), ("darkslategrey", 0x2f4f4f),
    ("darkturquoise", 0x00ced1), ("darkviolet", 0x9400d3), ("deeppink", 0xff1493), ("deepskyblue", 0x00bfff),
    ("dimgray", 0x696969), ("dimgrey", 0x696969), ("dodgerblue", 0x1e90ff), ("firebrick", 0xb22222),
    ("floralwhite", 0xfffaf0), ("forestgreen", 0x228b22), ("fuchsia", 0xff00ff), ("gainsboro", 0xdcdcdc),
    ("ghostwhite", 0xf8f8ff), ("gold", 0xffd700), ("goldenrod", 0xdaa520), ("gray", 0x808080),
    ("green", 0x008000), ("greenyellow", 0xadff2f), ("grey", 0x808080), ("honeydew", 0xf0fff0),
    ("hotpink", 0xff69b4), ("indianred", 0xcd5c5c), ("indigo", 0x4b0082), ("ivory", 0xfffff0),
    ("khaki", 0xf0e68c), ("lavender", 0xe6e6fa), ("lavenderblush", 0xfff0f5), ("lawngreen", 0x7cfc00),
    ("lemonchiffon", 0xfffacd), ("lightblue", 0xadd8e6), ("lightcoral", 0xf08080), ("lightcyan", 0xe0ffff),
    ("lightgoldenrodyellow", 0xfafad2), ("lightgray", 0xd3d3d3), ("lightgreen", 0x90ee90), ("lightgrey", 0xd3d3d3),
    ("lightpink", 0xffb6c1), ("lightsalmon", 0xffa07a), ("lightseagreen", 0x20b2aa), ("lightskyblue", 0x87cefa),
    ("lightslategray", 0x778899), ("lightslategrey", 0x778899), ("lightsteelblue", 0xb0c4de), ("lightyellow", 0xffffe0),
    ("limegreen", 0x32cd32), ("linen", 0xfaf0e6), ("magenta", 0xff00ff), ("maroon", 0x800000),
    ("mediumaquamarine", 0x66cdaa), ("mediumblue", 0x0000cd), ("mediumorchid", 0xba55d3), ("mediumpurple", 0x9370db),
    ("mediumseagreen", 0x3cb371), ("mediumslateblue", 0x7b68ee), ("mediumspringgreen", 0x00fa9a),
    ("mediumturquoise", 0x48d1cc), ("mediumvioletred", 0xc71585), ("midnightblue", 0x191970), ("mintcream", 0xf5fffa),
    ("mistyrose", 0xffe4e1), ("moccasin", 0xffe4b5), ("navajowhite", 0xffdead), ("navy", 0x000080),
    ("oldlace", 0xfdf5e6), ("olive", 0x808000), ("olivedrab", 0x6b8e23), ("orange", 0xffa500),
    ("orangered", 0xff4500), ("orchid", 0xda70d6), ("palegoldenrod", 0xeee8aa), ("palegreen", 0x98fb98),
    ("paleturquoise", 0xafeeee), ("palevioletred", 0xdb7093), ("papayawhip", 0xffefd5), ("peachpuff", 0xffdab9),
    ("peru", 0xcd853f), ("pink", 0xffc0cb), ("plum", 0xdda0dd), ("powderblue", 0xb0e0e6),
    ("purple", 0x800080), ("rebeccapurple", 0x663399), ("rosybrown", 0xbc8f8f), ("royalblue", 0x4169e1),
    ("saddlebrown", 0x8b4513), ("salmon", 0xfa8072), ("sandybrown", 0xf4a460), ("seagreen", 0x2e8b57),
    ("seashell", 0xfff5ee), ("sienna", 0xa0522d), ("silver", 0xc0c0c0), ("skyblue", 0x87ceeb),
    ("slateblue", 0x6a5acd), ("slategray", 0x708090), ("slategrey", 0x708090), ("snow", 0xfffafa),
    ("springgreen", 0x00ff7f), ("steelblue", 0x4682b4), ("tan", 0xd2b48c), ("teal", 0x008080),
    ("thistle", 0xd8bfd8), ("tomato", 0xff6347), ("turquoise", 0x40e0d0), ("violet", 0xee82ee),
    ("wheat", 0xf5deb3), ("whitesmoke", 0xf5f5f5), ("yellow", 0xffff00), ("yellowgreen", 0x9acd32),
];

#[cfg(test)]
mod tests {
    use super::ColorFmt::*;
    use super::*;

    fn conv(from: ColorFmt, to: ColorFmt, s: &str) -> String {
        to.format(from.parse(s).unwrap().unwrap())
    }

    #[test]
    fn conversions() {
        assert_eq!(conv(Hex, Rgb, "#ff5733"), "rgb(255, 87, 51)");
        assert_eq!(conv(Rgb, Hex, "255 87 51"), "#ff5733");
        assert_eq!(conv(Hex, Hsl, "#ff0000"), "hsl(0, 100%, 50%)");
        assert_eq!(conv(Hsl, Hex, "hsl(120, 100%, 25%)"), "#008000");
        assert_eq!(conv(Hex, Cmyk, "#ff0000"), "cmyk(0%, 100%, 100%, 0%)");
        assert_eq!(conv(Name, Hex, "Tomato"), "#ff6347");
        assert_eq!(conv(Hex, Name, "#ff6348"), "tomato");
        assert_eq!(conv(Hex, Lab, "#ffffff"), "lab(100 0 0)");
        assert_eq!(conv(Hex, Ansi256, "#ff0000"), "196");
        assert_eq!(conv(Hex, Integer, "#010203"), "66051");
        // Round trips through every format stay within one 8-bit step.
        for f in [Rgb, RgbFloat, Hsl, Hsv, Hwb, Cmyk, Xyz, Lab, Lch, Oklab, Oklch, Integer] {
            assert_eq!(conv(f, Hex, &conv(Hex, f, "#3a7bd5")), "#3a7bd5", "{f:?}");
        }
        assert_eq!(Hex.parse("#ff57"), Ok(None));
        assert!(Rgb.parse("300 0 0").is_err());
        assert!(Hex.accepts("#fF0") && !Hex.accepts("#xyz") && !Rgb.accepts("rgb(1,2,q)"));
    }
}
