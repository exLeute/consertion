//! Everything the app can convert lives in this catalog.
//!
//! Adding a new domain: create a module with `pub fn register(b: &mut Builder)`,
//! add categories/units there, and call it from `Catalog::new`. If the domain needs
//! a new kind of math, add a `Kind` variant and handle it in `crate::convert`.

pub mod calc;
pub mod chem;
pub mod color;
pub mod cooking;
pub mod coords;
pub mod currency;
pub mod custom;
mod hash;
pub mod lang;
pub mod music;
pub mod numbers;
pub mod parts;
pub mod physical;
pub mod scale;
pub mod sizes;
pub mod tech;
pub mod text;
pub mod time;
pub mod zones;

use crate::num::{self, R};
use color::ColorFmt;
use num_traits::{One, Zero};
use text::Format;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// base = v × factor
    Linear,
    /// base = (v + offset) × factor (temperatures)
    Affine,
    /// base = factor / v (fuel consumption)
    Reciprocal,
    /// Rate of the unit's symbol (ISO code) comes from online rates.
    Currency,
    Radix(u32),
    Roman,
    /// Output only: English number words.
    Words,
    Text(Format),
    Color(ColorFmt),
    /// A calculator (index into `calc::CALCS`), opened directly instead of picking a target.
    Calc(usize),
    /// Exact value shown as a fraction ("3/8").
    Fraction(numbers::Frac),
    /// Non-linear scale (dB, pH, AWG), computed in floating point.
    Scale(scale::Scale),
    /// A date; the category base is Unix seconds.
    Date(time::DateFmt),
    /// A length of time as "h:mm:ss"; base seconds.
    Duration(time::DurFmt),
    Chem(chem::ChemFmt),
    /// A language (index into `lang::LANGS`); converting translates online.
    Lang(usize),
    /// A file format (index into `files::FORMATS`, or `files::PICK` for "Browse for a file…").
    File(usize),
    /// Wall-clock time in a place (index into `zones::ZONES`); base Unix seconds.
    Zone(usize),
    /// A column of a size chart (`sizes::tables()`); base is the row number.
    Size(u8, u8),
    /// A note name ("A4"); base hertz.
    Note,
    /// An interval name ("perfect fifth"); base cents.
    Interval,
    Coord(coords::CoordFmt),
    /// A marking on a part ("104", "4R7", color bands); base farad or ohm.
    Code(parts::CodeFmt),
    /// "New custom converter…": opens the form instead of converting.
    NewCustom,
}

impl Kind {
    /// Typed as a number or arithmetic expression.
    pub fn is_numeric(&self) -> bool {
        matches!(self, Kind::Linear | Kind::Affine | Kind::Reciprocal | Kind::Currency | Kind::Scale(_) | Kind::Fraction(_))
    }

    /// Has a value relative to its category's base unit (numbers, dates, durations).
    pub fn is_quantity(&self) -> bool {
        self.is_numeric() || matches!(self, Kind::Date(_) | Kind::Duration(_) | Kind::Zone(_) | Kind::Size(..) | Kind::Note | Kind::Interval | Kind::Code(_))
    }

    /// Shown with digit grouping ("1,234.5").
    pub fn groupable(&self) -> bool {
        self.is_numeric() && !matches!(self, Kind::Fraction(_))
    }

    /// Can this unit be the source of a conversion (can we parse it)?
    pub fn can_parse(&self) -> bool {
        match self {
            Kind::Words => false,
            Kind::Text(f) => f.can_decode(),
            Kind::Date(f) => f.can_parse(),
            Kind::Duration(f) => *f == time::DurFmt::Clock,
            Kind::Chem(f) => *f == chem::ChemFmt::Formula,
            _ => true,
        }
    }

    /// Can this unit be picked as the target ("Detect language" can't)?
    pub fn can_target(&self) -> bool {
        !matches!(self, Kind::Lang(0) | Kind::File(crate::files::PICK) | Kind::NewCustom)
    }

    /// Input filter: is `s` an acceptable (possibly partial) value for this unit?
    pub fn accepts(&self, s: &str) -> bool {
        match self {
            k if k.is_numeric() => num::accepts_number_input(s),
            Kind::Radix(r) => numbers::accepts_radix(s, *r),
            Kind::Roman => s.len() <= 20 && s.chars().all(|c| "IVXLCDMivxlcdm".contains(c)),
            Kind::Text(f) => f.accepts(s),
            Kind::Color(f) => f.accepts(s),
            Kind::Date(f) => f.accepts(s),
            Kind::Duration(f) => f.accepts(s),
            Kind::Chem(_) => chem::accepts(s),
            Kind::Lang(_) => s.chars().count() <= crate::translate::MAX_CHARS,
            Kind::Zone(_) => zones::accepts(s),
            Kind::Size(..) => s.chars().count() <= 40,
            Kind::Note => music::accepts_note(s),
            Kind::Interval => music::accepts_interval(s),
            Kind::Coord(_) => coords::accepts(s),
            Kind::Code(_) => parts::accepts(s),
            _ => false,
        }
    }

    pub fn placeholder(&self) -> &'static str {
        match self {
            Kind::Text(f) => f.placeholder(),
            Kind::Color(f) => f.placeholder(),
            Kind::Date(f) => f.placeholder(),
            Kind::Duration(_) => "h:mm:ss, e.g. 1:30:00 or 2d 4:00",
            Kind::Fraction(numbers::Frac::Mixed) => "e.g. 2 3/4",
            Kind::Fraction(_) => "e.g. 3/8",
            Kind::Chem(_) => "e.g. C6H12O6, Ca(OH)2, CuSO4·5H2O",
            Kind::Lang(_) => "Type or paste text to translate",
            Kind::Zone(_) => "e.g. 15:00, 3pm, now or 2026-11-07 09:30",
            Kind::Size(..) => "a size from the chart",
            Kind::Note => "e.g. A4, C#5, Bb3 or A4 +20¢",
            Kind::Interval => "e.g. P5, minor third, tritone or octave",
            Kind::Coord(f) => f.placeholder(),
            Kind::Code(f) => f.placeholder(),
            _ if !self.can_parse() => "output only",
            _ => "",
        }
    }
}

pub struct Unit {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub kind: Kind,
    pub category: usize,
    /// Exact factor to the category's base unit (Linear/Affine/Reciprocal).
    pub factor: R,
    /// Exact offset (Affine).
    pub offset: R,
    /// Common units are listed before rare ones (e.g. "km" before "ronnameter").
    pub common: bool,
    /// Units with different non-zero tags don't convert to each other (cup of flour vs sugar).
    pub tag: u16,
    name_l: String,
    symbol_l: String,
    haystack: String,
}

pub struct Category {
    pub id: &'static str,
    pub name: &'static str,
    /// Needs internet (rates) to convert.
    pub online: bool,
    pub units: Vec<usize>,
    /// Sidebar section ("Everyday", "Physics"…) and position in display order.
    pub group: &'static str,
    pub rank: usize,
}

/// Sidebar sections, everyday things first; categories within each in display order.
const GROUPS: &[(&str, &[&str])] = &[
    // No heading: sits right under All / Favorites / Recent.
    ("", &["custom"]),
    ("Everyday", &[
        "length", "mass", "temperature", "volume", "currency", "timezone", "area", "time", "datetime", "speed",
        "cooking", "ratio", "percent", "everyday", "homecalc", "fuel", "coords", "typing",
    ]),
    ("Business & Finance", &["bizcalc"]),
    ("Sizes", &["shoe", "ring", "womens", "menssuit", "mensshirt", "bra", "paper", "areal", "lineardensity"]),
    ("Health", &["healthcalc", "glucose", "hba1c"]),
    ("Files, Text & Data", &["files", "translate", "data", "datarate", "text", "numbers", "color", "css", "perms"]),
    ("Programming & Servers", &["devcalc", "eventrate"]),
    ("Gaming & Displays", &["gamecalc", "framerate", "mouse"]),
    ("Music & Audio", &["pitch", "tempo", "interval", "frames", "musiccalc"]),
    ("Photography", &["photocalc", "stops"]),
    ("Math", &["mathcalc", "angle", "solidangle"]),
    ("Physics", &[
        "physcalc", "energy", "power", "pressure", "force", "acceleration", "frequency", "torque", "density", "flow",
        "momentum", "massflow", "stiffness", "inertia", "tempdiff", "specheat", "thermcond", "spectrum", "specenergy",
        "viscosity", "kinvisc", "tension",
    ]),
    ("Electricity & Magnetism", &[
        "eleccalc", "voltage", "current", "resistance", "charge", "capacitance", "inductance", "conductance",
        "magfield", "magflux", "resistivity", "conductivity", "efield", "magh",
    ]),
    ("Light & Radiation", &[
        "illuminance", "luminance", "colortemp", "irradiance", "lumflux", "luminous", "doserate", "radioactivity", "absorbed", "equivalent",
        "exposure",
    ]),
    ("Chemistry", &["chemistry", "chemcalc", "molarity", "substance", "catalytic"]),
];

#[derive(Default)]
pub struct Catalog {
    pub categories: Vec<Category>,
    pub units: Vec<Unit>,
    /// Category indices in display order (by group, everyday first).
    pub order: Vec<usize>,
    /// Your own units that couldn't be built (they refer to a unit that's gone).
    pub broken: Vec<String>,
}

impl Catalog {
    #[cfg(test)]
    pub fn new() -> Self {
        Self::with_custom(&[])
    }

    /// The built-in catalog plus your own units.
    pub fn with_custom(mine: &[custom::CustomUnit]) -> Self {
        let mut b = Builder::default();
        physical::register(&mut b);
        currency::register(&mut b);
        zones::register(&mut b);
        cooking::register(&mut b);
        sizes::register(&mut b);
        coords::register(&mut b);
        music::register(&mut b);
        tech::register(&mut b);
        lang::register(&mut b);
        numbers::register(&mut b);
        color::register(&mut b);
        chem::register(&mut b);
        text::register(&mut b);
        calc::register(&mut b);
        crate::files::register(&mut b);
        custom::register(&mut b);
        custom::add_all(&mut b, mine);
        let mut c = b.cat;
        let ids = GROUPS.iter().flat_map(|&(g, ids)| ids.iter().map(move |id| (g, *id)));
        let known = ids.filter_map(|(g, id)| Some((g, c.categories.iter().position(|k| k.id == id)?)));
        // Anything missing from GROUPS still shows up, at the end.
        let rest: Vec<_> = (0..c.categories.len()).map(|i| ("Other", i)).collect();
        for (g, i) in known.collect::<Vec<_>>().into_iter().chain(rest) {
            if !c.order.contains(&i) {
                c.categories[i].group = g;
                c.categories[i].rank = c.order.len();
                c.order.push(i);
            }
        }
        c
    }

    /// Can `a` be converted to `b`, as far as ingredient tags go?
    pub fn pairs(&self, a: usize, b: usize) -> bool {
        let (x, y) = (self.units[a].tag, self.units[b].tag);
        x == 0 || y == 0 || x == y
    }

    pub fn find(&self, id: &str) -> Option<usize> {
        self.units.iter().position(|u| u.id == id)
    }

    /// Ranked search. Empty query lists everything (common units first).
    pub fn search(&self, query: &str, category: Option<usize>, keep: impl Fn(&Unit) -> bool) -> Vec<usize> {
        let raw = query.trim();
        let terms: Vec<String> = raw.to_lowercase().split_whitespace().map(String::from).collect();
        let pool: Box<dyn Iterator<Item = usize>> = match category {
            Some(c) => Box::new(self.categories[c].units.iter().copied()),
            None => Box::new(0..self.units.len()),
        };
        let mut hits: Vec<(u32, usize)> = pool
            .filter(|&i| keep(&self.units[i]))
            .filter_map(|i| score(&self.units[i], &terms, raw).map(|s| (s, i)))
            .collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        hits.into_iter().map(|(_, i)| i).collect()
    }

    /// Best match for a free-text unit name, e.g. for the command line ("km", "usd", "hex").
    pub fn lookup(&self, query: &str, category: Option<usize>) -> Option<usize> {
        self.search(query, category, |_| true).first().copied()
    }

    /// A unit in `category` whose symbol or name is exactly `query` (ignoring case).
    pub fn exact(&self, query: &str, category: usize) -> Option<usize> {
        let q = query.trim().to_lowercase();
        let units = &self.categories[category].units;
        units.iter().copied().find(|&u| self.units[u].symbol == query.trim()).or_else(|| {
            // "c" / "f" also match "°C" / "°F".
            units.iter().copied().find(|&u| {
                let u = &self.units[u];
                u.symbol_l == q || u.name_l == q || u.symbol_l.strip_prefix('°') == Some(q.as_str())
            })
        })
    }
}

fn score(u: &Unit, terms: &[String], raw: &str) -> Option<u32> {
    let mut total = if u.common { 100 } else { 0 };
    if terms.is_empty() {
        return Some(total);
    }
    if u.symbol == raw {
        total += 2000; // case-sensitive symbol hit: "M" (mega) vs "m" (milli)
    }
    for t in terms {
        total += if u.name_l == *t || u.symbol_l == *t {
            1000
        } else if u.name_l.split(|c: char| !c.is_alphanumeric()).any(|w| w == t) {
            700 // a whole word of the name: "text" → "Plain text"
        } else if u.name_l.starts_with(t.as_str()) {
            600
        } else if u.haystack.split(|c: char| !c.is_alphanumeric()).any(|w| w.starts_with(t.as_str())) {
            400
        } else if u.haystack.contains(t.as_str()) {
            200
        } else if t.chars().count() >= 4 && is_subsequence(&u.name_l, t) {
            50
        } else {
            return None;
        };
    }
    // Prefer shorter names among equal matches ("meter" before "megameter").
    Some(total + 60u32.saturating_sub(u.name_l.chars().count() as u32))
}

fn is_subsequence(hay: &str, needle: &str) -> bool {
    let mut it = hay.chars();
    needle.chars().all(|c| it.any(|h| h == c))
}

/// SI prefixes: (name, symbol, power of ten).
pub const SI_PREFIXES: [(&str, &str, i64); 24] = [
    ("quetta", "Q", 30), ("ronna", "R", 27), ("yotta", "Y", 24), ("zetta", "Z", 21),
    ("exa", "E", 18), ("peta", "P", 15), ("tera", "T", 12), ("giga", "G", 9),
    ("mega", "M", 6), ("kilo", "k", 3), ("hecto", "h", 2), ("deca", "da", 1),
    ("deci", "d", -1), ("centi", "c", -2), ("milli", "m", -3), ("micro", "µ", -6),
    ("nano", "n", -9), ("pico", "p", -12), ("femto", "f", -15), ("atto", "a", -18),
    ("zepto", "z", -21), ("yocto", "y", -24), ("ronto", "r", -27), ("quecto", "q", -30),
];

/// Named constants usable in catalog factor expressions. All exact by definition except π.
fn constant(name: &str) -> Option<R> {
    let e = match name {
        "pi" => return Some(num::pi()),
        "inch" => "0.0254",
        "ft" => "0.3048",
        "yd" => "0.9144",
        "mi" => "1609.344",
        "nmi" => "1852",
        "lb" => "0.45359237",
        "g0" => "9.80665",
        "lbf" => "lb*g0",
        "gal" => "231*inch^3",
        "galuk" => "0.00454609",
        "c" => "299792458",
        "day" => "86400",
        "yr" => "365.2425*day",
        "au" => "149597870700",
        "btu" => "1055.05585262",
        "ev" => "1.602176634e-19",
        "na" => "6.02214076e23",
        "mmhg" => "133.322387415",
        "planck" => "6.62607015e-34",
        _ => return None,
    };
    Some(expr(e))
}

/// Evaluate a catalog factor expression. Panics on typos — the catalog test catches them.
pub fn expr(s: &str) -> R {
    num::eval(s, &constant).unwrap_or_else(|e| panic!("bad catalog expression {s:?}: {e:?}"))
}

/// A table row for `Builder::linear`: (name, symbol, factor-to-base expression, keywords).
pub type Row = (&'static str, &'static str, &'static str, &'static str);

#[derive(Default)]
pub struct Builder {
    cat: Catalog,
    current: usize,
    /// Tag given to the units added next (see `Unit::tag`).
    pub tag: u16,
}

impl Builder {
    pub fn category(&mut self, id: &'static str, name: &'static str, online: bool) {
        self.cat.categories.push(Category { id, name, online, units: Vec::new(), group: "", rank: 0 });
        self.current = self.cat.categories.len() - 1;
        self.tag = 0;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn unit(&mut self, name: &str, symbol: &str, kind: Kind, factor: R, offset: R, keywords: &str, common: bool) {
        let cat = &mut self.cat.categories[self.current];
        let slug: String = name.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
        let haystack = format!("{} {} {} {}", name, symbol, keywords, cat.name).to_lowercase();
        cat.units.push(self.cat.units.len());
        self.cat.units.push(Unit {
            id: format!("{}:{}", cat.id, slug),
            name: name.to_string(),
            symbol: symbol.to_string(),
            kind,
            category: self.current,
            factor,
            offset,
            common,
            tag: self.tag,
            name_l: name.to_lowercase(),
            symbol_l: symbol.to_lowercase(),
            haystack,
        });
    }

    /// A unit without a numeric factor (currencies, bases, text, colors…).
    pub fn add(&mut self, name: &str, symbol: &str, kind: Kind, keywords: &str, common: bool) {
        self.unit(name, symbol, kind, R::one(), R::zero(), keywords, common);
    }

    /// Linear units from a table of (name, symbol, factor-to-base expression, keywords).
    pub fn linear(&mut self, table: &[Row]) {
        for &(name, sym, factor, kw) in table {
            self.unit(name, sym, Kind::Linear, expr(factor), R::zero(), kw, true);
        }
    }

    pub fn affine(&mut self, name: &str, sym: &str, factor: &str, offset: &str, kw: &str) {
        self.unit(name, sym, Kind::Affine, expr(factor), expr(offset), kw, true);
    }

    /// A non-linear (logarithmic…) unit; see `scale::Scale`.
    pub fn scale(&mut self, name: &str, sym: &str, s: scale::Scale, kw: &str) {
        self.add(name, sym, Kind::Scale(s), kw, true);
    }

    pub fn reciprocal(&mut self, name: &str, sym: &str, factor: &str, kw: &str) {
        self.unit(name, sym, Kind::Reciprocal, expr(factor), R::zero(), kw, true);
    }

    /// A unit plus its SI-prefixed variants. `common` lists the prefix symbols to rank first.
    pub fn si(&mut self, name: &str, sym: &str, factor: &str, positive_only: bool, common: &[&str], kw: &str) {
        let f = expr(factor);
        self.unit(name, sym, Kind::Linear, f.clone(), R::zero(), kw, true);
        for &(pn, ps, exp) in &SI_PREFIXES {
            if positive_only && exp < 0 {
                continue;
            }
            let micro_kw = if ps == "µ" { format!("u{sym} mc{sym}") } else { String::new() };
            self.unit(
                &format!("{pn}{name}"),
                &format!("{ps}{sym}"),
                Kind::Linear,
                &f * num::pow10(exp),
                R::zero(),
                &format!("{kw} {micro_kw}"),
                common.contains(&ps),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_category_has_a_group() {
        let c = Catalog::new();
        assert_eq!(c.order.len(), c.categories.len());
        let lost: Vec<_> = c.categories.iter().filter(|k| k.group == "Other").map(|k| k.id).collect();
        assert!(lost.is_empty(), "add to GROUPS: {lost:?}");
        let ids: Vec<_> = GROUPS.iter().flat_map(|g| g.1.iter()).collect();
        assert!(ids.iter().all(|id| c.categories.iter().any(|k| k.id == **id)), "GROUPS names a missing category");
    }

    #[test]
    fn catalog_is_big_and_ids_unique() {
        let c = Catalog::new();
        assert!(c.units.len() >= 1000, "only {} units", c.units.len());
        let mut ids: Vec<&str> = c.units.iter().map(|u| u.id.as_str()).collect();
        ids.sort();
        let dups: Vec<&&str> = ids.windows(2).filter(|w| w[0] == w[1]).map(|w| &w[0]).collect();
        assert!(dups.is_empty(), "duplicate unit ids: {dups:?}");
    }

    #[test]
    fn search_ranks_obvious_hits_first() {
        let c = Catalog::new();
        let top = |q: &str| c.units[c.search(q, None, |_| true)[0]].name.clone();
        assert_eq!(top("meter"), "meter");
        assert_eq!(top("cm"), "centimeter");
        assert_eq!(top("usd"), "US Dollar");
        assert_eq!(top("UAH"), "Ukrainian Hryvnia");
        assert_eq!(top("MB"), "megabyte");
        assert_eq!(top("hex"), "Hexadecimal (base 16)");
        assert_eq!(top("text"), "Plain text");
        assert!(c.search("zzzzqqq", None, |_| true).is_empty());
    }
}
